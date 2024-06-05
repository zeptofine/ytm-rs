use std::{
    fmt::Debug,
    sync::{Arc, Mutex, RwLock},
};

use super::{
    readers::{CacheReader, FileData, FolderBasedReader, SourceItemPair},
    BufferedCache, IDed, RwMap,
};

#[derive(Debug, Clone)]
pub struct FolderCache<T: IDed<String>> {
    map: RwMap<String, T>,
    pub reader: Arc<Mutex<FolderBasedReader>>,
}
impl<T: IDed<String> + Debug> FolderCache<T> {
    pub fn new(reader: FolderBasedReader) -> Self {
        Self {
            map: Default::default(),
            reader: Arc::new(Mutex::new(reader)),
        }
    }
}
impl<T: Debug + IDed<String> + From<(String, Vec<u8>)>> BufferedCache<String, T>
    for FolderCache<T>
{
    fn items<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a Arc<RwLock<T>>)>
    where
        String: 'a,
        T: 'a,
    {
        self.map.iter()
    }

    fn keys(&self) -> std::collections::hash_map::Keys<'_, String, Arc<RwLock<T>>> {
        self.map.keys()
    }

    fn cache_size(&self) -> usize {
        self.map.len()
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|id| {
            self.map.remove(&id);
        })
    }

    fn cache_ids(
        &mut self,
        ids: &std::collections::HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<RwLock<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let reader = self.reader.lock().unwrap();
                    let items = match reader.read().map(|iter| {
                        iter.filter_map(move |sip: SourceItemPair<String, FileData<Vec<u8>>>| {
                            let item = sip.1;

                            let id = item.id().to_string();

                            ids.contains(&id).then(|| {
                                (
                                    id.clone(),
                                    Arc::new(RwLock::new((id, item.into_data()).into())),
                                )
                            })
                        })
                        .collect()
                    }) {
                        Ok(v) => v,
                        Err(e) => {
                            println!["Error: {e:?}"];
                            vec![]
                        }
                    };
                    items
                };
                self.map.extend(items.clone());

                items
                    .into_iter()
                    .map(|(str, arc)| (str.to_string(), Arc::clone(&arc)))
            })
            .into_iter()
            .flatten()
    }

    fn new_rw(&self, id: &String) -> Arc<RwLock<T>> {
        Arc::clone(&self.map[id])
    }

    fn find_unused_items<'a>(&'a self) -> impl Iterator<Item = &'a String>
    where
        String: 'a,
        T: 'a,
    {
        self.items()
            .filter_map(|(key, s)| (Arc::strong_count(s) == 1).then_some(key))
    }

    fn fetch(&mut self, ids: &std::collections::HashSet<String>) -> RwMap<String, T> {
        let cs_keys: std::collections::HashSet<String> = self.keys().cloned().collect();
        // println!["Cs keys: {:?}", cs_keys];

        match cs_keys.is_empty() {
            true => self.cache_ids(ids).collect(),
            false => {
                let not_cached: std::collections::HashSet<_> =
                    ids.difference(&cs_keys).cloned().collect();

                // chain caches from ndjson if not_cached is not empty
                let get_cache = (!not_cached.is_empty())
                    .then(|| self.cache_ids(&not_cached).collect::<Vec<_>>())
                    .into_iter()
                    .flatten();
                ids.intersection(&cs_keys) // Already in cache
                    .map(|k| (k.clone(), self.new_rw(k)))
                    .chain(get_cache)
                    .collect()
            }
        }
    }

    fn extend<R: CacheReader<String, String, T> + std::fmt::Debug + Clone>(
        reader: Arc<Mutex<R>>,
        items: impl IntoIterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let reader = reader.lock().unwrap();
        reader.clone().extend(items, overwrite)
    }
}
