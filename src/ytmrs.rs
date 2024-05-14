use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    time,
};

use iced::{
    advanced::widget::Id as WId,
    alignment::Horizontal,
    keyboard,
    widget::{
        column,
        container::{Container, Id as CId},
        row, scrollable, Space,
    },
    Alignment, Command as Cm, Element, Length, Subscription,
};
use iced_drop::{droppable, zones_on_point};
use reqwest::{Client, Url};

use crate::{
    backend_handler::{BackendHandler, BackendLaunchStatus, RequestResult},
    caching::FileCache,
    response_types::{YTResponseError, YTResponseType},
    settings::{SongKey, YTMRSettings},
    song::{Song, SongData},
    song_operations::{
        ConstructorItem, OperationTracker, SongOpConstructor, SongOpMessage, SongOpTracker,
        TreeDirected, UpdateResult,
    },
    styling::{BasicYtmrsScheme, FullYtmrsScheme},
    user_input::{InputMessage, SelectionMode, UserInputs},
};

#[derive(Debug)]
pub struct Tickers {
    cache: (bool, time::Duration),
    backend_status: (bool, time::Duration),
    playing_status: (bool, time::Duration),
}
impl Default for Tickers {
    fn default() -> Self {
        Self {
            cache: (true, time::Duration::from_secs(4)),
            backend_status: (true, time::Duration::from_secs(10)),
            playing_status: (false, time::Duration::from_secs(1)),
        }
    }
}
impl Tickers {
    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        let mut subs = vec![];
        if self.cache.0 {
            subs.push(iced::time::every(self.cache.1).map(|_| YtmrsMsg::CacheTick));
        }
        if self.backend_status.0 {
            subs.push(
                iced::time::every(self.backend_status.1).map(|_| YtmrsMsg::BackendStatusTick),
            );
        }
        if self.playing_status.0 {
            subs.push(
                iced::time::every(self.playing_status.1).map(|_| YtmrsMsg::PlayingStatusTick),
            );
        }
        Subscription::batch(subs)
    }
}

#[derive(Debug, Default)]
pub struct Ytmrs {
    inputs: UserInputs,
    // audio_manager: YTMRSAudioManager,
    tickers: Tickers,
    backend_handler: BackendHandler,
    pub settings: YTMRSettings,
}

#[derive(Debug, Clone)]
pub enum YtmrsMsg {
    Drop(String, iced::Point, iced::Rectangle),
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),

    CacheTick,
    BackendStatusTick,
    BackendStatusPollSuccess,
    BackendStatusPollFailure(String),

    PlayingStatusTick,

    SongClicked(usize, String),
    InputMessage(InputMessage),

    CachingSuccess(HashSet<String>),
    CachingFailure,

    RequestRecieved(RequestResult),
    RequestParsed(Box<YTResponseType>),
    RequestParseFailure(YTResponseError),

    SetNewBackground(String, BasicYtmrsScheme),

    SongOpMsg(SongOpMessage),

    ModifierChanged(keyboard::Modifiers),
}

impl Ytmrs {
    pub fn new(settings: YTMRSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn load(&mut self) -> Cm<YtmrsMsg> {
        let arcs = self
            .settings
            .cached_songs
            .fetch(&self.settings.queue.iter().cloned().collect());

        self.settings.queue_cache.extend(arcs);

        self.settings
            .operation_constructor
            .update_cache(&mut self.settings.cached_songs);

        self.backend_handler = BackendHandler::load(None);

        Cm::none()
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::batch([
            self.tickers.subscription(),
            keyboard::on_key_press(|_, m| Some(YtmrsMsg::ModifierChanged(m))),
            keyboard::on_key_release(|_, m| Some(YtmrsMsg::ModifierChanged(m))),
        ])
    }

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let input = self.inputs.view().map(YtmrsMsg::InputMessage);
        let keys: HashSet<SongKey> = self.settings.queue.iter().cloned().collect();
        let cached_map: HashMap<_, _> = self.settings.queue_cache.get(&keys).collect();

        let backend_status: Element<YtmrsMsg> = self.backend_handler.view();

        let songs = self.settings.queue.iter().enumerate().map(|(idx, key)| {
            let selected = {
                match &self.inputs.selection_mode {
                    SelectionMode::None => false,
                    SelectionMode::Single(idx_) => idx == *idx_,
                    SelectionMode::Multiple(v) => v.contains(&idx),
                    SelectionMode::Range { first: _, r } => r.contains(&idx),
                }
            };
            let style = scheme.song_appearance.update(selected);
            droppable(
                Container::new(match cached_map.get(key) {
                    Some(songc) => {
                        let song = songc.lock().unwrap();
                        song.as_data().column::<YtmrsMsg>()
                    }
                    None => SongData::mystery().column::<YtmrsMsg>(),
                })
                .style(move |_| style),
            )
            .on_drop(move |pt, rec| YtmrsMsg::Drop(key.clone(), pt, rec))
            .on_single_click(YtmrsMsg::SongClicked(idx, key.clone()))
            .into()
        });

        let song_list = scrollable(
            Container::new(column(songs).width(Length::Fill))
                .align_x(Horizontal::Left)
                .width(Length::Fill)
                .max_width(400)
                .padding(0)
                .align_x(Horizontal::Left),
        )
        .style(scheme.scrollable_style.clone().update());

        let constructor = scrollable(
            self.settings
                .operation_constructor
                .view(&scheme)
                .map(YtmrsMsg::SongOpMsg),
        )
        .style(scheme.scrollable_style.clone().update())
        .width(Length::Fill);

        let base_drop_target = Container::new(Space::with_height(Length::Fill))
            .height(Length::Shrink)
            .width(Length::Fill)
            .id(CId::new("base_drop_target"));

        let constructor_row = column![constructor, base_drop_target];

        column![
            row![input, backend_status],
            row![song_list, constructor_row]
        ]
        .align_items(Alignment::Center)
        .spacing(20)
        .padding(10)
        .into()
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Cm<YtmrsMsg> {
        let command = match message {
            YtmrsMsg::SongClicked(clicked_idx, key) => {
                println![
                    "{:?} Pressed with modifiers {:?}",
                    key, self.inputs.modifiers
                ];
                self.inputs.selection_mode = self
                    .inputs
                    .selection_mode
                    .clone()
                    .update_selection(clicked_idx, &self.inputs.modifiers);
                println!["Selection: {:#?}", self.inputs.selection_mode];

                Cm::none()
            }
            YtmrsMsg::ModifierChanged(m) => {
                self.inputs.modifiers = m;

                Cm::none()
            }
            YtmrsMsg::SetNewBackground(_, _) => {
                // // Save primary color to cache for future use
                // let mut handle = self.settings.index.get(&key);
                // if handle.get_color().is_none() {
                //     handle.set_color(scheme.primary_color);
                //     println!["Saved primary color: {:?}", scheme.primary_color];
                // }
                Cm::none()
            }
            YtmrsMsg::InputMessage(InputMessage::UrlSubmitted) => {
                // Check if URL is valid
                match Url::parse(&self.inputs.url) {
                    Ok(_) => Cm::perform(
                        self.backend_handler
                            .request_info(self.inputs.url.clone())
                            .unwrap(),
                        YtmrsMsg::RequestRecieved,
                    ),
                    // URL failed to parse, try to search Youtube
                    Err(e) => {
                        println!["Failed to parse: \"{}\". assuming it's a search query", e];
                        Cm::perform(
                            self.backend_handler
                                .request_search(self.inputs.url.clone())
                                .unwrap(),
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
                YTResponseType::Song(_song) => {
                    println!["Request is a song"];
                    todo!()
                }
                YTResponseType::Tab(t) => {
                    println!["Request is a 'tab'"];
                    self.settings.queue.clear();
                    let songs: Vec<Song> = t
                        .entries
                        .iter()
                        .map(|entry| Song {
                            id: entry.id.clone(),
                            title: entry.title.clone(),
                            channel: entry.channel.clone(),
                            view_count: entry.view_count,
                            thumbnail: entry.thumbnails[0].url.clone(),
                            webpage_url: entry.url.clone(),
                            duration: entry.duration,
                            artists: Some(vec![entry.channel.clone()]),
                            ..Default::default()
                        })
                        .collect();

                    let keys: HashSet<_> = songs.iter().map(|s| &s.id).cloned().collect();
                    self.settings.queue.clear();
                    self.settings.queue.extend(keys.clone());

                    Cm::perform(
                        FileCache::extend(
                            self.settings.cached_songs.filepath.clone(),
                            songs.clone().into_iter(),
                            true,
                        ),
                        move |s| match s {
                            Ok(_) => YtmrsMsg::CachingSuccess(keys),
                            Err(_) => YtmrsMsg::CachingFailure,
                        },
                    )
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
            YtmrsMsg::SongOpMsg(msg) => {
                match self.settings.operation_constructor.update(msg) {
                    UpdateResult::Cm(cm) => cm.map(YtmrsMsg::SongOpMsg),
                    UpdateResult::SongClicked(wid) => {
                        self.song_clicked(wid);
                        Cm::none()
                    }
                    UpdateResult::Move(from, to) => {
                        // Remove item at `from` and place it to `to`
                        let from_path = self.settings.operation_constructor.path_to_id(&from);
                        let to_path = self.settings.operation_constructor.path_to_id(&to);
                        if from_path.is_none() || to_path.is_none() {
                            return Cm::none();
                        }
                        let from_path = from_path.unwrap();
                        let to_path = to_path.unwrap();

                        let item = self
                            .settings
                            .operation_constructor
                            .pop_path(from_path.clone().into());
                        if item.is_none() {
                            return Cm::none();
                        }
                        let item = item.unwrap();

                        self.settings
                            .operation_constructor
                            .push_to_path(to_path.clone().into(), item);

                        let mut parent_path = to_path.clone();
                        parent_path.pop();

                        let item_at_id: Option<&mut SongOpConstructor> = if parent_path.is_empty() {
                            Some(&mut self.settings.operation_constructor)
                        } else {
                            let item_at_id = self
                                .settings
                                .operation_constructor
                                .item_at_path_mut(parent_path.into());

                            match item_at_id {
                                Some(item) => match item {
                                    ConstructorItem::Song(_, _) => None,
                                    ConstructorItem::Operation(opc) => Some(opc),
                                },
                                None => None,
                            }
                        };

                        if let Some(parent) = item_at_id {
                            parent.update_cache(&mut self.settings.cached_songs);
                        } else {
                            self.settings
                                .operation_constructor
                                .update_cache(&mut self.settings.cached_songs);
                        }

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
                        top.push_to_path(VecDeque::new(), song_key.into());
                        self.settings
                            .operation_constructor
                            .update_cache(&mut self.settings.cached_songs);
                    }
                }
                self.settings
                    .operation_constructor
                    .update_cache(&mut self.settings.cached_songs);

                Cm::none()
            }
            YtmrsMsg::CachingSuccess(keys) => {
                println!["Caching success!"];
                let new_songs = self.settings.cached_songs.fetch(&keys);
                self.settings.queue_cache.extend(new_songs);
                Cm::none()
            }
            YtmrsMsg::CachingFailure => {
                println!["Caching failure!"];
                Cm::none()
            }
            YtmrsMsg::CacheTick => {
                self.clean_cache();
                Cm::none()
            }
            YtmrsMsg::BackendStatusTick => {
                self.backend_handler.poll();
                Cm::none()
            }
            YtmrsMsg::BackendStatusPollSuccess => Cm::none(),
            YtmrsMsg::BackendStatusPollFailure(e) => {
                println!["Polling failure: {:?}", e];
                self.backend_handler.status = BackendLaunchStatus::Unknown;
                todo!()
            }
            YtmrsMsg::PlayingStatusTick => todo!(),
        };
        command
    }

    fn song_clicked(&mut self, wid: WId) {
        let path = self
            .settings
            .operation_constructor
            .path_to_id(&wid)
            .unwrap();

        println!["Given path: {:?}", path];
        let song_op = self.settings.operation_constructor.build();
        let tracker = SongOpTracker::from_song_op(&song_op, path.into());
        println!["SongOPTracker: {:?}", tracker];
        let generated_path: VecDeque<usize> = tracker.get_current().collect();
        println!["Generated path: {:?}", generated_path];
        let item = self
            .settings
            .operation_constructor
            .item_at_path(generated_path.clone());
        println!["Estimated item at path: {:?}", item];
        println!["Infinite loop type: {:?}", song_op.loop_type()];
        println!["Is valid: {:?}", song_op.is_valid()];
    }

    fn clean_cache(&mut self) {
        println!["CACHE TICK:"];
        let statistics = {
            let queue: HashSet<String> = self.settings.queue.iter().cloned().collect();
            let qarcs: HashSet<String> = self.settings.queue_cache.get_keys().clone();
            let queue_count = queue.len();
            let qarcs_count = qarcs.len();
            let deleted_count = if queue != qarcs {
                let used_arcs: HashSet<String> = qarcs.intersection(&queue).cloned().collect();

                self.settings
                    .queue_cache
                    .replace(self.settings.queue_cache.get(&used_arcs).collect());
                qarcs_count - used_arcs.len()
            } else {
                0
            };

            (queue_count, qarcs_count, deleted_count)
        };
        println![
            "   {:?} songs in the queue\n   {:?} songs in arcs\n   {:?} unused arcs deleted",
            statistics.0, statistics.1, statistics.2
        ];

        {
            let opcache = self.settings.operation_constructor.cache_size();
            self.settings
                .operation_constructor
                .update_cache(&mut self.settings.cached_songs);
            let new_opcache = self.settings.operation_constructor.cache_size();
            let diff = new_opcache as isize - opcache as isize;
            println!["   {:?} arcs changed in constructor", diff];
        }

        let unused: Vec<String> = self.settings.cached_songs.find_unused_itmes().collect();
        let unused_count = unused.len();
        self.settings.cached_songs.drop_from_cache(unused);
        println!["   {:?} songs dropped from cache", unused_count];
        println![
            "   {:?} songs currently in cache",
            self.settings.cached_songs.cache_size()
        ]
    }

    async fn parse_request(response: String) -> Result<YTResponseType, YTResponseError> {
        YTResponseType::new(response)
    }
}
