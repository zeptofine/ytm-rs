use serde::{Deserialize, Serialize};

use crate::song_operations::SongOpConstructor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: uuid::Uuid,
    pub name: String,
    pub constructor: SongOpConstructor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistHeader {
    pub id: uuid::Uuid,
    pub name: String,
    pub num_of_songs: usize,
}
impl From<&Playlist> for PlaylistHeader {
    fn from(value: &Playlist) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            num_of_songs: value.constructor.all_song_keys_rec().count(),
        }
    }
}
