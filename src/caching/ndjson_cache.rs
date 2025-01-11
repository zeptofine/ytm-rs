use serde::{Deserialize, Serialize};

use super::{readers::LineBasedReader, BufferedCache, IDed, RwMap};

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
    fn items(&self) -> &RwMap<String, T> {
        &self.map
    }
    fn items_mut(&mut self) -> &mut RwMap<String, T> {
        &mut self.map
    }

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = String>) {
        keys.into_iter().for_each(|key| {
            self.map.remove(&key);
        });
    }
}
