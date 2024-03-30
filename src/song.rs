use std::path::PathBuf;

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, image as icyimg, row, text, vertical_space, Image},
    Alignment, Command as Cm, Element, Length,
};

use image::GenericImageView;
use serde::{Deserialize, Serialize};

use crate::cache_handlers::{CacheHandle, YtmCache as _};
use crate::response_types::{UrlString, YTSong};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ThumbnailState {
    #[default]
    Unknown,

    NotDownloaded,
    Downloaded(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    #[serde(skip)]
    pub thumbnail_state: ThumbnailState,

    #[serde(skip)]
    pub thumbnail_handle: Option<icyimg::Handle>,

    pub data: YTSong,
}

impl Song {
    pub fn new(song: YTSong) -> Self {
        Self {
            thumbnail_state: ThumbnailState::Unknown,
            thumbnail_handle: None,
            data: song,
        }
    }

    pub fn load(&self, handle: &mut CacheHandle) -> Cm<SongMessage> {
        let thumbnail_path = handle.get_thumbnail_path();
        Cm::batch([match &self.thumbnail_state {
            ThumbnailState::NotDownloaded | ThumbnailState::Unknown => Cm::perform(
                Song::get_thumbnail(self.data.thumbnail.clone(), thumbnail_path),
                SongMessage::ThumbnailGathered,
            ),

            ThumbnailState::Downloaded(s) => {
                Cm::perform(async { thumbnail_path }, SongMessage::ThumbnailGathered)
            }
        }])
    }

    pub async fn get_thumbnail(thumbnail_url: String, output: PathBuf) -> PathBuf {
        let imgbytes = reqwest::get(thumbnail_url)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let mut thumbnail = image::load_from_memory(&imgbytes).unwrap();
        let (w, h) = thumbnail.dimensions();
        // crop it to a square
        let smaller = h.min(w);
        let left = (w - smaller) / 2;
        let top = (h - smaller) / 2;

        thumbnail = thumbnail.crop(left, top, smaller, smaller);
        match thumbnail.save(&output) {
            Ok(_) => {}
            Err(e) => println!["Failed to save thumbnail: {}", e],
        };
        output
    }

    pub fn view(&self) -> Element<SongMessage> {
        let thumbnail: Element<SongMessage> = match &self.thumbnail_handle {
            None => text("<...>")
                .height(100)
                .width(100)
                .vertical_alignment(Vertical::Center)
                .into(),
            Some(h) => Image::new(h.clone()).height(100).into(),
        };
        button(row![
            column![thumbnail].padding(1),
            column![
                text(&self.data.title),
                text(&self.data.duration),
                text(match &self.data.artists {
                    None => self.data.channel.clone(),
                    Some(v) => v.join(" & "),
                })
            ]
            .width(Length::Fill),
        ])
        .on_press(SongMessage::Clicked)
        .into()
    }

    pub fn update(&mut self, msg: SongMessage) -> Cm<SongMessage> {
        match msg {
            SongMessage::Clicked => {
                println!["Song was clicked"];
                Cm::none()
            }
            SongMessage::ThumbnailGathered(pth) => {
                self.thumbnail_state = ThumbnailState::Downloaded(pth.clone());
                self.thumbnail_handle = Some(icyimg::Handle::from_path(pth));
                println!["Thumbnail gathered"];
                Cm::none()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SongMessage {
    Clicked,
    ThumbnailGathered(PathBuf),
}
