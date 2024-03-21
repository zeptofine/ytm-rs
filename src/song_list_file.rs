use crate::song::Song;
use async_std::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongFileState {
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone)]
pub enum LoadError {
    File,
    Format,
}

#[derive(Debug, Clone)]
pub enum SaveError {
    File,
    Write,
    Format,
}

impl SongFileState {
    pub fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories_next::ProjectDirs::from("rs", "zeptofine", "ytm-rs")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or_default()
        };
        path.push("songlist.json");
        path
    }

    pub async fn load() -> Result<SongFileState, LoadError> {
        let mut contents = String::new();
        let mut file = async_std::fs::File::open(Self::path())
            .await
            .map_err(|_| LoadError::File)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;

        serde_json::from_str(&contents).map_err(|_| LoadError::Format)
    }

    pub async fn save(self) -> Result<std::path::PathBuf, SaveError> {
        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::Format)?;
        let path = Self::path();
        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir)
                .await
                .map_err(|_| SaveError::File)?;
        }

        {
            let mut file = async_std::fs::File::create(&path)
                .await
                .map_err(|_| SaveError::File)?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|_| SaveError::Write)?;
        }

        Ok(path)
    }
}
