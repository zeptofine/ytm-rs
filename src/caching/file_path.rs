use serde::{Deserialize, Serialize};

use super::IDed;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePathPair {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<String>,
}
impl IDed for FilePathPair {
    fn id(&self) -> &str {
        &self.id
    }
}
