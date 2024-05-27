use super::{IDed, SourceItemPair};

pub trait CacheReader<S, T: IDed> {
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read(&self) -> Result<impl Iterator<Item = SourceItemPair<S, T>>, std::io::Error>;

    fn extend(
        self,
        items: impl IntoIterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error>;
}
