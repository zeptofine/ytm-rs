use std::{collections::hash_map::Keys, sync::Arc};

use parking_lot::RwLock;
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

    fn push_cache<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (String, Arc<RwLock<T>>)>,
    {
        self.map.extend(items);
    }
}
