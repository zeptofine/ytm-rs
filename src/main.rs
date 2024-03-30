// use std::time::Duration;

use std::fmt::Debug;

use iced::alignment::{Alignment, Horizontal, Vertical};
use iced::keyboard;
use iced::widget::{
    button, checkbox, column, container, keyed_column, radio, row, scrollable, slider, text,
    text_input, Column, Text,
};
use iced::Command as Cm;
use iced::{Command, Element, Length, Subscription};
// use iced_aw::{color_picker, number_input};
// use kira::tween::Tween;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};
use once_cell::sync::Lazy;
use reqwest::{Response, Url};
use response_types::{YTResponseError, YTResponseType, YTSong};
use serde::Serialize;
use settings::{LoadError, SaveError, YTMRSettings};

mod response_types;
mod settings;
mod song;
mod thumbnails;
use song::{Song, SongData, SongMessage};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
#[derive(Debug, Default)]
struct UserInputs {
    url: String,
    audio_path: String,
}

#[derive(Debug, Clone)]
enum InputMessage {
    UrlChanged(String),
    AudioPathChanged(String),
}

impl UserInputs {
    fn view(&self) -> Element<InputMessage> {
        column![
            text_input("Youtube URL...", &self.url)
                .id(INPUT_ID.clone())
                .on_input(InputMessage::UrlChanged)
                .size(20)
                .padding(15),
            text_input("Audio path...", &self.audio_path)
                .id(INPUT_ID.clone())
                .on_input(InputMessage::AudioPathChanged)
                .size(20)
                .padding(15),
        ]
        .into()
    }

    fn update(&mut self, message: InputMessage) -> Cm<InputMessage> {
        match message {
            InputMessage::UrlChanged(s) => self.url = s,
            InputMessage::AudioPathChanged(s) => self.audio_path = s,
        }
        Cm::none()
    }
}

struct YTMRSAudioManager {
    m: AudioManager,
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
    SongMessage(SongMessage),
    InputMessage(InputMessage),
    SearchUrl,

    AddSong(YTSong),

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

    fn subscription(&self) -> Subscription<MainMsg> {
        Subscription::none()
    }

    fn view(&self) -> Element<MainMsg> {
        let input = self.inputs.view();

        let songs: Element<_> = column(self.settings.queue.iter().map(|song| {
            self.settings
                .saved_songs
                .get(song)
                .unwrap()
                .view()
                .map(move |message| MainMsg::SongMessage(message))
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
            slider(
                0.0..=1000.0,
                self.settings.volume * 1000.0,
                MainMsg::VolumeChanged
            )
        ]
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    fn update(&mut self, message: MainMsg) -> Cm<MainMsg> {
        match message {
            MainMsg::SongMessage(_) => {
                println!["Song was clicked"];
                Cm::none()
            }
            MainMsg::InputMessage(i) => self.inputs.update(i).map(MainMsg::InputMessage),
            MainMsg::VolumeChanged(v) => {
                self.settings.volume = v / 1000.0;
                println!["{}", self.settings.volume];
                Cm::none()
            }

            MainMsg::SearchUrl => {
                // Check if URL is valid
                match Url::parse(&self.inputs.url) {
                    Ok(u) => Cm::perform(
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
                YTResponseType::Song(s) => {
                    println!["Request is a song"];
                    self.add_song(s);
                    Cm::none()
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
            MainMsg::AddSong(s) => {
                self.add_song(s);
                Cm::none()
            }
        }
    }

    fn add_song(&mut self, s: YTSong) {
        let id = s.id.clone();
        if !self.settings.saved_songs.contains_key(&id) {
            self.settings.saved_songs.insert(id.clone(), Song::new(s));
        }
        self.settings.queue.push(id);
    }

    async fn parse_request(response: String) -> Result<YTResponseType, YTResponseError> {
        YTResponseType::new(response)
    }

    async fn request_info(url: String) -> RequestResult {
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
    fn load() -> Command<YTMRSMessage> {
        Command::perform(YTMRSettings::load(), |s| {
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

    fn update(&mut self, message: YTMRSMessage) -> Command<YTMRSMessage> {
        match self {
            Self::Loading => {
                match message {
                    YTMRSMessage::Loaded(Ok(state)) => {
                        *self = Self::Loaded {
                            state: Main::new(state),
                            saving: false,
                        };
                    }
                    YTMRSMessage::Loaded(Err(_)) => {
                        *self = Self::Loaded {
                            state: Main::default(),
                            saving: false,
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Self::Loaded { state, saving: _ } => match message {
                YTMRSMessage::MainMessage(m) => state.update(m).map(YTMRSMessage::MainMessage),
                YTMRSMessage::Save => {
                    println!["Saving"];
                    Command::perform(state.settings.clone().save(), YTMRSMessage::Saved)
                }
                YTMRSMessage::Saved(success) => {
                    match success {
                        Ok(p) => println!["Saved to {p:?}"],
                        Err(e) => println!["{e:?}"],
                    }
                    Command::none()
                }
                _ => Command::none(),
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
