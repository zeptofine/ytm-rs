use serde::{Deserialize, Serialize};

use crate::song_operations::SongOpConstructor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: uuid::Uuid,
    pub name: String,
    pub constructor: SongOpConstructor,
}
