use parking_lot::RwLock;
use std::sync::Arc;

mod folder_cache;
mod item_cache;
mod ndjson_cache;
pub mod readers;
mod sound_data;

pub use folder_cache::*;
pub use item_cache::*;
pub use ndjson_cache::*;
pub use sound_data::*;

use crate::{
    settings::{song_audio_path, song_metadata_path, thumbnails_directory},
    song::Song,
};
use readers::{FolderBasedReader, LazyFolderBasedReader, LineBasedReader};

#[derive(Debug)]
pub struct YtmrsCache {
    pub song_metadata: RwArc<NDJsonCache<Song>>,
    pub sounds: FolderCache<BasicSoundData>,
    pub thumbnails: LazyFolderBasedReader,
}

impl Default for YtmrsCache {
    fn default() -> Self {
        Self {
            song_metadata: Arc::new(RwLock::new(NDJsonCache::new(LineBasedReader::new(
                song_metadata_path(),
            )))),
            sounds: FolderCache::new(FolderBasedReader::new(song_audio_path())),
            thumbnails: LazyFolderBasedReader::new(thumbnails_directory()),
        }
    }
}
