use std::path::PathBuf;

use chrono::Duration;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, column, horizontal_space,
        image::{self as icyimg, Handle},
        row, text, vertical_space, Image, Space,
    },
    Alignment, Command as Cm, Element, Length,
};
use image::GenericImageView;
use serde::{Deserialize, Serialize};

use crate::{
    cache_handlers::{CacheHandle, YtmCache as _},
    response_types::{UrlString, YTSong},
    thumbnails::{get_thumbnail, ThumbnailState},
};

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
        let thumbnail_path = handle.get_thumbnail_path();
        Cm::batch([match &self.thumbnail {
            ThumbnailState::Unknown => Cm::perform(
                get_thumbnail(self.data.thumbnail.clone(), thumbnail_path),
                |r| match r {
                    Err(_) => SongMessage::ThumnailGatherFailure,
                    Ok((p, m)) => SongMessage::ThumbnailGathered(p, m),
                },
            ),

            ThumbnailState::Downloaded {
                path: _,
                handle: _,
                colors: _,
            } => Cm::perform(
                async {
                    let mut mp = thumbnail_path.clone();
                    mp.push("_mat");

                    (thumbnail_path, mp)
                },
                |(p, m)| SongMessage::ThumbnailGathered(p, m),
            ),
        }])
    }

    pub fn view(&self) -> Element<SongMessage> {
        let thumbnail: Element<SongMessage> = if let ThumbnailState::Downloaded {
            path: _,
            handle,
            colors: _,
        } = &self.thumbnail
        {
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
            SongMessage::ThumbnailGathered(pth, mat) => {
                let handle = icyimg::Handle::from_path(&pth);

                self.thumbnail = ThumbnailState::Downloaded {
                    path: pth,
                    handle: handle,
                    colors: None,
                };
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
    ThumbnailGathered(PathBuf, PathBuf),
    ThumnailGatherFailure,
}
