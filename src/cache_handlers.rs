use std::{collections::HashMap, path::PathBuf};

use once_cell::sync::Lazy;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Serialize,
};
use serde::{ser::SerializeMap, Deserializer, Serializer};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_path: Option<PathBuf>, // thumbnail path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_colors: Option<PathBuf>, // generated thumbnail material colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_path: Option<PathBuf>, // song path
}

impl CacheHandleItem {
    fn new() -> Self {
        Self {
            thumbnail_path: None,
            thumbnail_colors: None,
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
        if !self.map.0.contains_key(key) {
            self.map.0.insert(key.to_string(), CacheHandleItem::new());
        }
        CacheHandle {
            source: self.source.clone(),
            item: self.map.0.get_mut(key).unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
struct CacheMapper(HashMap<String, CacheHandleItem>);

impl CacheMapper {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

struct CacheVisitor;

impl<'de> Visitor<'de> for CacheVisitor {
    type Value = CacheMapper;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A cache mapping")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map: HashMap<String, CacheHandleItem> = HashMap::new();
        while let Some((key, value)) = access.next_entry::<String, CacheHandleItem>()? {
            map.insert(key, value);
        }

        Ok(CacheMapper(map))
    }
}

impl<'de> Deserialize<'de> for CacheMapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CacheVisitor)
    }
}

impl Serialize for CacheMapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_map(Some(self.0.len()))?;

        for (key, value) in self.0.clone() {
            seq.serialize_entry(&key, &value)?;
        }

        seq.end()
    }
}
