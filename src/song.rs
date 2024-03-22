use std::default;

use iced::widget::{button, column, row, text};
use iced::{Element, Length};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SongData {
    pub title: String,
    pub artist: String,
    pub duration: usize,
    pub album: String,

    #[serde(with = "dtfmt")]
    pub release_date: NaiveDateTime,
}

impl Default for SongData {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            duration: 0,
            album: String::new(),
            release_date: NaiveDateTime::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    id: Uuid,
    data: SongData,
    url: String,
    youtube_id: String,

    #[serde(skip)]
    state: SongState,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            data: SongData::default(),
            url: String::new(),
            youtube_id: String::new(),
            state: SongState::NotDownloaded,
        }
    }
}

impl Song {
    pub fn new(
        title: String,
        artist: String,
        duration: usize,
        url: String,
        youtube_id: String,
        release_date: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: SongData {
                title,
                artist,
                duration,
                release_date: release_date.unwrap_or_default(),
                ..SongData::default()
            },
            url,
            youtube_id,
            state: SongState::NotDownloaded,
        }
    }

    pub fn view(&self) -> Element<SongMessage> {
        button(row![
            text("Song Thumbnail"),
            column![
                text(&self.data.title),
                text(&self.data.duration),
                text(&self.data.artist)
            ]
            .width(Length::Fill),
        ])
        .on_press(SongMessage::Clicked)
        .into()
    }
}

#[derive(Debug, Clone, Default)]
pub enum SongState {
    #[default]
    NotDownloaded,
    Downloaded,
    Downloading {
        progress: usize,
        total: usize,
    },
}

#[derive(Debug, Clone)]
pub enum SongMessage {
    Clicked,
}

mod dtfmt {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(dt)
    }
}
