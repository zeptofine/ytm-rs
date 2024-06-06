use async_std::{
    fs::{self as afs},
    io::{ReadExt, WriteExt},
};
use futures::{future, Future};
use std::{collections::HashSet, fs as sfs, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::caching::IDed;

use super::{CacheReader, LineBasedReader, SourceItemPair};

fn random_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub async fn read_file(filepath: PathBuf) -> Result<Vec<u8>, async_std::io::Error> {
    println!["Reading data of: {:?}", filepath];
    let mut file = afs::File::open(filepath).await?;
    let mut data = Vec::with_capacity(file.metadata().await.map(|m| m.len()).unwrap_or(0) as usize); // approximate the file size
    let _ = file.read_to_end(&mut data).await;
    println!["Finished reading, {:?} bytes", data.len()];
    Ok(data)
}

pub async fn write_file(filepath: PathBuf, data: &[u8]) -> Result<(), async_std::io::Error> {
    println!["Writing data to: {:?}", filepath];
    let len = data.len();
    {
        let mut file = afs::File::create(&filepath).await?;
        file.write_all(data).await?;
        file.flush().await?;
    }
    println!["{:?} bytes written.", len];
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileData<T>(String, T);
// I cannot believe this can't be derived
impl<T> AsRef<FileData<T>> for FileData<T> {
    #[inline]
    fn as_ref(&self) -> &FileData<T> {
        self
    }
}

impl<T> FileData<T> {
    pub fn new(id: String, data: T) -> Self {
        FileData(id, data)
    }
    #[inline]
    pub fn into_data(self) -> T {
        self.1
    }
}
impl<T> IDed<String> for FileData<T> {
    #[inline]
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
        if !filepath.exists() {
            std::fs::create_dir_all(&filepath).unwrap();
        }

        if !linepath.exists() {
            // touch
            println!["Creating index file at {:?}...", linepath];
            println!["{:?}", sfs::File::create(&linepath)];
        }
        Self {
            filepath,
            index_reader: LineBasedReader::new(linepath),
        }
    }
}
impl CacheReader<String, String, FileData<Vec<u8>>> for FolderBasedReader {
    // Returns an iterator of pairs of the key and the File
    async fn read(&self) -> Result<Vec<SourceItemPair<String, FileData<Vec<u8>>>>, std::io::Error> {
        // Read the index file and find the filenames
        let index: Vec<SourceItemPair<_, FileData<PathBuf>>> = self.index_reader.read().await?;

        let mut items = Vec::new();

        for SourceItemPair(source, FileData(uuid, path_id)) in index {
            println!["Source: {:?}", source];
            let actual = self.filepath.join(&path_id);
            let data_future = read_file(actual);
            items.push(async { SourceItemPair(source, FileData(uuid, data_future.await.unwrap())) })
        }

        Ok(future::join_all(items).await)
    }

    // Finds the files, but only actually reads files that have the right id
    async fn read_filter(
        &self,
        filter: &HashSet<String>,
    ) -> Result<Vec<impl Future<Output = SourceItemPair<String, FileData<Vec<u8>>>>>, std::io::Error>
    {
        println!["Reading with filter: {:?}", filter];

        let index: Vec<SourceItemPair<_, FileData<PathBuf>>> =
            futures::future::join_all(self.index_reader.read_filter(filter).await?).await;
        let mut items = vec![];

        for SourceItemPair(source, FileData(uuid, path_id)) in index {
            println!["Source: {:?}", source];
            let actual = self.filepath.join(path_id);
            items.push(async move {
                SourceItemPair(source, FileData(uuid, read_file(actual).await.unwrap()))
            })
        }

        Ok(items)
    }

    async fn extend<T: AsRef<FileData<Vec<u8>>>, V: AsRef<Vec<T>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let items: Vec<(&FileData<_>, String)> = items
            .as_ref()
            .iter()
            .map(|f| (f.as_ref(), random_uuid()))
            .collect();

        // Write to files
        for (data, uuid) in items.iter() {
            let filepath = self.filepath.join(uuid);
            match (filepath.exists(), overwrite) {
                // Does not exist and overwrite is allowed
                // Does exist but overwrite is allowed
                // Does not exist and overwrite is not allowed
                (false, true) | (true, true) | (false, false) => {
                    write_file(filepath.clone(), &data.1).await?;
                }
                // Does exist and overwrite is not allowed
                (true, false) => {}
            }
        }

        // Extend the index
        let new_items: Vec<_> = items
            .iter()
            .map(|(data, uuid)| -> FileData<PathBuf> { FileData(data.0.to_string(), uuid.into()) })
            .collect();
        self.index_reader.clone().extend(new_items, overwrite).await
    }
}
