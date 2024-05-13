use std::{collections::HashMap, path::PathBuf};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use serde::{Deserialize, Serialize};

use iced::{
    widget::{column, text, Column},
    Length,
};

use crate::{response_types::UrlString, settings::SongKey};

fn r(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Song {
    pub id: SongKey,
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_path: Option<PathBuf>,
}

impl Song {
    // Creates a basic Youtube Song for testing purposes
    #![allow(unused)] // It's used for test funcs
    pub fn basic() -> Self {
        Self {
            id: r(11),
            title: r(14),
            description: None,
            channel: r(10),
            view_count: Some(thread_rng().gen_range(0..10_000_000)),
            thumbnail: "https://placehold.co/960x720".to_string(),
            album: None,
            webpage_url: "...".to_string(),
            duration: thread_rng().gen_range(0.0..(12.0 * 60.0 * 60.0)),
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
            thumbnail_path: None,
            song_path: None,
        }
    }

    pub fn as_data(&self) -> SongData {
        SongData {
            title: self.title.clone(),
            channel: self.channel.clone(),
            artists: self.artists.clone(),
            duration: self.duration,
        }
    }
}

pub fn to_hash_map(songs: impl Iterator<Item = Song>) -> HashMap<String, Song> {
    songs.map(|s| (s.id.clone(), s)).collect()
}

pub struct SongData {
    pub title: String,
    pub channel: String,
    pub artists: Option<Vec<String>>,
    pub duration: f32,
}
impl SongData {
    /// Used for placeholders of songs that are not cached yet
    pub fn mystery() -> Self {
        Self {
            title: "?????".to_string(),
            channel: "???".to_string(),
            artists: None,
            duration: -1.0,
        }
    }

    fn format_duration(&self) -> String {
        let minutes = self.duration / 60.0;
        let hours = minutes / 60.0;
        let seconds = (self.duration % 60.0).floor() as u8;
        match hours.floor() == 0.0 {
            true => format!("{}:{:0>2}", minutes.floor(), seconds),
            false => format!("{}:{:0>2}:{:0>2}", hours.floor(), minutes.floor(), seconds,),
        }
    }

    fn format_artists(&self) -> String {
        match &self.artists {
            None => self.channel.clone(),
            Some(v) => v.join(" & "),
        }
    }

    pub fn column<'a, M: 'a>(self) -> Column<'a, M, iced::Theme, iced::Renderer> {
        column![
            text(self.title.clone()),
            text(self.format_duration()),
            text(self.format_artists()),
        ]
        .spacing(1)
        .padding(5)
        .width(Length::Fill)
    }
}
