use iced::{
    alignment::Vertical,
    widget::{button, column, row, text, Image, Row},
    Command as Cm, Element, Length,
};

use serde::{Deserialize, Serialize};

use crate::{
    cache_handlers::{CacheHandle, YtmCache as _},
    response_types::YTSong,
    styling::{update_song_button, SongAppearance},
    thumbnails::{get_thumbnail, ThumbnailState},
};

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

    fn get_img<Msg>(&self) -> Element<Msg> {
        match &self.thumbnail {
            ThumbnailState::Downloaded { path: _, handle } => Image::new(handle.clone())
                .height(100)
                .width(100)
                .content_fit(iced::ContentFit::Cover)
                .into(),
            _ => text("<...>")
                .height(100)
                .width(100)
                .vertical_alignment(Vertical::Center)
                .into(),
        }
    }

    fn format_duration(&self) -> String {
        let minutes = self.data.duration / 60.0;
        let hours = minutes / 60.0;
        let seconds = (self.data.duration % 60.0).floor() as u8;
        match hours.floor() == 0.0 {
            true => format!("{}:{:0>2}", minutes.floor(), seconds),
            false => format!("{}:{:0>2}:{:0>2}", hours.floor(), minutes.floor(), seconds,),
        }
    }

    fn format_artists(&self) -> String {
        match &self.data.artists {
            None => self.data.channel.clone(),
            Some(v) => v.join(" & "),
        }
    }

    fn img_and_data<'a, M: 'a>(&'a self) -> Row<'a, M, iced::Theme, iced::Renderer> {
        row![
            self.get_img(),
            column![
                text(&self.data.title),
                text(self.format_duration()),
                text(self.format_artists())
            ]
            .spacing(1)
            .padding(5)
            .width(Length::Fill),
        ]
    }

    pub fn view(&self, appearance: &SongAppearance) -> Element<SongMessage> {
        let song_appearance = appearance.0;
        button::Button::new(self.img_and_data())
            .style(move |_theme, status| update_song_button(song_appearance, status))
            .padding(0)
            .on_press(SongMessage::Clicked)
            .into()
    }

    pub fn view_closable(&self, appearance: &SongAppearance) -> Element<ClosableSongMessage> {
        let song_appearance = appearance.0;
        self.img_and_data()
            .push(
                button("x")
                    .on_press(ClosableSongMessage::Closed)
                    .style(move |_t, status| update_song_button(song_appearance, status)),
            )
            .into()
    }

    pub fn update(&mut self, msg: SongMessage) -> Cm<SongMessage> {
        if let SongMessage::ThumbnailGathered(state) = msg {
            self.thumbnail = state;
        };

        Cm::none()
    }
}

#[derive(Debug, Clone)]
pub enum SongMessage {
    Clicked,
    ThumbnailGathered(ThumbnailState),
    ThumnailGatherFailure,
}

#[derive(Debug, Clone)]
pub enum ClosableSongMessage {
    Closed,
}
