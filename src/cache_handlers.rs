use std::{collections::HashMap, path::PathBuf};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

pub trait YtmCache {
    fn get_thumbnail_path(&mut self) -> PathBuf;

    fn get_song_path(&mut self) -> PathBuf;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CacheHandleItem {
    id: String,
    thumbnail_path: Option<PathBuf>,
    song_path: Option<PathBuf>,
}

impl CacheHandleItem {
    fn new(id: String) -> Self {
        Self {
            id,
            thumbnail_path: None,
            song_path: None,
        }
    }
}

impl YtmCache for CacheHandleItem {
    fn get_thumbnail_path(&mut self) -> PathBuf {
        if None == self.thumbnail_path {
            self.thumbnail_path = Some(PathBuf::from(generate_id()));
        }

        self.thumbnail_path.clone().unwrap()
    }

    fn get_song_path(&mut self) -> PathBuf {
        if None == self.song_path {
            self.song_path = Some(PathBuf::from(generate_id()));
        }
        self.song_path.clone().unwrap()
    }
}
pub struct CacheHandle<'a> {
    source: PathBuf,
    item: &'a mut CacheHandleItem,
}

impl YtmCache for CacheHandle<'_> {
    fn get_thumbnail_path(&mut self) -> PathBuf {
        let mut pth = self.source.clone();
        pth.push(self.item.get_thumbnail_path());
        pth.set_extension("jpg");
        pth
    }

    fn get_song_path(&mut self) -> PathBuf {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheHandler {
    source: PathBuf,
    map: HashMap<String, CacheHandleItem>,
}

impl CacheHandler {
    pub fn new(folder: PathBuf) -> Self {
        if !folder.exists() {
            std::fs::create_dir_all(&folder).unwrap();
        }
        Self {
            source: folder,
            map: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> CacheHandle {
        if !self.map.contains_key(key) {
            let s = key.to_string();
            self.map.insert(s.clone(), CacheHandleItem::new(s));
        }
        CacheHandle {
            source: self.source.clone(),
            item: self.map.get_mut(key).unwrap(),
        }
    }
}
