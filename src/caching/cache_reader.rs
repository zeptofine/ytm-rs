use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{IDed, LineItemPair};

pub trait CacheReader {
    /// Creates an iterator from reading the ndjson file and creating items.
    /// The iterator yields a struct of the original line and the created item
    fn read<T: IDed + for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error>;

    fn read_filter<T: IDed + for<'de> Deserialize<'de>>(
        &self,
        ids: HashSet<String>,
    ) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        Ok(self
            .read::<T>()?
            .filter(move |item| ids.contains(item.1.id())))
    }

    fn extend<T: IDed + Serialize + for<'de> Deserialize<'de>>(
        self,
        items: impl Iterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), async_std::io::Error>;
}
