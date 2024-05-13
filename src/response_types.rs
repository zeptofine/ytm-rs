use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::{settings::SongKey, song::Song};

pub type UrlString = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub height: u16,
    pub width: u16,
    pub url: UrlString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTabEntry {
    pub id: SongKey,
    pub url: UrlString,
    pub title: String,
    pub description: Option<String>,
    pub duration: f32,
    pub view_count: Option<usize>,
    pub channel: String,
    pub channel_url: String,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTab {
    pub id: SongKey,
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
pub enum YTIEKey {
    Youtube,
    YoutubeTab,
    YoutubeMusicSearchURL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSearchEntry {
    pub id: SongKey,
    pub title: Option<String>,
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
    Song(Song),
}

#[derive(Deserialize)]
struct ExtractorKey {
    extractor_key: String,
}

impl YTResponseType {
    pub fn new(response: String) -> Result<Self, YTResponseError> {
        let extractor: ExtractorKey = serde_json::from_str(&response)?;
        let key = extractor.extractor_key.borrow();
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
