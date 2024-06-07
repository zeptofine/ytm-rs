use std::{fmt::Debug, sync::Arc};

use parking_lot::RwLock;

use super::{readers::FolderBasedReader, BufferedCache, IDed, RwMap};

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

    fn push_cache<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (String, Arc<RwLock<T>>)>,
    {
        self.map.extend(items);
    }
}
