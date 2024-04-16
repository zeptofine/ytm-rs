use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use iced::Color;
use material_colors::color::Argb;
use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Serialize,
};
use uuid::Uuid;

use crate::{
    styling::{argb_to_color, color_to_argb},
    IDKey,
};

// use once_cell::sync::Lazy;
// use serde::{Deserializer, Serializer};

fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

pub trait YtmCache {
    fn ensure_thumbnail(&mut self) -> PathBuf;
    fn ensure_song(&mut self) -> PathBuf;
    fn get_color(&self) -> Option<Color>;
    fn set_color(&mut self, color: Color);
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CacheHandleItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail_path: Option<PathBuf>, // thumbnail path
    #[serde(skip_serializing_if = "Option::is_none")]
    song_path: Option<PathBuf>, // song path
    #[serde(skip_serializing_if = "Option::is_none")]
    primary_color: Option<String>,
}

impl CacheHandleItem {
    fn get_thumbnail(&self, source: &Path) -> Option<PathBuf> {
        let mut pth = source.to_path_buf();
        pth.push(self.thumbnail_path.clone()?);
        pth.set_extension("jpg");
        Some(pth)
    }
    fn get_song(&self, source: &Path) -> Option<PathBuf> {
        let mut pth = source.to_path_buf();
        pth.push(self.song_path.clone()?);
        pth.set_extension("mp3");
        Some(pth)
    }
}

impl YtmCache for CacheHandleItem {
    fn ensure_thumbnail(&mut self) -> PathBuf {
        if self.thumbnail_path.is_none() {
            self.thumbnail_path = Some(PathBuf::from(generate_id()));
        }

        self.thumbnail_path.clone().unwrap()
    }

    fn ensure_song(&mut self) -> PathBuf {
        if self.song_path.is_none() {
            self.song_path = Some(PathBuf::from(generate_id()));
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

#[derive(Debug, Clone)]
struct CacheMapper(HashMap<IDKey, CacheHandleItem>);

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
