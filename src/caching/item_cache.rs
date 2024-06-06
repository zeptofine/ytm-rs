use std::{
    collections::{hash_map::Keys, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use futures::Future;

pub trait IDed<T> {
    fn id(&self) -> &T;
}

pub type RwMap<K, V> = HashMap<K, Arc<RwLock<V>>>;

pub trait BufferedCache<K: Hash + Clone + Eq + Debug, V: IDed<K>> {
    fn items<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a Arc<RwLock<V>>)>
    where
        K: 'a,
        V: 'a;

    fn keys(&self) -> Keys<'_, K, Arc<RwLock<V>>>;

    fn drop_from_cache(&mut self, keys: impl IntoIterator<Item = K>);

    fn fetch_existing(&self, ids: &HashSet<K>) -> RwMap<K, V> {
        self.items()
            .filter_map(|(key, s)| {
                if ids.contains(key) {
                    Some((key.clone(), Arc::clone(s)))
                } else {
                    None
                }
            })
            .collect()
    }

    async fn read_from_ids_async(
        &self,
        ids: &HashSet<K>,
    ) -> Vec<impl Future<Output = (K, Arc<RwLock<V>>)>>;

    fn push_cache<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (K, Arc<RwLock<V>>)>;
}
