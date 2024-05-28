use super::{IDed, SourceItemPair};

pub trait CacheReader<SrcT, Id, OutT: IDed<Id>> {
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read(&self) -> Result<impl Iterator<Item = SourceItemPair<SrcT, OutT>>, std::io::Error>;

    fn extend(
        self,
        items: impl IntoIterator<Item = OutT>,
        overwrite: bool,
    ) -> Result<(), std::io::Error>;
}
