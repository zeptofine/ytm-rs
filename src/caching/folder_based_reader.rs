use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::caching::{CacheReader, IDed, LineBasedReader, LineItemPair, SourceItemPair};

fn random_uuid() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Serialize, Deserialize)]
pub struct FileData<T>(String, T);

impl<T> FileData<T> {
    pub fn new(id: String, data: T) -> Self {
        FileData(id, data)
    }
}

impl<T> IDed<String> for FileData<T> {
    fn id(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct FolderBasedReader {
    pub filepath: PathBuf,
    pub index_reader: LineBasedReader,
}

impl FolderBasedReader {
    pub fn new(filepath: PathBuf) -> Self {
        let linepath = filepath.join("index").with_extension("ndjson");
        if !linepath.exists() {
            // touch
            let _ = OpenOptions::new().truncate(true).open(&linepath);
        }
        Self {
            filepath,
            index_reader: LineBasedReader::new(linepath),
        }
    }
}
impl CacheReader<String, String, FileData<Vec<u8>>> for FolderBasedReader {
    // Returns an iterator of pairs of the key and the File
    fn read(
        &self,
    ) -> Result<impl Iterator<Item = SourceItemPair<String, FileData<Vec<u8>>>>, std::io::Error>
    {
        // Read the index file and find the filenames
        Ok(self.index_reader.read()?.filter_map(
            |SourceItemPair(id, FileData(uuid, path_id)): LineItemPair<FileData<PathBuf>>| {
                let actual = self.filepath.join(path_id);

                let mut buffer = vec![];
                let mut file = File::open(actual).ok()?;
                let _ = file.read_to_end(&mut buffer);
                Some(SourceItemPair(id, FileData(uuid, buffer)))
            },
        ))
    }

    // Finds the files, but only actually reads files that have the right id
    fn read_filter(
        &self,
        filter: HashSet<String>,
    ) -> Result<impl Iterator<Item = SourceItemPair<String, FileData<Vec<u8>>>>, std::io::Error>
    {
        Ok(self.index_reader.read()?.filter_map(
            move |SourceItemPair(id, FileData(uuid, path_id)): LineItemPair<FileData<PathBuf>>| {
                if !filter.contains(&id) {
                    return None;
                }

                let actual = self.filepath.join(path_id);
                let mut buffer = vec![];
                let mut file = File::open(actual).ok()?;
                let _ = file.read_to_end(&mut buffer);
                Some(SourceItemPair(id, FileData(uuid, buffer)))
            },
        ))
    }

    fn extend(
        &self,
        items: impl IntoIterator<Item = FileData<Vec<u8>>>,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let items: HashMap<String, (Vec<u8>, String)> = items
            .into_iter()
            .map(|f| (f.0, (f.1, random_uuid())))
            .collect();

        for (data, uuid) in items.values() {
            let filepath = self.filepath.join(uuid);
            match (filepath.exists(), overwrite) {
                // Does not exist and overwrite is allowed
                // Does exist but overwrite is allowed
                // Does not exist and overwrite is not allowed
                (false, true) | (true, true) | (false, false) => {
                    let mut file = File::create(&filepath)?;
                    file.write_all(data)?;
                    file.flush()?;
                }
                // Does exist and overwrite is not allowed
                (true, false) => {} // Do nothing (?)
            }
        }

        self.index_reader.clone().extend(
            items.iter().map(|(id, (_, uuid))| -> FileData<PathBuf> {
                FileData(id.to_string(), uuid.into())
            }),
            overwrite,
        )
    }
}
