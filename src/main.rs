use std::fmt::Debug;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use iced::Color;
use iced::{alignment::Horizontal, Command as Cm, Element, Length, Subscription};
use iced::{
    theme::{Palette, Theme},
    widget::{button, column, container, text},
};
use ytmrs::{Ytmrs, YtmrsMsg};

mod cache_handlers;
mod chunked_list;
mod response_types;
mod settings;
mod song;
mod song_operations;
mod styling;
mod thumbnails;
mod ytmrs;

use crate::{
    response_types::IDKey,
    settings::{LoadError, SaveError, YTMRSettings},
    styling::{transition_scheme, SchemeState},
};

pub const BACKGROUND_TRANSITION_DURATION: Duration = Duration::from_millis(500);
pub const BACKGROUND_TRANSITION_RATE: Duration = Duration::from_millis(1000 / 20); // ~15fps

#[derive(Debug)]
struct MainState {
    ytmrs: Ytmrs,
    saving: bool,
    state: SchemeState,
}

#[derive(Default, Debug)]
struct Main {
    state: Option<MainState>,
}

// The largest enum variant is MainMessage,
// but it is by far the most common, so it does not warrant
// a Box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum YTMRSMessage {
    Loaded(Result<YTMRSettings, LoadError>),
    Save,
    Saved(Result<PathBuf, SaveError>),
    MainMessage(YtmrsMsg),

    UpdateVisibleBackground(SchemeState),
}

impl Main {
    fn load() -> Cm<YTMRSMessage> {
        Cm::perform(YTMRSettings::load_default(), YTMRSMessage::Loaded)
    }

    fn theme(&self) -> Theme {
        match &self.state {
            Some(state) if state.saving => Theme::default(),
            None => Theme::default(),
            Some(state) => {
                let (primary, danger) = {
                    let choice = state.state.first_choice();
                    (choice.colors.primary_color, choice.colors.error_color)
                };

                Theme::custom(
                    "Hell".to_string(),
                    Palette {
                        background: Color::BLACK,
                        text: Color::WHITE,
                        primary,
                        success: Color::TRANSPARENT,
                        danger,
                    },
                )
            }
        }
    }

    fn subscription(&self) -> Subscription<YTMRSMessage> {
        Subscription::none()
    }

    fn update(&mut self, message: YTMRSMessage) -> Cm<YTMRSMessage> {
        match &mut self.state {
            None => match message {
                YTMRSMessage::Loaded(o) => {
                    let (s, coms) = match o {
                        Ok(settings) => {
                            let mut main = Ytmrs::new(settings);
                            let commands = main.load().map(YTMRSMessage::MainMessage);
                            (main, commands)
                        }
                        Err(_) => (Ytmrs::default(), Cm::none()),
                    };

                    self.state = Some(MainState {
                        ytmrs: s,
                        saving: false,
                        state: SchemeState::default(),
                    });
                    coms
                }
                _ => Cm::none(),
            },
            Some(ref mut state) => match message {
                YTMRSMessage::UpdateVisibleBackground(scheme_state) => {
                    match scheme_state {
                        SchemeState::Started(_) => todo!(), // how
                        SchemeState::Transitioning(_) => {
                            state.state = scheme_state.clone();

                            Cm::perform(
                                transition_scheme(scheme_state),
                                |state: SchemeState| -> YTMRSMessage {
                                    YTMRSMessage::UpdateVisibleBackground(state)
                                },
                            )
                        }
                        SchemeState::Finished(_) => {
                            state.state = *Box::new(scheme_state);
                            Cm::none()
                        }
                    }
                }

                YTMRSMessage::MainMessage(YtmrsMsg::SetNewBackground(k, scheme)) => {
                    let schemestate = SchemeState::Started(Box::new(styling::Started {
                        from: state.state.first_choice().clone(),
                        to: scheme.clone(),
                        started: SystemTime::now(),
                    }));
                    state.state = schemestate.clone();
                    Cm::batch([
                        Cm::perform(
                            transition_scheme(schemestate),
                            YTMRSMessage::UpdateVisibleBackground,
                        ),
                        state
                            .ytmrs
                            .update(YtmrsMsg::SetNewBackground(k, scheme))
                            .map(YTMRSMessage::MainMessage),
                    ])
                    // Cm::perform(, |state| {
                    //     YTMRSMessage::UpdateVisibleBackground(state)
                    // })
                }
                YTMRSMessage::MainMessage(msg) => {
                    state.ytmrs.update(msg).map(YTMRSMessage::MainMessage)
                }
                YTMRSMessage::Save => {
                    state.ytmrs.prepare_to_save();
                    Cm::perform(state.ytmrs.settings.clone().save(), YTMRSMessage::Saved)
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
        match &self.state {
            None => container(
                text("Loading...")
                    .horizontal_alignment(Horizontal::Center)
                    .size(5),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .into(),
            Some(state) => container(column![
                button(if state.saving { "saving..." } else { "save" })
                    .on_press(YTMRSMessage::Save),
                state
                    .ytmrs
                    .view(state.state.first_choice().clone())
                    .map(YTMRSMessage::MainMessage)
            ])
            .style(|_| container::Style {
                background: Some(state.state.first_choice().colors.to_background()),
                ..Default::default()
            })
            .into(),
        }
    }
}

pub fn main() -> iced::Result {
    iced::program("A cool song list", Main::update, Main::view)
        .load(Main::load)
        .theme(Main::theme)
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
