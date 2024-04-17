use std::{collections::HashMap, env, path::PathBuf};

use async_std::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{cache_handlers::CacheHandler, chunked_list::PagedSongList, song::Song};

pub type SongID = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMRUserSettings {
    pub volume: f32,
}

impl Default for YTMRUserSettings {
    fn default() -> Self {
        Self { volume: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMRSettings {
    pub saved_songs: HashMap<SongID, Song>,
    pub index: CacheHandler,
    pub queue: Vec<SongID>,
    pub user_settings: YTMRUserSettings,
}

impl YTMRSettings {
    fn validate(self) -> Self {
        Self {
            saved_songs: self.saved_songs,
            index: match self.index.validate_paths() {
                Some(new_idx) => new_idx,
                None => self.index,
            },
            queue: self.queue,
            user_settings: self.user_settings,
        }
    }
}

impl Default for YTMRSettings {
    fn default() -> Self {
        let index = CacheHandler::new({
            let mut dir = env::current_dir().unwrap();
            dir.push("cache");
            dir
        });

        Self {
            saved_songs: HashMap::new(),
            index,
            queue: vec![],
            user_settings: YTMRUserSettings::default(),
        }
    }
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
    pub fn path() -> PathBuf {
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

    pub async fn load_default() -> Result<YTMRSettings, LoadError> {
        Self::load(Self::path()).await
    }

    pub async fn load(path: PathBuf) -> Result<YTMRSettings, LoadError> {
        let mut contents = String::new();
        println!["Reading: {path:?}"];
        let mut file = async_std::fs::File::open(path)
            .await
            .map_err(|_| LoadError::File)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::File)?;
        let settings: YTMRSettings =
            serde_json::from_str(&contents).map_err(|_| LoadError::Format)?;
        Ok(settings.validate())
    }

    pub async fn save(self) -> Result<PathBuf, SaveError> {
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
