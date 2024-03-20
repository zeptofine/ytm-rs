use iced::alignment::{self, Alignment};
use iced::widget::{
    button, checkbox, column, container, keyed_column, radio, row, scrollable, text, text_input,
    Column, Text,
};
use iced::{Command, Element, Font, Length, Subscription};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Song {
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
    fn new(
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

    fn view(&self) -> Element<SongMessage> {
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

#[derive(Default)]
struct Main {
    title_value: String,
    artist_name: String,
    duration: usize,
    url: String,
    youtube_id: String,

    songs: Vec<Song>,
}

#[derive(Debug, Clone)]
enum Message {
    CreateSong,
    SongMessage(SongMessage),
    TitleChanged(String),
    ArtistChanged(String),
    DurationChanged(usize),
    UrlChanged(String),
    IdChanged(String),
}

impl Main {
    fn view(&self) -> Element<Message> {
        let input = column![
            text_input("Song title", &self.title_value)
                .id(INPUT_ID.clone())
                .on_input(Message::TitleChanged)
                .on_submit(Message::CreateSong)
                .padding(15)
                .size(30),
            text_input("Artist name", &self.artist_name)
                .id(INPUT_ID.clone())
                .on_input(Message::ArtistChanged)
                .on_submit(Message::CreateSong)
                .padding(15)
                .size(30),
            text_input("Url", &self.url)
                .id(INPUT_ID.clone())
                .on_input(Message::UrlChanged)
                .on_submit(Message::CreateSong)
                .padding(15)
                .size(30),
            text_input("Youtube ID", &self.youtube_id)
                .id(INPUT_ID.clone())
                .on_input(Message::IdChanged)
                .on_submit(Message::CreateSong)
                .padding(15)
                .size(30)
        ];

        let songs: Element<_> = column(self.songs.iter().map(|song| {
            song.view()
                .map(move |message| Message::SongMessage(message))
        }))
        .into();
        column![
            input,
            scrollable(
                container(songs)
                    .width(Length::Fill)
                    .padding(40)
                    .align_x(alignment::Horizontal::Center)
            )
        ]
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CreateSong => {
                if !self.title_value.is_empty()
                    && !self.artist_name.is_empty()
                    && !self.url.is_empty()
                    && !self.youtube_id.is_empty()
                {
                    println!("Creatednew song!");
                    self.songs.push(Song::new(
                        self.title_value.clone(),
                        self.artist_name.clone(),
                        self.duration.clone(),
                        self.url.clone(),
                        self.youtube_id.clone(),
                    ))
                }
                Command::none()
            }
            Message::UrlChanged(s) => {
                self.url = s;
                Command::none()
            }
            Message::TitleChanged(s) => {
                self.title_value = s;
                Command::none()
            }
            Message::ArtistChanged(s) => {
                self.artist_name = s;
                Command::none()
            }
            Message::DurationChanged(v) => {
                self.duration = v;
                Command::none()
            }
            Message::IdChanged(s) => {
                self.youtube_id = s;
                Command::none()
            }
            Message::SongMessage(_) => {
                println!["Song was clicked"];
                Command::none()
            }
        }
    }
}

pub fn main() -> iced::Result {
    iced::run("A cool song list", Main::update, Main::view)
}
