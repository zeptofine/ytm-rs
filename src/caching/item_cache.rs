use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    io::{BufRead, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

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

macro_rules! to_hash_map {
    ($items:expr) => {{
        $items.map(|item| (item.id().to_string(), item)).collect()
    }};
}

fn filter_file_items<'a, T: IDed>(
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

pub trait CacheReader {
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read<T: IDed + for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error>;

    fn read_filter<T: IDed + for<'de> Deserialize<'de>>(
        &self,
        ids: HashSet<String>,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        Ok(self
            .read::<T>()?
            .filter(move |item| ids.contains(item.1.id())))
    }

    fn extend<T: IDed + Serialize + for<'de> Deserialize<'de>>(
        self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error>;
}

#[derive(Debug, Clone)]
pub struct LineBasedCache {
    pub filepath: PathBuf,
}
impl LineBasedCache {
    pub fn new(filepath: PathBuf) -> Self {
        Self { filepath }
    }
}

impl CacheReader for LineBasedCache {
    fn read<T: IDed + for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        std::fs::File::open(&self.filepath).map(|file| {
            std::io::BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .filter_map(|l| {
                    serde_json::from_str::<T>(&l)
                        .ok()
                        .map(|s| LineItemPair(l, s))
                })
        })
    }

    fn extend<T: IDed + Serialize + for<'de> Deserialize<'de>>(
        self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let mut items: HashMap<String, T> = to_hash_map!(items);

        let filepath = &self.filepath;
        let tempfile = filepath.with_extension("ndjson.tmp");

        {
            // Create the temporary file to write the new list to
            let output_file = std::fs::File::create(&tempfile)?;
            let mut out = std::io::BufWriter::new(output_file);
            {
                // Read the original list
                if let Ok(itemlist) = self.read() {
                    // Filter through the lines, find existing keys and skip broken lines
                    let valid_items = filter_file_items(itemlist, overwrite, &mut items);

                    for bytes in valid_items {
                        out.write_all(&bytes)?;
                    }

                    out.flush()?;
                }
            }

            // Add remaining keys to the file
            for (_id, item) in items {
                let mut json = serde_json::to_string(&item).unwrap();
                json.push('\n');
                out.write_all(json.as_bytes())?;
            }
            out.flush()?;
        }

        // Replace songs.ndjson with songs.ndjson.tmp
        std::fs::rename(&tempfile, filepath)?;

        Ok(())
    }
}

// I would put this as an impl in FileCache but im not smart enough to make that happen
#[derive(Debug, Clone)]
pub enum CacheType {
    Line(LineBasedCache),
}
impl CacheReader for CacheType {
    fn read<T: IDed + for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        match self {
            CacheType::Line(lc) => lc.read(),
        }
    }

    fn extend<T: IDed + Serialize + for<'de> Deserialize<'de>>(
        self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        match self {
            CacheType::Line(lc) => lc.extend(items, overwrite),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileCache<T: Serialize + for<'de> Deserialize<'de> + IDed> {
    map: CacheMap<T>,
    pub reader: Arc<Mutex<CacheType>>,
}

impl<T: Serialize + for<'de> Deserialize<'de> + IDed> FileCache<T> {
    pub fn new(cache: CacheType) -> Self {
        Self {
            reader: Arc::new(Mutex::new(cache)),
            map: Default::default(),
        }
    }

    /// Filters out the items that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    pub fn find_unused_items(&self) -> impl Iterator<Item = String> + '_ {
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

    pub async fn extend(
        reader: Arc<Mutex<CacheType>>,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let reader = reader.lock().unwrap();
        reader.clone().extend(items, overwrite)?;
        Ok(())
    }

    fn cache_from_ndjson(
        &mut self,
        ids: &HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let reader = self.reader.lock().unwrap();
                    let items = match reader.read() {
                        Ok(iter) => iter
                            .filter_map(move |LineItemPair(_, item): LineItemPair<T>| {
                                let id = item.id().to_string();
                                ids.contains(&id).then(|| (id, Arc::new(Mutex::new(item))))
                            })
                            .collect(),
                        Err(_) => vec![],
                    };
                    items
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
    use crate::caching::{CacheReader, CacheType, IDed, LineBasedCache};
    use crate::{settings::SongKey, song::Song};
    use once_cell::sync::Lazy;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::{collections::HashSet, sync::Arc};
    use tempfile::TempDir;

    static TEST_FOLDER: Lazy<TempDir> = Lazy::new(|| tempfile::tempdir().unwrap());

    fn random_file() -> PathBuf {
        TEST_FOLDER
            .path()
            .join(format!("{:x}", rand::random::<u64>()))
    }

    static READER: Lazy<Arc<Mutex<CacheType>>> = Lazy::new(|| {
        Arc::new(Mutex::new(CacheType::Line(LineBasedCache {
            filepath: random_file(),
        })))
    });

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

        let mut sc: FileCache<Song> = FileCache {
            reader: READER.clone(),
            map: Default::default(),
        };

        {
            let reader = sc.reader.lock().unwrap();

            let r = reader.clone().extend(songs.clone().into_iter(), false);
            println!["{:?}", r];
            assert![r.is_ok()];

            // Sending the same songs to check the overwriting mechanism
            let r = reader.clone().extend(songs.clone().into_iter(), false);
            println!["{:?}", r];
            assert![r.is_ok()];
        }
        let songs = sc.fetch(&keys);
        println!["{:?}", sc];
        println!["{:?}", songs];
        assert_eq![songs.len(), 3];
    }

    #[test]
    fn checking_unused() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone();
        let keys: HashSet<SongKey> = songs.iter().map(|s| s.id.clone()).collect();

        let mut sc: FileCache<Song> = FileCache {
            reader: READER.clone(),
            map: Default::default(),
        };

        {
            let reader = sc.reader.lock().unwrap();
            let extension_result = reader.clone().extend(songs.into_iter(), false);
            println!["{extension_result:?}"];
            assert![extension_result.is_ok()];
        }
        sc.fetch(&keys);
        let mut unused: Vec<_> = sc.find_unused_items().collect();
        println!["{sc:?}"];
        println!["Unused songs before: {:?}", unused];
        assert_eq![unused.len(), 2];
        {
            let guards = sc.fetch(&[first_key.clone()].into_iter().collect());
            assert![guards.len() > 0];
            println!["Captured guards: {:?}", guards];
            let song = guards[&first_key].lock();
            println!["Captured song: {:?}", song];

            unused = sc.find_unused_items().collect();
            println!["Unused songs during: {:?}", unused];
            assert_eq![unused.len(), 1];
        }
        unused = sc.find_unused_items().collect();
        println!["Unused songs after: {:?}", unused];
        assert_eq![unused.len(), 2];
    }

    #[test]
    fn dropping() {
        let songs = vec![Song::basic(), Song::basic()];
        let first_key = songs[0].id.clone(); // the drop target

        let mut sc: FileCache<Song> = FileCache {
            reader: READER.clone(),
            map: Default::default(),
        };

        {
            let reader = sc.reader.lock().unwrap();
            let r = reader.clone().extend(songs.clone().into_iter(), false);

            println!["{r:?}"];
            assert![r.is_ok()];
        }
        sc.fetch(&songs.into_iter().map(|s| s.id().to_string()).collect());
        println!["SC: {:?}", sc];
        let mut unused: Vec<_> = sc.find_unused_items().collect();
        println!["{:?}", unused];
        assert_eq![unused.len(), 2];
        sc.drop_from_cache([first_key]);
        unused = sc.find_unused_items().collect();
        assert_eq![unused.len(), 1];

        sc.drop_from_cache(unused);
        unused = sc.find_unused_items().collect();
        assert_eq![unused.len(), 0];
    }
}
