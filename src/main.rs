#![allow(unused_imports)]

// use std::time::Duration;

use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

// use iced_aw::{color_picker, number_input};
use iced::{
    alignment::{Alignment, Horizontal, Vertical},
    keyboard,
    widget::{
        button, checkbox, column, container, keyed_column, progress_bar, radio, row, scrollable,
        slider, text, text_input, Column, Text,
    },
    Command as Cm, Element, Length, Subscription,
};
use once_cell::sync::Lazy;
use reqwest::Url;
use rodio::{
    source::{SineWave, Source},
    OutputStreamHandle,
};
use rodio::{Decoder, OutputStream, Sink};
use serde::Serialize;

mod cache_handlers;
mod response_types;
mod settings;
mod song;
use response_types::{YTResponseError, YTResponseType, YTSong};
use settings::{LoadError, SaveError, YTMRSettings};
use song::{Song, SongMessage};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Default)]
struct UserInputs {
    url: String,
}

#[derive(Debug, Clone)]
enum InputMessage {
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
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
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
            stream,
            handle,
            sink,
        }
    }
}

#[derive(Debug, Default)]
struct Main {
    inputs: UserInputs,
    audio_manager: YTMRSAudioManager,
    settings: YTMRSettings,
}

#[derive(Debug, Clone)]
enum MainMsg {
    SongMessage(String, SongMessage),
    InputMessage(InputMessage),
    SearchUrl,

    RequestRecieved(RequestResult),
    RequestParsed(YTResponseType),
    RequestParseFailure(YTResponseError),
    AddSong(YTSong),
    AddSavedSong(String),
    VolumeChanged(f32),
}

#[derive(Debug, Clone)]
enum RequestResult {
    Success(String),
    RequestError,
    JsonParseError,
}

impl Main {
    fn new(settings: YTMRSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    fn load(&mut self) -> Cm<MainMsg> {
        let mut commands = vec![];
        for key in self.settings.queue.clone() {
            commands.push(self.update(MainMsg::AddSavedSong(key)));
        }

        Cm::batch(commands)
    }

    fn prepare_to_save(&mut self) {}

    fn subscription(&self) -> Subscription<MainMsg> {
        Subscription::none()
    }

    fn view(&self) -> Element<MainMsg> {
        let input = self.inputs.view();
        let songs: Element<_> = column(self.settings.queue.iter().map(|song| {
            self.settings.saved_songs[song]
                .view()
                .map(move |message| MainMsg::SongMessage(song.clone(), message))
        }))
        .into();
        column![
            input.map(MainMsg::InputMessage),
            button("Generate").on_press(MainMsg::SearchUrl),
            scrollable(
                container(songs)
                    .width(Length::Fill)
                    .padding(10)
                    .align_x(Horizontal::Center)
            ),
        ]
        // .push(row![slider(
        //     0.0..=1000.0,
        //     self.settings.volume * 1000.0,
        //     MainMsg::VolumeChanged
        // )
        // .height(20)])
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    fn update(&mut self, message: MainMsg) -> Cm<MainMsg> {
        match message {
            MainMsg::SongMessage(key, msg) => self
                .settings
                .saved_songs
                .get_mut(&key)
                .unwrap()
                .update(msg)
                .map(move |msg| MainMsg::SongMessage(key.clone(), msg)),
            MainMsg::InputMessage(i) => self.inputs.update(i).map(MainMsg::InputMessage),
            MainMsg::VolumeChanged(v) => {
                self.settings.volume = v / 1000.0;
                self.audio_manager.sink.set_volume(self.settings.volume);
                Cm::none()
            }

            MainMsg::SearchUrl => {
                // Check if URL is valid
                match Url::parse(&self.inputs.url) {
                    Ok(_) => Cm::perform(
                        Main::request_info(self.inputs.url.clone()),
                        MainMsg::RequestRecieved,
                    ),

                    Err(e) => {
                        println!["Failed to parse: {e}"];
                        Cm::none()
                    }
                }
            }
            MainMsg::RequestRecieved(response) => match response {
                RequestResult::Success(s) => {
                    Cm::perform(Main::parse_request(s), |result| match result {
                        Ok(r) => MainMsg::RequestParsed(r),
                        Err(e) => MainMsg::RequestParseFailure(e),
                    })
                }
                _ => {
                    println!["{:?}", response];
                    Cm::none()
                }
            },
            MainMsg::RequestParsed(response_type) => match response_type {
                YTResponseType::Song(song) => {
                    println!["Request is a song"];
                    let id = song.id.clone();
                    self.add_ytsong(song)
                        .map(move |msg| MainMsg::SongMessage(id.clone(), msg))
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
                            .map(move |msg| MainMsg::SongMessage(id.clone(), msg))
                    }))
                }
                YTResponseType::Search(s) => {
                    println!["Request is a search"];
                    Cm::none()
                }
            },
            MainMsg::RequestParseFailure(e) => {
                println!["{:?}", e];
                Cm::none()
            }
            MainMsg::AddSong(s) => {
                let id = s.id.clone();
                self.add_ytsong(s)
                    .map(move |msg| MainMsg::SongMessage(id.clone(), msg))
            }
            MainMsg::AddSavedSong(id) => {
                let song = self.settings.saved_songs.get(&id);
                self.settings.queue.push(id.clone());
                match song {
                    Some(s) => s
                        .load(&mut self.settings.index.get(&id))
                        .map(move |msg| MainMsg::SongMessage(id.clone(), msg)),
                    None => Cm::none(),
                }
            }
        }
    }

    fn add_ytsong(&mut self, song: YTSong) -> Cm<SongMessage> {
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

#[derive(Default, Debug)]
enum YTMRS {
    #[default]
    Loading,
    Loaded {
        state: Main,
        saving: bool,
    },
}

#[derive(Debug, Clone)]
enum YTMRSMessage {
    Loaded(Result<YTMRSettings, LoadError>),
    Save,
    Saved(Result<std::path::PathBuf, SaveError>),
    MainMessage(MainMsg),
}

impl YTMRS {
    fn load() -> Cm<YTMRSMessage> {
        Cm::perform(YTMRSettings::load(), |s| {
            println!["Loaded: {s:?}"];
            YTMRSMessage::Loaded(s)
        })
    }

    fn subscription(&self) -> Subscription<YTMRSMessage> {
        Subscription::batch([
            keyboard::on_key_press(|key_code, modifiers| {
                if !(modifiers.command()) {
                    return None;
                }
                Self::handle_hotkey(key_code, modifiers)
            }),
            match self {
                Self::Loaded { state, saving: _ } => {
                    state.subscription().map(YTMRSMessage::MainMessage)
                }
                _ => Subscription::none(),
            },
        ])
    }

    fn handle_hotkey(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Option<YTMRSMessage> {
        if key == keyboard::Key::Character("s".into()) && modifiers.command() {
            Some(YTMRSMessage::Save)
        } else {
            println!["{key:?} {modifiers:?}"];
            None
        }
    }

    fn update(&mut self, message: YTMRSMessage) -> Cm<YTMRSMessage> {
        match self {
            Self::Loading => match message {
                YTMRSMessage::Loaded(Ok(state)) => {
                    let mut main = Main::new(state);
                    let commands = main.load();
                    *self = Self::Loaded {
                        state: main,
                        saving: false,
                    };
                    commands.map(YTMRSMessage::MainMessage)
                }
                YTMRSMessage::Loaded(Err(_)) => {
                    *self = Self::Loaded {
                        state: Main::default(),
                        saving: false,
                    };
                    Cm::none()
                }
                _ => Cm::none(),
            },
            Self::Loaded { state, saving: _ } => match message {
                YTMRSMessage::MainMessage(m) => state.update(m).map(YTMRSMessage::MainMessage),
                YTMRSMessage::Save => {
                    println!["Saving"];
                    state.prepare_to_save();
                    Cm::perform(state.settings.clone().save(), YTMRSMessage::Saved)
                }
                YTMRSMessage::Saved(success) => {
                    match success {
                        Ok(p) => println!["Saved to {p:?}"],
                        Err(e) => println!["{e:?}"],
                    }
                    Cm::none()
                }
                _ => Cm::none(),
            },
        }
    }

    fn view(&self) -> Element<YTMRSMessage> {
        match self {
            Self::Loading => container(
                text("Loading...")
                    .horizontal_alignment(Horizontal::Center)
                    .size(5),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .into(),
            Self::Loaded { state, saving } => container(column![
                button(if *saving { "saving..." } else { "save" }).on_press(YTMRSMessage::Save),
                state.view().map(YTMRSMessage::MainMessage)
            ])
            .into(),
        }
    }
}

pub fn main() -> iced::Result {
    // let response =
    //     reqwest::blocking::get("http:/localhost:55001").expect("Failed to get a response");

    iced::program("A cool song list", YTMRS::update, YTMRS::view)
        .load(YTMRS::load)
        .subscription(YTMRS::subscription)
        .run()
}

// pub fn main() {
//     let (_stream, stream_handle) = OutputStream::try_default().unwrap();
//     let sink = Sink::try_new(&stream_handle).unwrap();

//     let file = BufReader::new(File::open("Alain.wav").unwrap());
//     let source = Decoder::new(file).unwrap();
//     let duration = source.total_duration().unwrap();

//     sink.append(source);

//     std::thread::sleep(duration);
// }
