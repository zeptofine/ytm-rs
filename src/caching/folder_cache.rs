use std::sync::{Arc, Mutex};

use super::{
    readers::{CacheReader, FileData, FolderBasedReader, SourceItemPair},
    ArcMap, BufferedCache, IDed,
};

#[derive(Debug, Clone)]
pub struct FolderCache<T: IDed<String>> {
    map: ArcMap<String, T>,
    pub reader: Arc<Mutex<FolderBasedReader>>,
}
impl<T: IDed<String>> FolderCache<T> {
    pub fn new(reader: FolderBasedReader) -> Self {
        Self {
            map: Default::default(),
            reader: Arc::new(Mutex::new(reader)),
        }
    }
}
impl<T: IDed<String> + From<(String, Vec<u8>)>> BufferedCache<String, T> for FolderCache<T> {
    fn items<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a Arc<Mutex<T>>)>
    where
        String: 'a,
        T: 'a,
    {
        self.map.iter()
    }

    fn keys(&self) -> std::collections::hash_map::Keys<'_, String, Arc<Mutex<T>>> {
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
    ) -> impl Iterator<Item = (String, Arc<Mutex<T>>)> + '_ {
        (!ids.is_empty())
            .then(|| {
                let items: Vec<_> = {
                    let reader = self.reader.lock().unwrap();
                    let items = reader
                        .read_filter(ids)
                        .map(|iter| {
                            iter.filter_map(
                                move |SourceItemPair(_, item): SourceItemPair<
                                    String,
                                    FileData<Vec<u8>>,
                                >| {
                                    let id = item.id().to_string();

                                    ids.contains(&id).then(|| {
                                        (
                                            id.clone(),
                                            Arc::new(Mutex::new((id, item.into_data()).into())),
                                        )
                                    })
                                },
                            )
                            .collect()
                        })
                        .unwrap_or_default();
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

    fn new_arc(&self, id: &String) -> Arc<Mutex<T>> {
        Arc::clone(&self.map[id])
    }
}
