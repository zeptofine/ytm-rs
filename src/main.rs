#![allow(unused_imports)]

// use std::time::Duration;

use std::fmt::Debug;

use iced::{
    alignment::{Alignment, Horizontal, Vertical},
    keyboard,
    widget::{
        button, checkbox, column, container, keyed_column, progress_bar, radio, row, scrollable,
        slider, text, text_input, Column, Text,
    },
    Command as Cm, Element, Length, Subscription,
};
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
};
// use iced_aw::{color_picker, number_input};
// use kira::tween::Tween;
use once_cell::sync::Lazy;
use reqwest::Url;
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
    m: AudioManager,
    current_data: Option<StaticSoundData>,
    current_handle: Option<StaticSoundHandle>,
}

impl Debug for YTMRSAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YTMRSAudioManager")
    }
}

impl Default for YTMRSAudioManager {
    fn default() -> Self {
        Self {
            m: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .expect("Failed to create backend"),
            current_data: None,
            current_handle: None,
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
        Cm::none()
    }

    fn subscription(&self) -> Subscription<MainMsg> {
        Subscription::none()
    }

    fn view(&self) -> Element<MainMsg> {
        let input = self.inputs.view();

        let songs: Element<_> = column(self.settings.queue.iter().enumerate().map(|(_, song)| {
            self.settings
                .saved_songs
                .get(song)
                .unwrap()
                .view()
                .map(move |message| MainMsg::SongMessage(song.to_owned(), message))
        }))
        .into();
        column![
            input.map(MainMsg::InputMessage),
            button("Generate").on_press(MainMsg::SearchUrl),
            scrollable(
                container(songs)
                    .width(Length::Fill)
                    .padding(40)
                    .align_x(Horizontal::Center)
            ),
        ]
        .push_maybe(match &self.audio_manager.current_handle {
            None => None,
            Some(h) => Some(progress_bar(
                0.0..=match &self.audio_manager.current_data {
                    None => 100.0,
                    Some(d) => d.duration().as_secs_f32(),
                },
                h.position() as f32,
            )),
        })
        .push(row![
            text("Volume:"),
            slider(
                0.0..=1000.0,
                self.settings.volume * 1000.0,
                MainMsg::VolumeChanged
            )
        ])
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
                    self.add_song(song)
                        .map(move |msg| MainMsg::SongMessage(id.clone(), msg))
                }
                YTResponseType::Tab(t) => {
                    println!["Request is a 'tab'"];
                    Cm::none()
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
        }
    }

    fn add_song(&mut self, s: YTSong) -> Cm<SongMessage> {
        let id = s.id.clone();
        let mut handle = self.settings.index.get(&id);
        if !self.settings.saved_songs.contains_key(&id) {
            self.settings
                .saved_songs
                .insert(id.clone(), Song::new(s, &mut handle));
        }
        self.settings.queue.push(id.clone());
        self.settings
            .saved_songs
            .get(&id)
            .unwrap()
            .load(&mut handle)
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
                    let commands = main.load().map(YTMRSMessage::MainMessage);
                    *self = Self::Loaded {
                        state: main,
                        saving: false,
                    };
                    commands
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

    // let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
    //     .expect("Failed to create backend");
    // let sound_data = StaticSoundData::from_file("Alain.wav", StaticSoundSettings::default())
    //     .expect("Failed to read file");
    // let handle = manager
    //     .play(sound_data.clone())
    //     .expect("Failed to play song");

    iced::program("A cool song list", YTMRS::update, YTMRS::view)
        .load(YTMRS::load)
        .subscription(YTMRS::subscription)
        .run()
}
