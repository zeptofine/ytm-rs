use async_std::{
    fs::{self as afs},
    io::{ReadExt, WriteExt},
};
use futures::{future::join_all, Future};

use std::{borrow::Borrow, collections::HashSet, fs as sfs, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::caching::IDed;

use super::{CacheReader, LineBasedReader, SourceItemPair};

fn random_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub async fn read_file<T: Borrow<PathBuf>>(filepath: T) -> Result<Vec<u8>, async_std::io::Error> {
    let filepath = filepath.borrow();
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

    pub async fn extend_to<T: AsRef<FileData<Vec<u8>>>, V: AsRef<Vec<(T, PathBuf)>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let items: Vec<(&FileData<_>, PathBuf)> = items
            .as_ref()
            .iter()
            .map(|(item, pth)| (item.as_ref(), self.filepath.join(pth)))
            .collect();

        // Write to files
        for (data, filepath) in items.iter() {
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

        Ok(join_all(items).await)
    }

    // Finds the files, but only actually reads files that have the right id
    async fn read_filter(
        &self,
        f: &HashSet<String>,
    ) -> Result<
        Vec<(
            String,
            impl Future<Output = SourceItemPair<String, FileData<Vec<u8>>>>,
        )>,
        std::io::Error,
    > {
        println![
            "Reading {:?} with filter: {:?}",
            self.index_reader.filepath, f
        ];

        let items = self.index_reader.read_filter(f).await?;
        let ids: Vec<String> = items.iter().map(|i| i.0.clone()).collect();
        let futures = join_all(items.into_iter().map(|i| i.1)).await;

        let index: Vec<(String, SourceItemPair<_, FileData<PathBuf>>)> =
            ids.into_iter().zip(futures).collect();
        let mut items = vec![];

        for (id, SourceItemPair(source, FileData(uuid, path_id))) in index {
            println!["Source: {:?}", source];
            let actual = self.filepath.join(path_id);
            items.push((id, async move {
                SourceItemPair(source, FileData(uuid, read_file(actual).await.unwrap()))
            }))
        }

        Ok(items)
    }

    async fn extend<T: AsRef<FileData<Vec<u8>>>, V: AsRef<Vec<T>>>(
        &self,
        items: V,
        overwrite: bool,
    ) -> Result<(), std::io::Error> {
        let items: Vec<(&FileData<_>, PathBuf)> = items
            .as_ref()
            .iter()
            .map(|f| (f.as_ref(), PathBuf::from(random_uuid())))
            .collect();

        self.extend_to(items, overwrite).await
    }
}

#[derive(Debug, Clone)]
pub struct LazyFolderBasedReader {
    pub filepath: PathBuf,
    pub index_reader: LineBasedReader,
}
impl LazyFolderBasedReader {
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

    pub fn generate_paths(
        &self,
        count: usize,
    ) -> std::iter::Map<std::ops::Range<usize>, impl FnMut(usize) -> PathBuf> {
        (0..count).map(|_| PathBuf::from(random_uuid()))
    }

    #[inline]
    pub fn convert_path(&self, path: &PathBuf) -> PathBuf {
        self.filepath.join(path)
    }
}

impl CacheReader<String, String, FileData<PathBuf>> for LazyFolderBasedReader {
    async fn read(&self) -> Result<Vec<SourceItemPair<String, FileData<PathBuf>>>, std::io::Error> {
        let index: Vec<SourceItemPair<_, FileData<PathBuf>>> = self.index_reader.read().await?;

        Ok(index
            .into_iter()
            .map(|SourceItemPair(source, FileData(uuid, path_id))| {
                let actual = self.filepath.join(path_id);
                SourceItemPair(source, FileData(uuid, actual))
            })
            .collect())
    }

    async fn read_filter(
        &self,
        f: &HashSet<String>,
    ) -> Result<
        Vec<(
            String,
            impl Future<Output = SourceItemPair<String, FileData<PathBuf>>>,
        )>,
        std::io::Error,
    > {
        let index: Vec<SourceItemPair<_, FileData<PathBuf>>> = self.index_reader.read().await?;
        Ok(index
            .into_iter()
            .filter_map(
                |SourceItemPair(source, FileData(id, path_id))| match f.contains(&id) {
                    true => {
                        let actual = self.filepath.join(path_id);
                        Some((id.clone(), async move {
                            SourceItemPair(source, FileData(id, actual))
                        }))
                    }
                    false => None,
                },
            )
            .collect())
    }

    async fn extend<T: AsRef<FileData<PathBuf>>, V: AsRef<Vec<T>>>(
        &self,
        _items: V,
        _overwrite: bool,
    ) -> Result<(), std::io::Error> {
        unreachable!() // But truthfully i am too lazy to implement this.
    }
}
