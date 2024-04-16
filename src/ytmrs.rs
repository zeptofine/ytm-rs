use std::fmt::Debug;

use iced::widget::{button, column, container, row, scrollable, text_input};
use iced::{
    alignment::{Alignment, Horizontal},
    Command as Cm, Element, Length, Subscription,
};
use once_cell::sync::Lazy;
use reqwest::Url;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use serde::Serialize;

use crate::{
    cache_handlers::YtmCache,
    response_types::{YTResponseError, YTResponseType, YTSong},
    settings::YTMRSettings,
    song::{Song, SongMessage},
    styling::{color_to_argb, BasicYtmrsScheme, FullYtmrsScheme},
};

// use iced_aw::{color_picker, number_input};
// use iced::{
//     alignment::Vertical,
//     widget::{checkbox, keyed_column, progress_bar, radio, row, slider, Column, Text},
// };
// use rodio::{source::Source, Decoder};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Default)]
struct UserInputs {
    url: String,
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    UrlChanged(String),
}

impl UserInputs {
    fn view(&self) -> Element<InputMessage> {
        column![text_input("Youtube URL...", &self.url)
            .id(INPUT_ID.clone())
            .on_input(InputMessage::UrlChanged)
            .size(20)
            .padding(15),]
        .into()
    }

    fn update(&mut self, message: InputMessage) -> Cm<InputMessage> {
        match message {
            InputMessage::UrlChanged(s) => self.url = s,
        }
        Cm::none()
    }
}

struct YTMRSAudioManager {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    _sink: Sink,
}

impl Debug for YTMRSAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YTMRSAudioManager")
    }
}

impl Default for YTMRSAudioManager {
    fn default() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        Self {
            _stream: stream,
            _handle: handle,
            _sink: sink,
        }
    }
}

#[derive(Debug, Default)]
pub struct Ytmrs {
    inputs: UserInputs,
    // audio_manager: YTMRSAudioManager,
    pub settings: YTMRSettings,
}

#[derive(Debug, Clone)]
pub enum YtmrsMsg {
    SongMessage(String, SongMessage),
    InputMessage(InputMessage),
    SearchUrl,

    RequestRecieved(RequestResult),
    RequestParsed(YTResponseType),
    RequestParseFailure(YTResponseError),

    SetNewBackground(String, BasicYtmrsScheme),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Success(String),
    RequestError,
    JsonParseError,
}

impl Ytmrs {
    pub fn new(settings: YTMRSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn load(&mut self) -> Cm<YtmrsMsg> {
        let mut commands = vec![];
        for key in self.settings.queue.clone() {
            if let Some(song) = self.settings.saved_songs.get(&key) {
                commands.push(
                    song.load(&mut self.settings.index.get(&key))
                        .map(move |msg| YtmrsMsg::SongMessage(key.clone(), msg)),
                )
            }
        }

        Cm::batch(commands)
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::none()
    }

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let input = self.inputs.view();
        let songs: Element<_> = column(self.settings.queue.iter().map(|song| {
            self.settings.saved_songs[song]
                .view(*scheme.song_appearance.clone())
                .map(move |message| YtmrsMsg::SongMessage(song.clone(), message))
        }))
        .padding(0)
        .into();

        column![
            input.map(YtmrsMsg::InputMessage),
            button("Generate").on_press(YtmrsMsg::SearchUrl),
            row![
                scrollable(
                    container(songs)
                        .width(Length::Fill)
                        .max_width(400)
                        .padding(0)
                        .align_x(Horizontal::Left)
                ),
                scrollable(self.inputs.view().map(YtmrsMsg::InputMessage),)
                    .width(Length::FillPortion(1)),
            ]
        ]
        .align_items(Alignment::Start)
        .spacing(20)
        .padding(10)
        .into()
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Cm<YtmrsMsg> {
        match message {
            YtmrsMsg::SongMessage(key, msg) => {
                let song = self.settings.saved_songs.get_mut(&key).unwrap();
                match msg {
                    SongMessage::Clicked => {
                        Cm::batch([
                            // Change background color to indicate the playing song
                            match song.thumbnail.clone() {
                                crate::thumbnails::ThumbnailState::Unknown => Cm::none(),
                                crate::thumbnails::ThumbnailState::Downloaded {
                                    path,
                                    handle: _,
                                } => {
                                    let handle = self.settings.index.get(&key);
                                    match &handle.get_color() {
                                        Some(col) => Cm::perform(
                                            BasicYtmrsScheme::from_argb(color_to_argb(*col)),
                                            |scheme| YtmrsMsg::SetNewBackground(key, scheme),
                                        ),
                                        None => Cm::perform(
                                            BasicYtmrsScheme::from_image(path),
                                            |scheme| YtmrsMsg::SetNewBackground(key, scheme),
                                        ),
                                    }
                                }
                            },
                        ])
                    }
                    _ => song
                        .update(msg)
                        .map(move |msg| YtmrsMsg::SongMessage(key.clone(), msg)),
                }
            }
            YtmrsMsg::SetNewBackground(key, scheme) => {
                // Save primary color to cache for future use
                let mut handle = self.settings.index.get(&key);
                if handle.get_color().is_none() {
                    handle.set_color(scheme.primary_color);
                    println!["Saved primary color: {:?}", scheme.primary_color];
                }
                Cm::none()
            }
            YtmrsMsg::InputMessage(i) => self.inputs.update(i).map(YtmrsMsg::InputMessage),
            YtmrsMsg::SearchUrl => {
                // Check if URL is valid
                match Url::parse(&self.inputs.url) {
                    Ok(_) => Cm::perform(
                        Ytmrs::request_info(self.inputs.url.clone()),
                        YtmrsMsg::RequestRecieved,
                    ),

                    Err(e) => {
                        println!["Failed to parse: {e}"];
                        Cm::none()
                    }
                }
            }
            YtmrsMsg::RequestRecieved(response) => match response {
                RequestResult::Success(s) => {
                    Cm::perform(Ytmrs::parse_request(s), |result| match result {
                        Ok(r) => YtmrsMsg::RequestParsed(r),
                        Err(e) => YtmrsMsg::RequestParseFailure(e),
                    })
                }
                _ => {
                    println!["{:?}", response];
                    Cm::none()
                }
            },
            YtmrsMsg::RequestParsed(response_type) => match response_type {
                YTResponseType::Song(song) => {
                    println!["Request is a song"];
                    let id = song.id.clone();
                    self.add_ytsong(song)
                        .map(move |msg| YtmrsMsg::SongMessage(id.clone(), msg))
                }
                YTResponseType::Tab(t) => {
                    println!["Request is a 'tab'"];
                    self.settings.queue.clear();

                    Cm::batch(t.entries.iter().map(|entry| {
                        let id = entry.id.clone();
                        let song = YTSong {
                            id: entry.id.clone(),
                            title: entry.title.clone(),
                            description: None,
                            channel: entry.channel.clone(),
                            view_count: entry.view_count,
                            thumbnail: entry.thumbnails[0].url.clone(),
                            album: None,
                            webpage_url: entry.url.clone(),
                            duration: entry.duration,
                            artists: Some(vec![entry.channel.clone()]),
                            tags: vec![],
                        };
                        self.add_ytsong(song)
                            .map(move |msg| YtmrsMsg::SongMessage(id.clone(), msg))
                    }))
                }
                YTResponseType::Search(_s) => {
                    println!["Request is a search"];
                    Cm::none()
                }
            },
            YtmrsMsg::RequestParseFailure(e) => {
                println!["{:?}", e];
                Cm::none()
            }
        }
    }

    pub fn add_ytsong(&mut self, song: YTSong) -> Cm<SongMessage> {
        let s = Song::new(song.clone());
        let id = s.data.id.clone();
        if !self.settings.saved_songs.contains_key(&id) {
            self.settings.saved_songs.insert(id.clone(), s);
        }
        self.settings.queue.push(id.clone());
        self.settings
            .saved_songs
            .get(&id)
            .unwrap()
            .load(&mut self.settings.index.get(&id))
    }

    async fn parse_request(response: String) -> Result<YTResponseType, YTResponseError> {
        YTResponseType::new(response)
    }

    async fn request_info(url: String) -> RequestResult {
        println!["Requesting info for {}", url];
        let client = reqwest::Client::new();

        match client
            .post("http://127.0.0.1:55001/request_info")
            .json(&RequestInfoDict {
                url,
                process: false,
            })
            .send()
            .await
        {
            Err(e) => {
                println!["{e:?}"];
                RequestResult::RequestError
            }
            Ok(r) => match r.text().await {
                Err(_) => RequestResult::JsonParseError,
                Ok(j) => RequestResult::Success(j),
            },
        }
    }
}

#[derive(Serialize)]
struct RequestInfoDict {
    url: String,
    process: bool,
}
