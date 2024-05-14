use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_std::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LineItemPair<T>(
    pub String, // line
    pub T,      // item generated from line
);

pub trait IDed {
    fn id(&self) -> &str;
}

pub type CacheMap<T> = HashMap<String, Arc<Mutex<T>>>;

#[derive(Debug, Clone)]
pub struct FileCache<T: Serialize + for<'de> Deserialize<'de> + IDed> {
    map: CacheMap<T>,
    lock: Arc<Mutex<()>>,
    pub filepath: PathBuf,
}

pub fn to_hash_map<T: IDed>(items: impl Iterator<Item = T>) -> HashMap<String, T> {
    items.map(|s| (s.id().to_string(), s)).collect()
}

impl<T: Serialize + for<'de> Deserialize<'de> + IDed> FileCache<T> {
    pub fn new(filepath: PathBuf) -> Self {
        Self {
            filepath,
            map: Default::default(),
            lock: Default::default(),
        }
    }

    /// Filters out the items that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    pub fn find_unused_itmes(&self) -> impl Iterator<Item = String> + '_ {
        self.map.iter().filter_map(|(key, s)| {
            let count = Arc::strong_count(s);
            match count == 1 {
                true => Some(key.clone()),
                false => None,
            }
        })
    }

    pub fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }

    pub fn fetch(&mut self, ids: &HashSet<String>) -> CacheMap<T> {
        let cs_keys: HashSet<String> = self.map.keys().cloned().collect();

        match cs_keys.is_empty() {
            true => self.cache_from_ndjson(ids).collect(),
            false => {
                let already_cached = ids.intersection(&cs_keys);
                let not_cached: HashSet<_> = ids.difference(&cs_keys).cloned().collect();

                // chain caches from ndjson if not_cached is not empty
                let get_cache = (!not_cached.is_empty())
                    .then(|| {
                        self.cache_from_ndjson(&not_cached)
                            .collect::<Vec<(String, Arc<Mutex<T>>)>>()
                    })
                    .into_iter()
                    .flatten();
                already_cached
                    .into_iter()
                    .map(|k| (k.clone(), self.new_arc(k)))
                    .chain(get_cache)
                    .collect()
            }
        }
    }

    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a tuple of the original line and the created item
    pub fn read_items_from_ndjson(
        pth: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        println!["Reading from ndjson"];
        let pth = pth.as_ref();

        std::fs::File::open(pth).map(|file| {
            std::io::BufReader::new(file)
                .lines()
                // Filter to lines that successfully read
                .map_while(Result::ok)
                // Filter to lines that are valid <T>s
                .filter_map(|l| {
                    serde_json::from_str::<T>(&l)
                        .ok()
                        .map(|s| LineItemPair(l, s))
                })
        })
    }

    fn cache_from_ndjson(
        &mut self,
        ids: &HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let _lock = self.lock.lock().unwrap();
                    Self::read_items_from_ndjson(&self.filepath)
                        .unwrap()
                        .filter_map(move |LineItemPair(_, item)| {
                            let id = item.id().to_string();

                            let contains = ids.contains(&id);
                            contains.then(|| (id, Arc::new(Mutex::new(item))))
                        })
                        .collect()
                };
                self.map.extend(items.clone());
                items.into_iter().map(|(str, _)| {
                    let arc = self.new_arc(&str);
                    (str.to_string(), arc)
                })
            })
            .into_iter()
            .flatten()
    }

    fn new_arc(&self, id: &str) -> Arc<Mutex<T>> {
        Arc::clone(&self.map[id])
    }

    pub fn cache_size(&self) -> usize {
        self.map.len()
    }

    /// Extends the cache with new items. This is different from extend() in that this uses the map's filelock and precaches the new items.
    #[allow(unused)]
    pub async fn extend_file(
        &mut self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let mut items: HashMap<String, T> = to_hash_map(items);

        let file_path = &self.filepath;
        let tempfile = file_path.with_extension("ndjson.tmp");
        {
            // Create the temporary file to write the new list to
            let output_file = async_std::fs::File::create(&tempfile).await?;
            let mut out = async_std::io::BufWriter::new(output_file);

            // Read the original list
            {
                let valid_items: Vec<Vec<u8>> = {
                    let _lock = self.lock.lock().unwrap();
                    if let Ok(itemlist) = FileCache::read_items_from_ndjson(file_path) {
                        // Filter through the lines, find existing keys and skip broken lines
                        Self::filter_file_itmes(itemlist, overwrite, &mut items).collect()
                    } else {
                        vec![vec![]]
                    }
                };
                for bytes in valid_items {
                    out.write(&bytes).await?;
                }

                out.flush().await?;
            }

            // Add remaining keys to the file
            for (id, item) in items {
                let mut json = serde_json::to_string(&item).unwrap();
                json.push('\n');
                out.write(json.as_bytes()).await?;

                // Add item to cache
                self.map.insert(id, Arc::new(Mutex::new(item)));
            }
            out.flush().await?;
        }
        // Replace songs.ndjson with songs.ndjson.tmp
        {
            let _lock = self.lock.lock().unwrap();
            std::fs::rename(&tempfile, file_path)?;
        }

        Ok(())
    }

    pub async fn extend(
        pth: impl AsRef<Path>,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let mut items = to_hash_map(items);

        let file_path = pth.as_ref();
        let tempfile = file_path.with_extension("ndjson.tmp");

        {
            // Create the temporary file to write the new list to
            let output_file = async_std::fs::File::create(&tempfile).await?;
            let mut out = async_std::io::BufWriter::new(output_file);
            {
                // Read the original list
                if let Ok(itemlist) = FileCache::read_items_from_ndjson(file_path) {
                    // Filter through the lines, find existing keys and skip broken lines
                    let valid_items = Self::filter_file_itmes(itemlist, overwrite, &mut items);

                    for bytes in valid_items {
                        out.write(&bytes).await?;
                    }

                    out.flush().await?;
                }
            }

            // Add remaining keys to the file
            for (_id, item) in items {
                let mut json = serde_json::to_string(&item).unwrap();
                json.push('\n');
                out.write(json.as_bytes()).await?;
            }
            out.flush().await?;
        }
        // Replace songs.ndjson with songs.ndjson.tmp
        async_std::fs::rename(&tempfile, &file_path).await?;

        Ok(())
    }

    fn filter_file_itmes<'a>(
        items: impl Iterator<Item = LineItemPair<T>> + 'a,
        overwrite: bool,
        filter: &'a mut HashMap<String, T>,
    ) -> impl Iterator<Item = Vec<u8>> + 'a {
        items.filter_map(move |LineItemPair(mut line, item)| {
            line.push('\n');
            let id = item.id();
            match (overwrite, filter.contains_key(id)) {
                (true, true) => None,
                (true, false) => Some(line.as_bytes().to_vec()),
                (false, true) => {
                    filter.remove(id);
                    Some(line.as_bytes().to_vec())
                }
                (false, false) => Some(line.as_bytes().to_vec()),
            }
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct CacheInterface<T: ?Sized> {
    cache: CacheMap<T>,
    keys: HashSet<String>,
}

impl<T: IDed + ?Sized> CacheInterface<T> {
    pub fn get<'a>(
        &'a self,
        ids: &'a HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_ {
        let existing = ids.intersection(&self.keys).cloned();
        existing.map(|k| (k.clone(), Arc::clone(&self.cache[&k])))
    }

    pub fn extend(&mut self, items: CacheMap<T>) {
        let new_keys: HashSet<String> = items.keys().cloned().collect();
        self.keys = self.keys.union(&new_keys).cloned().collect();
        self.cache.extend(items)
    }

    pub fn replace(&mut self, cache: CacheMap<T>) {
        self.keys = cache.keys().cloned().collect();
        self.cache = cache;
    }

    /// Returns the number of elements in the cache1

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn get_keys(&self) -> &HashSet<String> {
        &self.keys
    }
}

#[cfg(test)]
mod tests {
    use crate::{settings::SongKey, song::Song};
    use once_cell::sync::Lazy;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::{collections::HashSet, sync::Arc};
    use tempfile::TempDir;

    static TESTING_LOCK: Lazy<Arc<Mutex<()>>> = Lazy::new(|| Arc::new(Mutex::new(())));

    static TEST_FOLDER: Lazy<TempDir> = Lazy::new(|| tempfile::tempdir().unwrap());

    fn random_file() -> PathBuf {
        TEST_FOLDER
            .path()
            .join(format!("{:x}", rand::random::<u64>()))
    }

    use super::FileCache;

    #[test]
    fn fetching() {
        let songs = vec![Song::basic(), Song::basic(), Song::basic()];
        let missing_songs = vec![Song::basic(), Song::basic(), Song::basic()];
        let real_keys: HashSet<SongKey> = songs.iter().map(|s| s.id.clone()).collect();
        let keys: HashSet<SongKey> = real_keys
            .iter()
            .cloned()
            .chain(missing_songs.clone().iter().map(|s| s.id.clone()))
            .collect();

        let tmpfile = random_file();
        let mut sc = FileCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            map: Default::default(),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let r = runtime.block_on(sc.extend_file(songs.clone().into_iter(), false));
        println!["{:?}", r];
        assert![r.is_ok()];

        // Sending the same songs to check the overwriting mechanism
        let r = runtime.block_on(sc.extend_file(songs.into_iter(), false));
        println!["{:?}", r];
        assert![r.is_ok()];
        let songs = sc.fetch(&keys);
        println!["{:?}", sc];
        println!["{:?}", songs];
        assert_eq![songs.len(), 3];
    }

    #[test]
    fn checking_unused() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone();
        let keys: HashSet<SongKey> = songs.clone().iter().map(|s| s.id.clone()).collect();

        let tmpfile = random_file();
        let mut sc = FileCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            map: Default::default(),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let extension_result = { runtime.block_on(sc.extend_file(songs.into_iter(), false)) };
        println!["{extension_result:?}"];
        assert![extension_result.is_ok()];

        sc.fetch(&keys);
        let mut unused: Vec<_> = sc.find_unused_itmes().collect();
        println!["{sc:?}"];
        println!["Unused songs before: {:?}", unused];
        assert_eq![unused.len(), 2];
        {
            let guards = sc.fetch(&[first_key.clone()].into_iter().collect());
            assert![guards.len() > 0];
            println!["Captured guards: {:?}", guards];
            let song = guards[&first_key].lock();
            println!["Captured song: {:?}", song];

            unused = sc.find_unused_itmes().collect();
            println!["Unused songs during: {:?}", unused];
            assert_eq![unused.len(), 1];
        }
        unused = sc.find_unused_itmes().collect();
        println!["Unused songs after: {:?}", unused];
        assert_eq![unused.len(), 2];
    }

    #[test]
    fn dropping() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone(); // the drop target

        let tmpfile = random_file();
        let mut sc = FileCache {
            lock: TESTING_LOCK.clone(),
            filepath: tmpfile,
            map: Default::default(),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        {
            let r = runtime.block_on(sc.extend_file(songs.into_iter(), false));
            println!["{r:?}"];
            assert![r.is_ok()];
        }
        let mut unused: Vec<_> = sc.find_unused_itmes().collect();
        assert_eq![unused.len(), 2];
        sc.drop_from_cache([first_key]);
        unused = sc.find_unused_itmes().collect();
        assert_eq![unused.len(), 1];

        sc.drop_from_cache(unused);
        unused = sc.find_unused_itmes().collect();
        assert_eq![unused.len(), 0];
    }
}
