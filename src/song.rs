use iced::{
    alignment::Vertical,
    widget::{button, column, row, text, Image},
    Command as Cm, Element, Length,
};

use serde::{Deserialize, Serialize};

use crate::{
    cache_handlers::{CacheHandle, YtmCache as _},
    response_types::YTSong,
    thumbnails::{get_thumbnail, ThumbnailState},
};

// use chrono::Duration;
// use std::path::PathBuf;
// use iced::{
//     alignment::Horizontal,
//     widget::{image::Handle, vertical_space, Space},
//     Alignment,
// };
// use crate::response_types::UrlString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    #[serde(skip)]
    pub thumbnail: ThumbnailState,

    pub data: YTSong,
}

impl Song {
    pub fn new(song: YTSong) -> Self {
        Self {
            thumbnail: ThumbnailState::Unknown,
            data: song,
        }
    }

    pub fn load(&self, handle: &mut CacheHandle) -> Cm<SongMessage> {
        Cm::batch([match &self.thumbnail {
            ThumbnailState::Unknown => Cm::perform(
                get_thumbnail(self.data.thumbnail.clone(), handle.ensure_thumbnail()),
                |r| match r {
                    Err(_) => SongMessage::ThumnailGatherFailure,
                    Ok(state) => SongMessage::ThumbnailGathered(state),
                },
            ),
            _ => Cm::none(),
        }])
    }

    pub fn view(&self) -> Element<SongMessage> {
        let thumbnail: Element<SongMessage> =
            if let ThumbnailState::Downloaded { path: _, handle } = &self.thumbnail {
                Image::new(handle.clone())
                    .height(100)
                    .width(100)
                    .content_fit(iced::ContentFit::Cover)
                    .into()
            } else {
                text("<...>")
                    .height(100)
                    .width(100)
                    .vertical_alignment(Vertical::Center)
                    .into()
            };
        button(row![
            column![thumbnail],
            column![
                text(&self.data.title),
                {
                    let hours = self.data.duration / 60.0 / 60.0;
                    let minutes = self.data.duration / 60.0 % 60.0;
                    let seconds = self.data.duration % 60.0;
                    text(format!(
                        "{}:{:0>2}:{:0>2.2}",
                        hours.floor(),
                        minutes.floor(),
                        seconds.floor(),
                    ))
                },
                text(match &self.data.artists {
                    None => self.data.channel.clone(),
                    Some(v) => v.join(" & "),
                })
            ]
            .spacing(1)
            .padding(5)
            .width(Length::Fill),
        ])
        .padding(0)
        .on_press(SongMessage::Clicked)
        .into()
    }

    pub fn update(&mut self, msg: SongMessage) -> Cm<SongMessage> {
        match msg {
            SongMessage::Clicked => {
                println!["Song was clicked"];
                Cm::none()
            }
            SongMessage::ThumbnailGathered(state) => {
                self.thumbnail = state;
                Cm::none()
            }
            SongMessage::ThumnailGatherFailure => {
                println!["Failed to gather thumbnail"];
                Cm::none()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SongMessage {
    Clicked,
    ThumbnailGathered(ThumbnailState),
    ThumnailGatherFailure,
}
