use std::fmt::Debug;

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
    fn items(&self) -> &RwMap<String, T> {
        &self.map
    }
    fn items_mut(&mut self) -> &mut RwMap<String, T> {
        &mut self.map
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|id| {
            self.map.remove(&id);
        })
    }
}
