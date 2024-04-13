use std::fmt::Debug;
use std::path::PathBuf;

use iced::widget::{button, column, container, text};
use iced::{alignment::Horizontal, keyboard, Command as Cm, Element, Length, Subscription};
use ytmrs::{Ytmrs, YtmrsMsg};

mod background_canvas;
mod cache_handlers;
mod response_types;
mod settings;
mod song;
mod song_operations;
mod thumbnails;
mod ytmrs;

use crate::{
    response_types::IDKey,
    settings::{LoadError, SaveError, YTMRSettings},
};

#[derive(Default, Debug)]
enum Main {
    #[default]
    Loading,
    Loaded {
        state: Ytmrs,
        saving: bool,
    },
}

#[derive(Debug, Clone)]
enum YTMRSMessage {
    Loaded(Result<YTMRSettings, LoadError>),
    Save,
    Saved(Result<PathBuf, SaveError>),
    MainMessage(YtmrsMsg),
}

impl Main {
    fn load() -> Cm<YTMRSMessage> {
        Cm::perform(YTMRSettings::load_default(), YTMRSMessage::Loaded)
    }

    fn subscription(&self) -> Subscription<YTMRSMessage> {
        Subscription::batch([
            keyboard::on_key_press(|key_code, modifiers| {
                println!["{:#?} {:#?}", key_code, modifiers];
                if !(modifiers.command()) {
                    return None;
                }
                Self::handle_hotkey(key_code, modifiers)
            }),
            keyboard::on_key_release(|kcode, modifiers| {
                println!["{:#?} {:#?}", kcode, modifiers];

                None
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
                YTMRSMessage::Loaded(o) => match o {
                    Ok(state) => {
                        let mut main = Ytmrs::new(state);
                        let commands = main.load();
                        *self = Self::Loaded {
                            state: main,
                            saving: false,
                        };
                        commands.map(YTMRSMessage::MainMessage)
                    }
                    Err(_) => {
                        *self = Self::Loaded {
                            state: Ytmrs::default(),
                            saving: false,
                        };
                        Cm::none()
                    }
                },
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
    iced::program("A cool song list", Main::update, Main::view)
        .load(Main::load)
        .subscription(Main::subscription)
        .antialiasing(true)
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
