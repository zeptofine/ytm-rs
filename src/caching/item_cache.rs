use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use super::{CacheReader, LineBasedReader};

#[derive(Debug, Clone)]
pub struct SourceItemPair<S, T>(
    pub S, // source
    pub T, // result
);

pub trait IDed {
    fn id(&self) -> &str;
}

pub type CacheMap<T> = HashMap<String, Arc<Mutex<T>>>;

pub trait BufferedCache<S, T: IDed> {
    /// Filters out the items that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    fn find_unused_items(&self) -> impl Iterator<Item = String> + '_;

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>);

    fn fetch(&mut self, ids: &HashSet<String>) -> CacheMap<T>;

    fn extend<R: CacheReader<S, T> + Debug + Clone>(
        reader: Arc<Mutex<R>>,
        items: impl IntoIterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error>;

    fn cache_ids(
        &mut self,
        ids: &HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_;

    fn new_arc(&self, id: &str) -> Arc<Mutex<T>>;

    fn cache_size(&self) -> usize;
}

pub type LineItemPair<T> = SourceItemPair<String, T>;

#[derive(Debug, Clone)]
pub struct FileCache<T: Serialize + for<'de> Deserialize<'de> + IDed> {
    map: CacheMap<T>,
    pub reader: Arc<Mutex<LineBasedReader>>,
}
impl<T: Serialize + for<'de> Deserialize<'de> + IDed> FileCache<T> {
    pub fn new(cache: LineBasedReader) -> Self {
        Self {
            reader: Arc::new(Mutex::new(cache)),
            map: Default::default(),
        }
    }
}

impl<T: Serialize + for<'de> Deserialize<'de> + IDed> BufferedCache<String, T> for FileCache<T> {
    /// Filters out the items that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    fn find_unused_items(&self) -> impl Iterator<Item = String> + '_ {
        self.map.iter().filter_map(|(key, s)| {
            let count = Arc::strong_count(s);
            match count == 1 {
                true => Some(key.clone()),
                false => None,
            }
        })
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }

    fn fetch(&mut self, ids: &HashSet<String>) -> CacheMap<T> {
        let cs_keys: HashSet<String> = self.map.keys().cloned().collect();

        match cs_keys.is_empty() {
            true => self.cache_ids(ids).collect(),
            false => {
                let already_cached = ids.intersection(&cs_keys);
                let not_cached: HashSet<_> = ids.difference(&cs_keys).cloned().collect();

                // chain caches from ndjson if not_cached is not empty
                let get_cache = (!not_cached.is_empty())
                    .then(|| {
                        self.cache_ids(&not_cached)
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

    fn extend<R: CacheReader<String, T> + Debug + Clone>(
        reader: Arc<Mutex<R>>,
        items: impl IntoIterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error> {
        let reader = reader.lock().unwrap();
        reader.clone().extend(items, overwrite)
    }

    fn cache_ids(
        &mut self,
        ids: &HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let reader = self.reader.lock().unwrap();
                    let items = match reader.read() {
                        Ok(iter) => iter
                            .filter_map(move |SourceItemPair(_, item): LineItemPair<T>| {
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

    fn cache_size(&self) -> usize {
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
    use crate::caching::item_cache::BufferedCache;
    use crate::caching::{CacheReader, IDed, LineBasedReader};
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

    static READER: Lazy<Arc<Mutex<LineBasedReader>>> = Lazy::new(|| {
        Arc::new(Mutex::new(LineBasedReader {
            filepath: random_file(),
        }))
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
