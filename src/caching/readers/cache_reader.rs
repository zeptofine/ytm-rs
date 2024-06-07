use std::{collections::HashSet, hash::Hash, sync::Arc};

use futures::Future;
use parking_lot::RwLock;

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
    async fn read(&self) -> Result<Vec<SourceItemPair<SrcT, OutT>>, std::io::Error>;

    async fn read_filter(
        &self,
        f: &HashSet<IDT>,
    ) -> Result<Vec<impl Future<Output = SourceItemPair<SrcT, OutT>>>, std::io::Error> {
        Ok(self
            .read()
            .await?
            .into_iter()
            .filter_map(|i| match f.contains(i.1.id()) {
                true => Some(async { i }),
                false => None,
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
                    Ok(iter) => {
                        let items = futures::future::join_all(iter).await;
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
