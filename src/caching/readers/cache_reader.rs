use std::{collections::HashSet, hash::Hash};

use crate::caching::IDed;

#[derive(Debug, Clone)]
pub struct SourceItemPair<S, T>(
    pub S, // source
    pub T, // result
);

#[allow(unused)]
pub trait CacheReader<SrcT, IDT, OutT>
where
    IDT: Eq + PartialEq + Hash,
    OutT: IDed<IDT>,
{
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read(&self) -> Result<impl Iterator<Item = SourceItemPair<SrcT, OutT>>, std::io::Error>;

    fn read_filter(
        &self,
        f: &HashSet<IDT>,
    ) -> Result<impl Iterator<Item = SourceItemPair<SrcT, OutT>>, std::io::Error> {
        Ok(self
            .read()?
            .filter(move |SourceItemPair(_, o)| f.contains(o.id())))
    }

    /// Extends the cache with the given items.
    fn extend(
        &self,
        items: impl IntoIterator<Item = OutT>,
        overwrite: bool,
    ) -> Result<(), std::io::Error>;
}
