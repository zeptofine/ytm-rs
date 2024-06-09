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

pub type RwArc<T> = Arc<RwLock<T>>;
pub type RwMap<K, V> = HashMap<K, RwArc<V>>;

pub trait ToRwMapExt<K, V>: IntoIterator<Item = (K, V)> + Sized
where
    K: Hash + Clone + Eq + Debug,
{
    fn to_rwmap(self) -> RwMap<K, V> {
        self.into_iter()
            .map(|(k, v)| (k.clone(), Arc::new(RwLock::new(v))))
            .collect()
    }
}

impl<K: Hash + Clone + Eq + Debug, V, T: IntoIterator<Item = (K, V)>> ToRwMapExt<K, V> for T {}

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
