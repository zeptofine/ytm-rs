use std::{collections::HashSet, hash::Hash};

use futures::Future;

use crate::caching::IDed;

#[derive(Debug, Clone)]
pub struct SourceItemPair<S, T>(
    pub S, // source
    pub T, // result
);

pub trait CacheReader<SrcT, IDT, OutT>
where
    IDT: Eq + PartialEq + Hash,
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
    /// Extends the cache with the given items.
    async fn extend<T: AsRef<OutT>, V: AsRef<Vec<T>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error>;
}
