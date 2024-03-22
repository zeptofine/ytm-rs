use iced::alignment::{Alignment, Horizontal, Vertical};
use iced::keyboard;
use iced::widget::{
    button, checkbox, column, container, keyed_column, radio, row, scrollable, text, text_input,
    Column, Text,
};
use iced::{Command, Element, Length, Subscription};
use iced_aw::{color_picker, number_input};
use once_cell::sync::Lazy;
use song_list_file::{LoadError, SaveError, SongFileState};

mod song;
mod song_list_file;
mod thumbnails;
use song::{Song, SongMessage};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Default)]
struct Main {
    url: String,

    songs: Vec<Song>,
}

#[derive(Debug, Clone)]
enum Message {
    UrlChanged(String),
    SearchUrl,
    SongMessage(SongMessage),
}

impl Main {
    fn new(state: SongFileState) -> Self {
        Self {
            songs: state.songs,
            ..Main::default()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<Message> {
        let input = column![text_input("Youtube URL....", &self.url)
            .id(INPUT_ID.clone())
            .on_input(Message::UrlChanged)
            .on_submit(Message::SearchUrl)
            .padding(15)
            .size(20),];

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
                    .align_x(Horizontal::Center)
            ),
        ]
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UrlChanged(s) => {
                self.url = s;
                Command::none()
            }
            Message::SearchUrl => {
                todo!();
            }
            Message::SongMessage(_) => {
                println!["Song was clicked"];
                Command::none()
            }
        }
    }
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
    Loaded(Result<SongFileState, LoadError>),
    Save,
    Saved(Result<std::path::PathBuf, SaveError>),
    MainMessage(Message),
}

impl YTMRS {
    fn load() -> Command<YTMRSMessage> {
        Command::perform(SongFileState::load(), YTMRSMessage::Loaded)
    }

    fn subscription(&self) -> Subscription<YTMRSMessage> {
        Subscription::batch([
            keyboard::on_key_press(|key_code, modifiers| {
                if !modifiers.command() {
                    return None;
                }
                Self::handle_hotkey(key_code)
            }),
            match self {
                Self::Loaded { state, saving: _ } => {
                    state.subscription().map(YTMRSMessage::MainMessage)
                }
                _ => Subscription::none(),
            },
        ])
    }

    fn handle_hotkey(key: keyboard::Key) -> Option<YTMRSMessage> {
        println!["{key:?}"];
        None
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
                    Command::perform(
                        SongFileState {
                            songs: state.songs.clone(),
                        }
                        .save(),
                        YTMRSMessage::Saved,
                    )
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
    iced::program("A cool song list", YTMRS::update, YTMRS::view)
        .load(YTMRS::load)
        .subscription(YTMRS::subscription)
        .run()
}
