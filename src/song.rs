use iced::{
    alignment::Vertical,
    widget::{column, container, row, text, Column, Image, Row},
    Command as Cm, Element, Length,
};

use serde::{Deserialize, Serialize};

use crate::{
    cache_handlers::{CacheHandle, YtmCache as _},
    response_types::YTSong,
    styling::SongStyle,
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

    pub fn get_img<Msg>(&self, height: u16, width: u16) -> Element<Msg> {
        match &self.thumbnail {
            ThumbnailState::Downloaded { path: _, handle } => Image::new(handle.clone())
                .height(height)
                .width(width)
                .content_fit(iced::ContentFit::Cover)
                .into(),
            _ => text("<...>")
                .height(height)
                .width(width)
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

    pub fn get_data<'a, M: 'a>(&'a self) -> Column<'a, M, iced::Theme, iced::Renderer> {
        column![
            text(&self.data.title),
            text(self.format_duration()),
            text(self.format_artists()),
        ]
        .spacing(1)
        .padding(5)
        .width(Length::Fill)
    }

    fn img_and_data<'a, M: 'a>(
        &'a self,
        width: u16,
        height: u16,
    ) -> Row<'a, M, iced::Theme, iced::Renderer> {
        row![self.get_img(height, width), self.get_data(),]
    }

    pub fn view(&self, appearance: &SongStyle) -> Element<SongMessage> {
        let song_appearance = appearance.0;
        container(self.img_and_data(100, 100))
            .style(move |_| song_appearance)
            .padding(0)
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
    ThumbnailGathered(ThumbnailState),
    ThumnailGatherFailure,
}

#[derive(Debug, Clone)]
pub enum ClosableSongMessage {
    Closed,
}
