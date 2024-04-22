use std::collections::HashMap;

use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Serialize,
};

use crate::{cache_handlers::CacheHandleItem, IDKey};

#[derive(Debug, Clone)]
pub struct CacheMapper(pub HashMap<IDKey, CacheHandleItem>);

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
