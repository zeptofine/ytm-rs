use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use super::{CacheReader, IDed, LineItemPair, SourceItemPair};

fn filter_file_items<'a, T: IDed>(
    items: impl Iterator<Item = LineItemPair<T>> + 'a,
    overwrite: bool,
    filter: &'a mut HashMap<String, T>,
) -> impl Iterator<Item = Vec<u8>> + 'a {
    items.filter_map(move |SourceItemPair(mut line, item)| {
        line.push('\n');
        let id = item.id();
        match (overwrite, filter.contains_key(id)) {
            (true, true) => None,
            (true, false) => Some(line.as_bytes().to_vec()),
            (false, true) => {
                filter.remove(id);
                Some(line.as_bytes().to_vec())
            }
            (false, false) => Some(line.as_bytes().to_vec()),
        }
    })
}

#[derive(Debug, Clone)]
pub struct LineBasedReader {
    pub filepath: PathBuf,
}
impl LineBasedReader {
    pub fn new(filepath: PathBuf) -> Self {
        Self { filepath }
    }
}

impl<T: IDed + Serialize + for<'de> Deserialize<'de>> CacheReader<String, T> for LineBasedReader {
    fn read(&self) -> Result<impl Iterator<Item = LineItemPair<T>>, std::io::Error> {
        std::fs::File::open(&self.filepath).map(|file| {
            std::io::BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .filter_map(|l| {
                    serde_json::from_str::<T>(&l)
                        .ok()
                        .map(|s| SourceItemPair(l, s))
                })
        })
    }

    fn extend(
        self,
        items: impl IntoIterator<Item = T>,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let mut items: HashMap<String, T> =
            items.into_iter().map(|i| (i.id().to_string(), i)).collect();

        let filepath = &self.filepath;
        let tempfile = filepath.with_extension("ndjson.tmp");

        {
            // Create the temporary file to write the new list to
            let output_file = std::fs::File::create(&tempfile)?;
            let mut out = std::io::BufWriter::new(output_file);
            {
                // Read the original list
                if let Ok(itemlist) = self.read() {
                    // Filter through the lines, find existing keys and skip broken lines
                    let valid_items = filter_file_items(itemlist, overwrite, &mut items);

                    for bytes in valid_items {
                        out.write_all(&bytes)?;
                    }

                    out.flush()?;
                }
            }

            // Add remaining keys to the file
            for (_id, item) in items {
                let mut json = serde_json::to_string(&item).unwrap();
                json.push('\n');
                out.write_all(json.as_bytes())?;
            }
            out.flush()?;
        }

        // Replace songs.ndjson with songs.ndjson.tmp
        std::fs::rename(&tempfile, filepath)?;

        Ok(())
    }
}
