use std::{
    collections::{hash_map::Keys, HashSet},
    sync::{Arc, Mutex, RwLock},
};

use serde::{Deserialize, Serialize};

use super::{
    readers::{CacheReader, LineBasedReader, SourceItemPair},
    BufferedCache, IDed, RwMap,
};

#[derive(Debug, Clone)]
pub struct NDJsonCache<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> {
    map: RwMap<String, T>,
    pub reader: Arc<Mutex<LineBasedReader>>,
}
impl<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> NDJsonCache<T> {
    pub fn new(cache: LineBasedReader) -> Self {
        let parent = cache.filepath.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }

        Self {
            reader: Arc::new(Mutex::new(cache)),
            map: Default::default(),
        }
    }
}
impl<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> From<Arc<Mutex<LineBasedReader>>>
    for NDJsonCache<T>
{
    fn from(value: Arc<Mutex<LineBasedReader>>) -> Self {
        Self {
            reader: value,
            map: Default::default(),
        }
    }
}

impl<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> BufferedCache<String, T>
    for NDJsonCache<T>
{
    fn items<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a Arc<RwLock<T>>)>
    where
        T: 'a,
    {
        self.map.iter()
    }

    fn keys(&self) -> Keys<'_, String, Arc<RwLock<T>>> {
        self.map.keys()
    }

    fn cache_size(&self) -> usize {
        self.map.len()
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }

    fn cache_ids(
        &mut self,
        ids: &HashSet<String>,
    ) -> impl Iterator<Item = (String, Arc<RwLock<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let reader = self.reader.lock().unwrap();
                    let items = reader
                        .read()
                        .map(|iter| {
                            iter.filter_map(move |SourceItemPair(_, item): SourceItemPair<_, T>| {
                                let id = item.id().to_string();
                                ids.contains(&id).then(|| (id, Arc::new(RwLock::new(item))))
                            })
                            .collect()
                        })
                        .unwrap_or_default();
                    items
                };
                self.map.extend(items.clone());
                items.into_iter().map(|(str, _)| {
                    let arc = self.new_rw(&str);
                    (str.to_string(), arc)
                })
            })
            .into_iter()
            .flatten()
    }

    fn new_rw(&self, id: &String) -> Arc<RwLock<T>> {
        Arc::clone(&self.map[id])
    }
}
