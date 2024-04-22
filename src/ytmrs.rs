use std::{collections::VecDeque, fmt::Debug};

use iced::{
    advanced::widget::Id as WId,
    alignment::{Alignment, Horizontal},
    widget::{column, container::Id as CId, row, scrollable, text_input, Container, Space},
    Command as Cm, Element, Length,
};
use iced_drop::{droppable, zones_on_point};
use once_cell::sync::Lazy;
use reqwest::{Client, Url};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use serde::Serialize;

use crate::{
    cache_handlers::YtmCache,
    response_types::{YTResponseError, YTResponseType, YTSong},
    settings::YTMRSettings,
    song::{Song, SongMessage},
    song_operations::{ConstructorItem, SongOpMessage, TreeDirected, UpdateResult},
    styling::{color_to_argb, BasicYtmrsScheme, FullYtmrsScheme},
    thumbnails::ThumbnailState,
};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Default)]
struct UserInputs {
    url: String,
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    UrlChanged(String),
    UrlSubmitted,
}

impl UserInputs {
    fn view(&self) -> Element<InputMessage> {
        column![text_input("", &self.url)
            .id(INPUT_ID.clone())
            .on_input(InputMessage::UrlChanged)
            .on_submit(InputMessage::UrlSubmitted)
            .size(20)
            .padding(15),]
        .into()
    }

    fn update(&mut self, message: InputMessage) -> Cm<InputMessage> {
        match message {
            InputMessage::UrlChanged(s) => self.url = s,
            InputMessage::UrlSubmitted => {}
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
    Drop(String, iced::Point, iced::Rectangle),
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),

    SongClicked(String),
    SongMessage(String, SongMessage),
    InputMessage(InputMessage),

    RequestRecieved(RequestResult),
    RequestParsed(Box<YTResponseType>),
    RequestParseFailure(YTResponseError),

    SetNewBackground(String, BasicYtmrsScheme),

    OpConstructorMsg(SongOpMessage),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Success(String),
    RequestError,
    JsonParseError,
}

async fn request_info(url: String) -> RequestResult {
    println!["Requesting info for {}", url];
    let client = Client::new();

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

async fn request_search(query: String) -> RequestResult {
    let mut url = Url::parse("http://localhost:55001/search").unwrap();
    url.query_pairs_mut().append_pair("q", &query);
    println!["{}", url];

    let client = Client::new();

    match client.get(url).send().await {
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

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let input = self.inputs.view().map(YtmrsMsg::InputMessage);

        let songs = self.settings.queue.iter().map(|song| {
            droppable(
                self.settings.saved_songs[song]
                    .view(&scheme.song_appearance)
                    .map(|msg| YtmrsMsg::SongMessage(song.clone(), msg)),
            )
            // .on_click(YtmrsMsg::SongClicked(song.clone()))
            .on_drop(move |pt, rec| YtmrsMsg::Drop(song.clone(), pt, rec))
            .into()
        });

        let constructor = scrollable(
            self.settings
                .operation_constructor
                .view(&self.settings.saved_songs, &scheme)
                .map(YtmrsMsg::OpConstructorMsg),
        )
        .style(scheme.scrollable_style.clone().update())
        .width(Length::Fill);

        let base_drop_target = Container::new(Space::with_height(Length::Fill))
            .height(Length::Shrink)
            .width(Length::Fill)
            .id(CId::new("base_drop_target"));

        let song_list = scrollable(
            Container::new(column(songs))
                .width(Length::Fill)
                .max_width(400)
                .padding(0)
                .align_x(Horizontal::Left),
        )
        .style(scheme.scrollable_style.update());

        column![
            input,
            row![song_list, column![constructor, base_drop_target]]
        ]
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Cm<YtmrsMsg> {
        match message {
            YtmrsMsg::SongMessage(key, msg) => self
                .settings
                .saved_songs
                .get_mut(&key)
                .unwrap()
                .update(msg)
                .map(move |msg| YtmrsMsg::SongMessage(key.clone(), msg)),
            YtmrsMsg::SongClicked(key) => {
                let song = self.settings.saved_songs.get_mut(&key).unwrap();
                // Add song to queue
                self.settings
                    .operation_constructor
                    .push(ConstructorItem::from(key.clone()));

                Cm::batch([
                    // Change background color to indicate the playing song
                    match song.thumbnail.clone() {
                        ThumbnailState::Unknown => Cm::none(),
                        ThumbnailState::Downloaded { path, handle: _ } => {
                            let handle = self.settings.index.get(&key);
                            match &handle.get_color() {
                                Some(col) => Cm::perform(
                                    BasicYtmrsScheme::from_argb(color_to_argb(*col)),
                                    |scheme| YtmrsMsg::SetNewBackground(key, scheme),
                                ),
                                None => Cm::perform(BasicYtmrsScheme::from_image(path), |scheme| {
                                    YtmrsMsg::SetNewBackground(key, scheme)
                                }),
                            }
                        }
                    },
                ])
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
            YtmrsMsg::InputMessage(InputMessage::UrlSubmitted) => {
                // Check if URL is valid
                match Url::parse(&self.inputs.url) {
                    Ok(_) => Cm::perform(
                        request_info(self.inputs.url.clone()),
                        YtmrsMsg::RequestRecieved,
                    ),
                    // URL failed to parse, try to search Youtube
                    Err(e) => {
                        println!["Failed to parse: \"{}\". assuming it's a search query", e];
                        Cm::perform(
                            request_search(self.inputs.url.clone()),
                            YtmrsMsg::RequestRecieved,
                        )
                    }
                }
            }
            YtmrsMsg::InputMessage(i) => self.inputs.update(i).map(YtmrsMsg::InputMessage),
            YtmrsMsg::RequestRecieved(response) => match response {
                RequestResult::Success(s) => {
                    Cm::perform(Ytmrs::parse_request(s), |result| match result {
                        Ok(r) => YtmrsMsg::RequestParsed(Box::new(r)),
                        Err(e) => YtmrsMsg::RequestParseFailure(e),
                    })
                }
                _ => {
                    println!["{:?}", response];
                    Cm::none()
                }
            },
            YtmrsMsg::RequestParsed(response_type) => match *response_type {
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
            YtmrsMsg::OpConstructorMsg(msg) => {
                match self.settings.operation_constructor.update(msg) {
                    UpdateResult::Cm(cm) => cm.map(YtmrsMsg::OpConstructorMsg),
                    UpdateResult::Move(from, to) => {
                        println!["From: {:?}\nTo: {:?}", from, to];

                        // Remove item at `from` and place it to `to`

                        Cm::none()
                    }
                    UpdateResult::None => Cm::none(),
                }
            }
            YtmrsMsg::Drop(key, point, _rec) => zones_on_point(
                move |zones| YtmrsMsg::HandleZones(key.clone(), zones),
                point,
                None,
                None,
            ),
            YtmrsMsg::HandleZones(song_key, zones) => {
                if zones.is_empty() {
                    return Cm::none();
                }

                let top = &mut self.settings.operation_constructor;
                println!["{:?}", zones];

                if let Some((id, _)) = zones.iter().rev().find(|(id, _r)| top.item_has_id(id)) {
                    println!["Target: {:#?}", id];

                    let path = top.path_to_id(id).unwrap();
                    println!["{:?}", path];
                    let item: ConstructorItem = song_key.into();
                    top.push_to_path(VecDeque::from(path), item);
                } else if let Some((id, _)) = zones.last() {
                    if *id == WId::new("base_drop_target") {
                        top.push_to_path(VecDeque::new(), song_key.into())
                    }
                }

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
}

#[derive(Serialize)]
struct RequestInfoDict {
    url: String,
    process: bool,
}
