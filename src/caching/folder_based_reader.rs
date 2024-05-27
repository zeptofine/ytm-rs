use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FolderBasedReader {
    pub filepath: PathBuf,
}
