use futures::Future;
use parking_lot::RwLock;
use std::io::Result as IoResult;
use std::{collections::HashSet, hash::Hash, sync::Arc};

use crate::caching::IDed;

#[derive(Debug, Clone)]
pub struct SourceItemPair<S, T>(
    pub S, // source
    pub T, // result
);

pub trait CacheReader<SrcT, IDT, OutT>
where
    IDT: Eq + PartialEq + Hash + Clone,
    OutT: IDed<IDT>,
{
    async fn read(&self) -> IoResult<Vec<SourceItemPair<SrcT, OutT>>>;

    async fn read_filter(
        &self,
        f: &HashSet<IDT>,
    ) -> IoResult<Vec<(IDT, impl Future<Output = SourceItemPair<SrcT, OutT>>)>> {
        Ok(self
            .read()
            .await?
            .into_iter()
            .filter_map(|i| {
                let id = i.1.id().clone();
                match f.contains(&id) {
                    true => Some((id, async { i })),
                    false => None,
                }
            })
            .collect())
    }

    async fn read_from_ids(
        &self,
        ids: &HashSet<IDT>,
    ) -> Vec<impl Future<Output = (IDT, Arc<RwLock<OutT>>)>> {
        match ids.is_empty() {
            true => vec![],
            false => {
                let futures = self.read_filter(ids).await;
                match futures {
                    Ok(v) => {
                        let futures = v.into_iter().map(|(_, f)| f);
                        let items = futures::future::join_all(futures).await;
                        items
                            .into_iter()
                            .map(
                                move |SourceItemPair(_, item): SourceItemPair<_, OutT>| async {
                                    let id = item.id();
                                    (id.clone(), Arc::new(RwLock::new(item)))
                                },
                            )
                            .collect()
                    }
                    Err(e) => {
                        println!["Error: {e:?}"];
                        vec![]
                    }
                }
            }
        }
    }

    /// Extends the cache with the given items.
    async fn extend<T: AsRef<OutT>, V: AsRef<Vec<T>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error>;
}
