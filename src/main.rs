// #![windows_subsystem = "windows"]
// #![allow(dead_code)]

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::advanced::Application;
use iced::{alignment::Horizontal, Command as Cm, Element, Length, Subscription};
use iced::{executor, window, Color, Renderer, Settings, Size};
use iced::{
    theme::{Palette, Theme},
    widget::{button, column, container, text},
};

mod audio;
mod backend_handler;
mod caching;
mod playlist;
mod response_types;
mod search_window;
mod settings;
mod song;
mod song_list;
mod song_operations;
mod styling;
#[cfg(feature = "thumbnails")]
mod thumbnails;
mod user_input;
mod ytmrs;

use crate::{
    backend_handler::{BackendHandler, BackendLaunchStatus, ConnectionMode},
    settings::{LoadError, SaveError, YTMRSettings},
    styling::SchemeState,
    ytmrs::{Ytmrs, YtmrsMsg},
};

pub const BACKGROUND_TRANSITION_DURATION: Duration = Duration::from_millis(500);
pub const BACKGROUND_TRANSITION_RATE: Duration = Duration::from_millis(1000 / 20); // ~15fps

#[derive(Debug)]
struct MainState {
    ytmrs: Ytmrs,
    saving: bool,
    state: SchemeState,
}

#[derive(Debug)]
struct Main {
    backend: Arc<Mutex<BackendHandler>>,
    state: Option<MainState>,
}

// The largest enum variant is MainMessage,
// but it is by far the most common, so it does not warrant
// a Box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum MAINMessage {
    Loaded(Result<YTMRSettings, LoadError>),
    Save,
    Saved(Result<PathBuf, SaveError>),
    YtmrsMessage(YtmrsMsg),
    // UpdateVisibleBackground(SchemeState),
}

impl Main {}

impl Application for Main {
    type Executor = executor::Default;

    type Message = MAINMessage;

    type Theme = Theme;

    type Renderer = Renderer;

    type Flags = Arc<Mutex<BackendHandler>>;

    fn new(backend: Self::Flags) -> (Self, Cm<Self::Message>) {
        let me = Self {
            backend,
            state: None,
        };

        (
            me,
            Cm::perform(YTMRSettings::load_default(), MAINMessage::Loaded),
        )
    }

    fn title(&self) -> String {
        "A cool song list".to_string()
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

    fn update(&mut self, message: Self::Message) -> Cm<Self::Message> {
        match &mut self.state {
            None => match message {
                MAINMessage::Loaded(o) => {
                    let mut s = match o {
                        Ok(settings) => Ytmrs::new(settings, self.backend.clone()),
                        Err(_) => Ytmrs::default(),
                    };

                    let commands = s.load().map(MAINMessage::YtmrsMessage);

                    self.state = Some(MainState {
                        ytmrs: s,
                        saving: false,
                        state: SchemeState::default(),
                    });
                    commands
                }
                _ => Cm::none(),
            },
            Some(ref mut state) => match message {
                // MAINMessage::UpdateVisibleBackground(scheme_state) => {
                //     match scheme_state {
                //         SchemeState::Started(_) => todo!(), // how
                //         SchemeState::Transitioning(_) => {
                //             state.state = scheme_state.clone();

                //             Cm::perform(
                //                 transition_scheme(scheme_state),
                //                 |state: SchemeState| -> MAINMessage {
                //                     MAINMessage::UpdateVisibleBackground(state)
                //                 },
                //             )
                //         }
                //         SchemeState::Finished(_) => {
                //             state.state = *Box::new(scheme_state);
                //             Cm::none()
                //         }
                //     }
                // }
                // MAINMessage::YtmrsMessage(YtmrsMsg::SetNewBackground(k, scheme)) => {
                //     let schemestate = SchemeState::Started(Box::new(styling::Started {
                //         from: state.state.first_choice().clone(),
                //         to: scheme.clone(),
                //         started: SystemTime::now(),
                //     }));
                //     state.state = schemestate.clone();
                //     Cm::batch([
                //         Cm::perform(
                //             transition_scheme(schemestate),
                //             MAINMessage::UpdateVisibleBackground,
                //         ),
                //         state
                //             .ytmrs
                //             .update(YtmrsMsg::SetNewBackground(k, scheme))
                //             .map(MAINMessage::YtmrsMessage),
                //     ])
                //     // Cm::perform(, |state| {
                //     //     YTMRSMessage::UpdateVisibleBackground(state)
                //     // })
                // }
                MAINMessage::YtmrsMessage(msg) => {
                    state.ytmrs.update(msg).map(MAINMessage::YtmrsMessage)
                }
                MAINMessage::Save => {
                    state.ytmrs.prepare_to_save();
                    Cm::perform(state.ytmrs.settings.clone().save(), MAINMessage::Saved)
                }
                MAINMessage::Saved(success) => {
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

    fn subscription(&self) -> Subscription<MAINMessage> {
        match &self.state {
            Some(state) => state.ytmrs.subscription().map(MAINMessage::YtmrsMessage),
            None => Subscription::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        match &self.state {
            None => container(
                text("Loading...")
                    .horizontal_alignment(Horizontal::Center)
                    .size(50),
            )
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .center_y(Length::Fill)
            .into(),

            Some(state) => {
                let contents = {
                    let c = column![
                        button(if state.saving { "saving..." } else { "save" })
                            .on_press(MAINMessage::Save),
                        state
                            .ytmrs
                            .view(state.state.first_choice().clone())
                            .map(MAINMessage::YtmrsMessage),
                    ];

                    #[cfg(not(target_os = "macos"))]
                    {
                        c
                    }
                    #[cfg(target_os = "macos")]
                    {
                        c.align_items(iced::Alignment::End)
                    }
                };
                container(contents)
                    .style(|_| container::Style {
                        background: Some(state.state.first_choice().colors.to_background()),
                        ..Default::default()
                    })
                    .into()
            }
        }
    }
}

pub fn main() -> iced::Result {
    let backend = Arc::new(Mutex::new(BackendHandler::default()));

    let main = Main::run(Settings {
        id: None,
        flags: backend.clone(),
        antialiasing: true,
        window: window::Settings {
            size: Size::new(1024.0, 512.0),
            position: window::Position::Centered,
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: true,
            level: window::Level::Normal,
            icon: None,
            #[cfg(target_os = "macos")]
            platform_specific: window::settings::PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            #[cfg(target_os = "windows")]
            platform_specific: window::settings::PlatformSpecific {
                parent: None,
                drag_and_drop: true,
                skip_taskbar: false,
            },
            #[cfg(target_os = "linux")]
            platform_specific: window::settings::PlatformSpecific {
                application_id: "YtmRs".to_string(),
            },
            exit_on_close_request: true,
        },
        ..Default::default()
    });

    println!["App exited"];

    // If backend is owned by current process, try to kill it
    {
        let mut b = backend.lock().unwrap();
        if let BackendLaunchStatus::Launched(ConnectionMode::Child(process, _)) = &mut b.status {
            println!["Kill result: {:?}", process.kill()];
            println!["Killed backend"];
        }
    }

    main
}

// pub fn main() {
//     let sound = Sound::from_path("BeetrootKvass.wav").unwrap();
//     let sample_rate = sound.sample_rate();
//     let duration = sound.duration();
//     println!("Duration: {:?}", duration);
//     println!["Sample rate: {:?}", sample_rate];

//     let mut mixer = Mixer::new();
//     mixer.init();

//     let playing_sound = mixer.play(sound);

//     while !playing_sound.finished() {
//         std::thread::sleep(Duration::from_millis(500));
//         let index = playing_sound.index();
//         let secs = index as f32 / sample_rate as f32;
//         let finished = secs / duration.as_secs_f32();
//         println!["Current seconds: {:?}", secs];
//         println!["Percentage finished: {:?}", finished * 100.0];
//     }
// }
