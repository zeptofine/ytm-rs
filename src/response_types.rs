use std::borrow::Borrow;

use serde::{self, Deserialize, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thumbnail {
    pub height: u16,
    pub width: u16,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTabEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub duration: usize,
    pub view_count: usize,
    pub channel: String,
    pub channel_url: String,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTab {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub view_count: usize,
    pub thumbnails: Vec<Thumbnail>,
    pub availability: Option<String>,
    pub webpage_url: String,
    pub modified_date: String,
    pub entries: Vec<YTabEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSong {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub view_count: usize,
    pub thumbnail: String,
    pub album: String,
    pub webpage_url: String,
    pub duration: usize,
    pub like_count: usize,
    pub artists: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YTIEKey {
    Youtube,
    YoutubeTab,
    YoutubeMusicSearchURL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSearchEntry {
    pub title: Option<String>,
    pub id: String,
    pub ie_key: YTIEKey,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMSearch {
    pub title: String,
    pub webpage_url: String,
    pub entries: Vec<YTSearchEntry>,
}

#[derive(Debug, Clone)]
pub enum YTResponseError {
    ExtractionErr,
    ParseErr,
}

impl From<serde_json::Error> for YTResponseError {
    fn from(e: serde_json::Error) -> Self {
        println!["{e:?}"];
        Self::ParseErr
    }
}

#[derive(Debug, Clone)]
pub enum YTResponseType {
    Tab(YTab),
    Search(YTMSearch),
    Song(YTSong),
}

#[derive(Deserialize)]
struct ExtractorKey {
    extractor_key: String,
}

impl YTResponseType {
    pub fn new(response: String) -> Result<Self, YTResponseError> {
        let extractor: ExtractorKey = serde_json::from_str(&response)?;
        let key = extractor.extractor_key.borrow();

        match key {
            "Youtube" => Ok(YTResponseType::Song(serde_json::from_str(&response)?)),
            "YoutubeTab" => Ok(YTResponseType::Tab(serde_json::from_str(&response)?)),
            "YoutubeMusicSearchURL" => Ok(YTResponseType::Search(serde_json::from_str(&response)?)),
            _ => {
                println!["Unrecognized key: {key}"];
                Err(YTResponseError::ExtractionErr)
            }
        }
    }
}
