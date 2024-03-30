use std::borrow::Borrow;

use serde::{self, Deserialize, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thumbnail {
    height: u16,
    width: u16,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTabEntry {
    id: String,
    url: String,
    title: String,
    description: Option<String>,
    duration: usize,
    view_count: usize,
    channel: String,
    channel_url: String,
    thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTab {
    id: String,
    title: String,
    view_count: usize,
    availability: Option<String>,
    channel: String,
    webpage_url: String,
    modified_date: String,
    entries: Vec<YTabEntry>,
    thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSong {
    channel: String,
    artists: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YTIEKey {
    Youtube,
    YoutubeTab,
    YoutubeMusicSearchURL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSearchEntry {
    title: Option<String>,
    id: String,
    ie_key: YTIEKey,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTMSearch {
    title: String,
    webpage_url: String,
    entries: Vec<YTSearchEntry>,
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

#[derive(Serialize, Deserialize)]
pub struct ExtractorKey {
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
