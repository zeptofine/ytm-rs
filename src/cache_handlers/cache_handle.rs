use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use iced::Color;
use material_colors::color::Argb;
use serde::{Deserialize, Serialize};

use crate::styling::{argb_to_color, color_to_argb};

use super::{generate_cache_id, YtmCache};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CacheHandleItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_path: Option<PathBuf>, // thumbnail path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_path: Option<PathBuf>, // song path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
}

impl CacheHandleItem {
    pub fn get_thumbnail(&self, source: &Path) -> Option<PathBuf> {
        let mut pth = source.to_path_buf();
        pth.push(self.thumbnail_path.clone()?);
        pth.set_extension("jpg");
        Some(pth)
    }
    pub fn get_song(&self, source: &Path) -> Option<PathBuf> {
        let mut pth = source.to_path_buf();
        pth.push(self.song_path.clone()?);
        pth.set_extension("mp3");
        Some(pth)
    }
}

impl YtmCache for CacheHandleItem {
    fn ensure_thumbnail(&mut self) -> PathBuf {
        if self.thumbnail_path.is_none() {
            self.thumbnail_path = Some(PathBuf::from(generate_cache_id()));
        }

        self.thumbnail_path.clone().unwrap()
    }

    fn ensure_song(&mut self) -> PathBuf {
        if self.song_path.is_none() {
            self.song_path = Some(PathBuf::from(generate_cache_id()));
        }
        self.song_path.clone().unwrap()
    }

    fn get_color(&self) -> Option<Color> {
        self.primary_color
            .clone()
            .map(|argb| argb_to_color(Argb::from_str(&argb).unwrap()))
    }

    fn set_color(&mut self, color: Color) {
        self.primary_color = Some(color_to_argb(color).to_hex());
    }
}
