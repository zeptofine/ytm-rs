use iced::alignment::Alignment;
use iced::widget::{
    button, checkbox, column, container, keyed_column, radio, row, scrollable, text, text_input,
    Column, Text,
};
use iced::{Command, Element, Font, Length, Subscription};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    id: Uuid,
    title: String,
    artist: String,
    duration: usize,
    url: String,
    youtube_id: String,

    #[serde(skip)]
    state: SongState,
}

impl Song {
    pub fn new(
        title: String,
        artist: String,
        duration: usize,
        url: String,
        youtube_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            artist,
            duration,
            url,
            youtube_id,
            state: SongState::NotDownloaded,
        }
    }

    pub fn view(&self) -> Element<SongMessage> {
        button(row![
            text("Song Thumbnail"),
            column![text(&self.title), text(&self.duration), text(&self.artist)]
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
