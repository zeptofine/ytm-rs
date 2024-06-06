use std::path::PathBuf;

use async_std::prelude::*;
use serde::{Deserialize, Serialize};

use crate::playlist::Playlist;

pub type SongKey = String;

pub fn project_data_dir() -> PathBuf {
    if let Some(project_dirs) = directories_next::ProjectDirs::from("rs", "zeptofine", "ytm-rs") {
        project_dirs.data_dir().into()
    } else {
        std::env::current_dir().unwrap_or_default()
    }
}

pub fn settings_path() -> PathBuf {
    let mut path = project_data_dir();
    path.push("songlist.json");
    path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMRUserSettings {
    pub volume: f32,
}

impl Default for YTMRUserSettings {
    fn default() -> Self {
        Self { volume: 1.0 }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct YTMRSettings {
    pub playlist: Playlist,
    pub user: YTMRUserSettings,
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

impl YTMRSettings {
    pub async fn load_default() -> Result<YTMRSettings, LoadError> {
        Self::load(settings_path()).await
    }

    pub async fn load(path: PathBuf) -> Result<Self, LoadError> {
        let mut contents = String::new();
        let mut file = async_std::fs::File::open(path)
            .await
            .map_err(|_| LoadError::File)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;
        let settings: Self = serde_json::from_str(&contents).map_err(|_| LoadError::Format)?;
        Ok(settings)
    }

    pub async fn save(self) -> Result<PathBuf, SaveError> {
        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::Format)?;
        let path = settings_path();
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
