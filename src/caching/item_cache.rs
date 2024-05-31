use std::{
    collections::{hash_map::Keys, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex},
};

use super::readers::CacheReader;

pub trait IDed<T> {
    fn id(&self) -> &T;
}

pub type ArcMap<K, V> = HashMap<K, Arc<Mutex<V>>>;

pub trait BufferedCache<K: Hash + Clone + Eq, V: IDed<K>> {
    fn items<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a Arc<Mutex<V>>)>
    where
        K: 'a,
        V: 'a;

    fn keys(&self) -> Keys<'_, K, Arc<Mutex<V>>>;

    fn cache_size(&self) -> usize;

    /// Filters out the items that have only one refcount,
    /// meaning they are no longer being used by anything other than the map
    fn find_unused_items<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
        V: 'a,
    {
        self.items()
            .filter_map(|(key, s)| (Arc::strong_count(s) == 1).then_some(key))
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = K>);

    fn fetch(&mut self, ids: &HashSet<K>) -> ArcMap<K, V> {
        let cs_keys: HashSet<K> = self.keys().cloned().collect();

        match cs_keys.is_empty() {
            true => self.cache_ids(ids).collect(),
            false => {
                let not_cached: HashSet<_> = ids.difference(&cs_keys).cloned().collect();

                // chain caches from ndjson if not_cached is not empty
                let get_cache = (!not_cached.is_empty())
                    .then(|| self.cache_ids(&not_cached).collect::<Vec<_>>())
                    .into_iter()
                    .flatten();
                ids.intersection(&cs_keys) // Already in cache
                    .map(|k| (k.clone(), self.new_arc(k)))
                    .chain(get_cache)
                    .collect()
            }
        }
    }

    fn extend<R: CacheReader<K, K, V> + Debug + Clone>(
        reader: Arc<Mutex<R>>,
        items: impl IntoIterator<Item = V>,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let reader = reader.lock().unwrap();
        reader.clone().extend(items, overwrite)
    }

    fn cache_ids(&mut self, ids: &HashSet<K>) -> impl Iterator<Item = (K, Arc<Mutex<V>>)> + '_;

    fn new_arc(&self, id: &K) -> Arc<Mutex<V>>;
}

#[derive(Default, Debug, Clone)]
pub struct CacheInterface<K: Hash + Eq + PartialEq + Clone, V: ?Sized> {
    cache: HashMap<K, Arc<Mutex<V>>>,
    keys: HashSet<K>,
}

impl<S: Hash + Eq + PartialEq + Clone, T: IDed<S>> CacheInterface<S, T> {
    pub fn get<'a>(&'a self, ids: &'a HashSet<S>) -> impl Iterator<Item = (S, Arc<Mutex<T>>)> + '_ {
        let existing = ids.intersection(&self.keys).cloned();
        existing.map(|k| (k.clone(), Arc::clone(&self.cache[&k])))
    }

    pub fn extend(&mut self, items: ArcMap<S, T>) {
        let new_keys: HashSet<S> = items.keys().cloned().collect();
        self.keys = self.keys.union(&new_keys).cloned().collect();
        self.cache.extend(items)
    }

    pub fn replace(&mut self, cache: ArcMap<S, T>) {
        self.keys = cache.keys().cloned().collect();
        self.cache = cache;
    }

    /// Returns the number of elements in the cache
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn get_keys(&self) -> &HashSet<S> {
        &self.keys
    }
}

#[cfg(test)]
mod tests {
    use crate::caching::item_cache::BufferedCache;
    use crate::caching::readers::{CacheReader, LineBasedReader};
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

    use crate::caching::{IDed, NDJsonCache};

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

        let mut sc: NDJsonCache<Song> = NDJsonCache::from(READER.clone());

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

        let mut sc: NDJsonCache<Song> = NDJsonCache::from(READER.clone());

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
            assert![!guards.is_empty()];
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

        let mut sc: NDJsonCache<Song> = NDJsonCache::from(READER.clone());

        {
            let reader = sc.reader.lock().unwrap();
            let r = reader.clone().extend(songs.clone(), false);

            println!["{r:?}"];
            assert![r.is_ok()];
        }
        sc.fetch(&songs.into_iter().map(|s| s.id().to_string()).collect());
        println!["SC: {:?}", sc];
        let mut unused: Vec<String> = sc.find_unused_items().cloned().collect();
        println!["{:?}", unused];
        assert_eq![unused.len(), 2];
        sc.drop_from_cache([first_key]);
        unused = sc.find_unused_items().cloned().collect();
        assert_eq![unused.len(), 1];

        sc.drop_from_cache(unused);
        assert_eq![sc.find_unused_items().collect::<Vec<_>>().len(), 0];
    }
}
