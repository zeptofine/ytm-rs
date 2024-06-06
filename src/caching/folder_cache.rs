use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use futures::Future;

use super::{
    readers::{CacheReader, FolderBasedReader},
    BufferedCache, IDed, RwMap,
};

#[derive(Debug, Clone)]
pub struct FolderCache<T: IDed<String>> {
    map: RwMap<String, T>,
    pub reader: FolderBasedReader,
}
impl<T: IDed<String> + Debug> FolderCache<T> {
    pub fn new(reader: FolderBasedReader) -> Self {
        Self {
            map: Default::default(),
            reader,
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

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|id| {
            self.map.remove(&id);
        })
    }

    async fn read_from_ids_async(
        &self,
        ids: &std::collections::HashSet<String>,
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
                            .map(move |sip| async {
                                println!["Creating item"];
                                let item = sip.1;
                                let id = item.id().to_string();

                                let data = item.into_data();
                                println!["Created data"];
                                (id.clone(), Arc::new(RwLock::new((id, data).into())))
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
