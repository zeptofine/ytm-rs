use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

use parking_lot::RwLock;

pub trait IDed<T> {
    fn id(&self) -> &T;
}

pub type RwMap<K, V> = HashMap<K, Arc<RwLock<V>>>;

pub trait BufferedCache<K: Hash + Clone + Eq + Debug, V: IDed<K>> {
    fn items(&self) -> &HashMap<K, Arc<RwLock<V>>>;
    fn items_mut(&mut self) -> &mut HashMap<K, Arc<RwLock<V>>>;

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = K>);

    fn fetch_existing(&self, ids: &HashSet<K>) -> RwMap<K, V> {
        self.items()
            .iter()
            .filter_map(|(key, s)| {
                if ids.contains(key) {
                    Some((key.clone(), Arc::clone(s)))
                } else {
                    None
                }
            })
            .collect()
    }
}
