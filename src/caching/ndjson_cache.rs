use std::{
    collections::{hash_map::Keys, HashSet},
    sync::{Arc, RwLock},
};

use futures::prelude::Future;
use serde::{Deserialize, Serialize};

use super::{
    readers::{CacheReader, LineBasedReader, SourceItemPair},
    BufferedCache, IDed, RwMap,
};

#[derive(Debug, Clone)]
pub struct NDJsonCache<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> {
    map: RwMap<String, T>,
    pub reader: LineBasedReader,
}
impl<T: Serialize + for<'de> Deserialize<'de> + IDed<String>> NDJsonCache<T> {
    pub fn new(cache: LineBasedReader) -> Self {
        let parent = cache.filepath.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }

        Self {
            reader: cache,
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

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }

    async fn read_from_ids_async(
        &self,
        ids: &HashSet<String>,
    ) -> Vec<impl Future<Output = (String, Arc<RwLock<T>>)>> {
        let items = match ids.is_empty() {
            true => vec![],
            false => {
                let futures = self.reader.read_filter(ids).await;
                match futures {
                    Ok(iter) => {
                        let items = futures::future::join_all(iter).await;
                        items
                            .into_iter()
                            .map(move |SourceItemPair(_, item): SourceItemPair<_, T>| async {
                                let id = item.id().to_string();
                                (id, Arc::new(RwLock::new(item)))
                            })
                            .collect()
                    }
                    Err(e) => {
                        println!["Error: {e:?}"];
                        vec![]
                    }
                }
            }
        };
        items
    }

    fn push_cache<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (String, Arc<RwLock<T>>)>,
    {
        self.map.extend(items);
    }
}
