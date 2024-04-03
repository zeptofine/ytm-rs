use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

pub type UrlString = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thumbnail {
    pub height: u16,
    pub width: u16,
    pub url: UrlString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTabEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub duration: f64,
    pub view_count: Option<usize>,
    pub channel: String,
    pub channel_url: String,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTab {
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub view_count: Option<usize>,
    pub thumbnails: Option<Vec<Thumbnail>>,
    pub availability: Option<String>,
    pub webpage_url: String,
    pub modified_date: Option<String>,
    pub entries: Vec<YTabEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSong {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_count: Option<usize>,
    pub thumbnail: UrlString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    pub webpage_url: UrlString,
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
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
    pub url: UrlString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMSearch {
    pub title: String,
    pub webpage_url: UrlString,
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
struct ExtractorKey(String);

impl YTResponseType {
    pub fn new(response: String) -> Result<Self, YTResponseError> {
        let extractor: ExtractorKey = serde_json::from_str(&response)?;
        let key = extractor.0.borrow();
        println!["{key}"];

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
