use serde::{Deserialize, Serialize};

use super::{IDed, LineItemPair};

pub trait CacheReader {
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read<T: IDed + for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error>;

    fn extend<T: IDed + Serialize + for<'de> Deserialize<'de>>(
        self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error>;
}
