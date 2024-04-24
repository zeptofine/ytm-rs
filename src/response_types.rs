use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

fn r(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub type UrlString = String;
pub type IDKey = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub height: u16,
    pub width: u16,
    pub url: UrlString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTabEntry {
    pub id: IDKey,
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
    pub id: IDKey,
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
    pub id: IDKey,
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
    pub duration: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tags: Vec<String>,
}
impl YTSong {
    // Creates a basic Youtube Song for testing purposes
    #![allow(unused)] // It's used for test funcs
    pub fn basic() -> Self {
        Self {
            id: r(11),
            title: r(14),
            description: None,
            channel: r(10),
            view_count: Some(99_999),
            thumbnail: "https://placehold.co/960x720".to_string(),
            album: None,
            webpage_url: "...".to_string(),
            duration: 120.0,
            artists: Some(
                ["Me!!".into()]
                    .into_iter()
                    .cycle()
                    .take(thread_rng().gen_range(1..=3))
                    .collect(),
            ),
            tags: ["Tag".into()]
                .into_iter()
                .cycle()
                .take(thread_rng().gen_range(0..=5))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YTIEKey {
    Youtube,
    YoutubeTab,
    YoutubeMusicSearchURL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YTSearchEntry {
    pub id: IDKey,
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
