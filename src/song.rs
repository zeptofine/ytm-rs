use std::default;

use iced::widget::{button, column, row, text};
use iced::{Element, Length};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::response_types::YTSong;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThumbnailState {}

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
    pub id: Uuid,
    pub data: YTSong,
}

impl Song {
    pub fn new(song: YTSong) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: song,
        }
    }

    pub fn view(&self) -> Element<SongMessage> {
        button(row![
            text("Song Thumbnail"),
            column![
                text(&self.data.title),
                text(&self.data.duration),
                text(&self.data.artists.join(" & "))
            ]
            .width(Length::Fill),
        ])
        .on_press(SongMessage::Clicked)
        .into()
    }
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
