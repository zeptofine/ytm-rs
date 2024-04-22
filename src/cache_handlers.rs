use std::{collections::HashMap, path::PathBuf};

use iced::Color;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod cache_handle;
mod cache_mapper;

use cache_handle::CacheHandleItem;
use cache_mapper::CacheMapper;

pub fn generate_cache_id() -> String {
    Uuid::new_v4().to_string()
}

pub trait YtmCache {
    fn ensure_thumbnail(&mut self) -> PathBuf;
    fn ensure_song(&mut self) -> PathBuf;
    fn get_color(&self) -> Option<Color>;
    fn set_color(&mut self, color: Color);
}

pub struct CacheHandle<'a> {
    source: PathBuf,
    item: &'a mut CacheHandleItem,
}

impl YtmCache for CacheHandle<'_> {
    fn ensure_thumbnail(&mut self) -> PathBuf {
        self.item.ensure_thumbnail();
        self.item.get_thumbnail(&self.source).unwrap()
    }

    fn ensure_song(&mut self) -> PathBuf {
        todo!()
    }

    fn get_color(&self) -> Option<Color> {
        self.item.get_color()
    }

    fn set_color(&mut self, color: Color) {
        self.item.set_color(color)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheHandler {
    source: PathBuf,
    map: CacheMapper,
}

impl CacheHandler {
    pub fn new(folder: PathBuf) -> Self {
        if !folder.exists() {
            std::fs::create_dir_all(&folder).unwrap();
        }
        Self {
            source: folder,
            map: CacheMapper::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> CacheHandle {
        let item = self.map.0.entry(key.to_string()).or_default();
        CacheHandle {
            source: self.source.clone(),
            item,
        }
    }
    pub fn validate_paths(&self) -> Option<Self> {
        let unfinished: HashMap<String, CacheHandleItem> = self
            .map
            .0
            .iter()
            .filter_map(|(k, item)| {
                if item.thumbnail_path.is_none() && item.song_path.is_none() {
                    return None;
                }

                let exists_thumb = item.get_thumbnail(&self.source).filter(|p| p.exists());
                let exists_song = item.get_song(&self.source).filter(|p| p.exists());
                match (item.thumbnail_path.is_some() == exists_thumb.is_some())
                    && (item.song_path.is_some() == exists_song.is_some())
                {
                    true => None,
                    false => Some((
                        k.clone(),
                        CacheHandleItem {
                            thumbnail_path: item.thumbnail_path.clone().and(exists_thumb),
                            song_path: item.song_path.clone().and(exists_song),
                            primary_color: item.primary_color.clone(),
                        },
                    )),
                }
            })
            .collect();

        if unfinished.is_empty() {
            None
        } else {
            println!["Fixing {} cache items", unfinished.len()];
            println!["{:#?}", unfinished];
            let mut new_map = self.map.clone();
            new_map.0.extend(unfinished);
            Some(Self {
                source: self.source.clone(),
                map: new_map,
            })
        }
    }
}
