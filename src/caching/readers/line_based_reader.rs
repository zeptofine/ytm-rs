use std::collections::HashSet;
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

use async_std::{fs as afs, io::prelude::BufReadExt};
use async_std::{io as aio, stream::StreamExt};

use fs4::async_std::AsyncFileExt;
use fs4::FileExt;
use serde::{Deserialize, Serialize};

use crate::caching::IDed;

use super::cache_reader::{CacheReader, SourceItemPair};

pub type LineItemPair<T> = SourceItemPair<String, T>;

fn filter_file_items<'a, T: IDed<String>>(
    items: impl Iterator<Item = LineItemPair<T>> + 'a,
    overwrite: bool,
    filter: &'a HashSet<String>,
) -> impl Iterator<Item = Vec<u8>> + 'a {
    items.filter_map(move |SourceItemPair(mut line, item)| {
        line.push('\n');
        let id = item.id();
        match (overwrite, filter.contains(id)) {
            (true, true) => None,
            (true, false) => Some(line.as_bytes().to_vec()),
            (false, true) => Some(line.as_bytes().to_vec()),
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

impl<T: IDed<String> + Serialize + for<'de> Deserialize<'de>> CacheReader<String, String, T>
    for LineBasedReader
{
    async fn read(&self) -> Result<Vec<SourceItemPair<String, T>>, std::io::Error> {
        let file = afs::File::open(&self.filepath).await?;
        println![
            "(READ) LOCKING {:?}: {:?}",
            self.filepath,
            file.lock_shared()
        ];

        let reader = aio::BufReader::new(&file);
        let mut lines = reader.lines();
        let mut vec: Vec<SourceItemPair<String, T>> = Vec::new();
        while let Some(Ok(line)) = lines.next().await {
            serde_json::from_str::<T>(&line).map(|s| vec.push(SourceItemPair(line, s)))?;
        }
        println!["(READ) UNLOCKING {:?}: {:?}", self.filepath, file.unlock()];

        Ok(vec)
    }
    async fn extend<OutT: AsRef<T>, V: AsRef<Vec<OutT>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        {
            let items: HashMap<String, &T> = items
                .as_ref()
                .iter()
                .map(|i| {
                    let r = i.as_ref();
                    (r.id().to_string(), r)
                })
                .collect();

            let filepath = &self.filepath;
            let tempfile = filepath.with_extension("ndjson.tmp");

            {
                // Create the temporary file to write the new list to
                let output_file = File::create(&tempfile)?;
                println![
                    "(XTND) LOCKING {:?}: {:?}",
                    tempfile,
                    output_file.lock_exclusive()
                ];

                let mut out = std::io::BufWriter::new(&output_file);
                {
                    // Read the original list
                    if let Ok(itemlist) = self.read().await {
                        let itemlist: Vec<SourceItemPair<String, T>> = itemlist;

                        // Filter through the lines, find existing keys and skip broken lines
                        let keys = items.keys().cloned().collect();
                        for bytes in filter_file_items(itemlist.into_iter(), overwrite, &keys) {
                            out.write_all(&bytes)?;
                        }

                        out.flush()?;
                    }
                }

                // Add remaining keys to the file
                for (_id, item) in items {
                    let mut json = serde_json::to_string(item).unwrap();
                    json.push('\n');
                    out.write_all(json.as_bytes())?;
                }
                out.flush()?;

                println![
                    "(XTND) UNLOCKING {:?}: {:?}",
                    tempfile,
                    output_file.unlock()
                ];
            }

            // Replace songs.ndjson with songs.ndjson.tmp

            std::fs::rename(&tempfile, filepath)?;

            Ok(())
        }
    }
}
