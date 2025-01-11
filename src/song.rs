use rand::{distributions::Alphanumeric, thread_rng, Rng};

use serde::{Deserialize, Serialize};

use iced::{
    advanced::image as iced_image,
    alignment::{Horizontal, Vertical},
    widget::{self, button, column, container, hover, row, text, Image, Row, Text},
    Background, Border, Color, Element, Length, Shadow, Vector,
};

#[cfg(feature = "svg")]
use iced::widget::Svg;

#[cfg(feature = "svg")]
use crate::audio::PLAY_SVG;

use crate::{
    caching::IDed,
    response_types::{RequestedDownload, UrlString},
    settings::SongKey,
};

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
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_downloads: Option<Vec<RequestedDownload>>,
    #[serde(skip)]
    pub ui_state: SongState,
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
            requested_downloads: None,
            ui_state: SongState::default(),
        }
    }

    pub fn as_data(&self) -> SongData {
        SongData {
            title: self.title.clone(),
            channel: self.channel.clone(),
            artists: self.artists.clone(),
            duration: self.duration,
            state: self.ui_state.clone(),
        }
    }
}
impl AsRef<Song> for Song {
    #[inline]
    fn as_ref(&self) -> &Song {
        self
    }
}
impl IDed<String> for Song {
    #[inline]
    fn id(&self) -> &String {
        &self.id
    }
}

pub fn format_duration(d: &f32) -> String {
    let minutes = d / 60.0;
    let hours = minutes / 60.0;
    let seconds = (d % 60.0).floor() as u8;
    match hours.floor() == 0.0 {
        true => format!("{}:{:0>2}", minutes.floor(), seconds),
        false => format!("{}:{:0>2}:{:0>2}", hours.floor(), minutes.floor(), seconds,),
    }
}

#[derive(Debug, Clone)]
pub enum SongMessage {
    ThumbnailClicked,
}
#[derive(Clone)]
pub struct SongData {
    pub title: String,
    pub channel: String,
    pub artists: Option<Vec<String>>,
    pub duration: f64,
    pub state: SongState,
}
impl SongData {
    /// Used for placeholders of songs that are not cached yet
    pub fn mystery() -> Self {
        Self {
            title: "?????".to_string(),
            channel: "???".to_string(),
            artists: None,
            duration: -1.0,
            state: SongState::default(),
        }
    }

    pub fn mystery_with_title(title: String) -> Self {
        Self {
            title,
            ..Self::mystery()
        }
    }

    fn format_artists(&self) -> String {
        match &self.artists {
            None => self.channel.clone(),
            Some(v) => v.join(" & "),
        }
    }

    fn img(h: Option<iced_image::Handle>, x: u16, y: u16) -> Option<Image<iced_image::Handle>> {
        h.map(|h| {
            Image::new(h)
                .width(x)
                .height(y)
                .content_fit(iced::ContentFit::Cover)
        })
    }

    fn placeholder<'a>(width: u16, height: u16) -> Text<'a> {
        text("???")
            .height(height)
            .width(width)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
    }

    /// Creates the thumbnail of the song data, replacing with "???" text upon missing data.
    fn image_or_placeholder<'a, M>(
        h: Option<iced_image::Handle>,
        width: u16,
        height: u16,
    ) -> Element<'a, M> {
        match Self::img(h, width, height) {
            Some(img) => Element::new(img),
            None => Element::new(Self::placeholder(width, height)),
        }
    }

    pub fn row<'a>(self, clickable: bool, hover_play_button: bool) -> Row<'a, SongMessage> {
        let row = row![
            {
                let play_button = {
                    #[cfg(feature = "svg")]
                    let x = Svg::new(PLAY_SVG.clone());

                    #[cfg(not(feature = "svg"))]
                    let x = text("|>");
                    x
                };
                container(play_button)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(|_| widget::container::Style {
                    background: Some(Background::Color(Color::new(0., 0., 0., 0.75))),
                    text_color: Some(Color::WHITE),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.,
                        radius: 0.into(),
                    },
                    shadow: Shadow {
                        color: Color::BLACK,
                        offset: Vector::ZERO,
                        blur_radius: 0.,
                    },
                })
                .width(80)
                .height(80)
            },
            column![text(format!(
                "{}\n{}\n{}",
                self.title.clone(),
                format_duration(&(self.duration as f32)),
                self.format_artists()
            )),]
            .width(Length::Fill),
        ];
        row.spacing(8).padding(0)
    }
}

#[derive(Debug, Clone, Default)]
pub enum SongState {
    #[default]
    None,

    Fetching,
    Downloading,
    Downloaded,

    Cached,
    Playing,
}
