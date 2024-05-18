use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, Mutex},
    time,
};

use iced::{
    advanced::widget::Id as WId,
    keyboard,
    widget::{
        column,
        container::{Container, Id as CId},
        row, scrollable, Space,
    },
    Alignment, Command as Cm, Element, Length, Subscription,
};
use reqwest::Url;

use crate::{
    audio::{AudioProgressTracker, TrackerMsg, YTMRSAudioManager},
    backend_handler::{BackendHandler, BackendLaunchStatus, RequestResult},
    caching::FileCache,
    playlist::PlaylistMessage,
    response_types::{YTResponseError, YTResponseType},
    search_window::{SWMessage, SearchType, SearchWindow},
    settings::{project_data_dir, YTMRSettings},
    song::Song,
    song_operations::{
        ConstructorItem, OperationTracker, SongOpConstructor, SongOpTracker, TreeDirected,
        UpdateResult,
    },
    styling::{BasicYtmrsScheme, FullYtmrsScheme},
    user_input::UserInputs,
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
            cache: (true, time::Duration::from_secs(20)),
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

fn songs_path() -> PathBuf {
    let mut path = project_data_dir();
    path.push("songs.ndjson");
    path
}

fn playlists_directory() -> PathBuf {
    let mut path = project_data_dir();
    path.push("playlists");
    path
}

#[derive(Debug)]
pub struct YtmrsCache {
    pub songs: FileCache<Song>,
}

impl Default for YtmrsCache {
    fn default() -> Self {
        Self {
            songs: FileCache::new(songs_path()),
        }
    }
}

#[derive(Debug, Default)]
pub struct Ytmrs {
    inputs: UserInputs,
    search: SearchWindow,

    audio_manager: YTMRSAudioManager,
    audio_tracker: AudioProgressTracker,

    tickers: Tickers,
    backend_handler: Arc<Mutex<BackendHandler>>,
    pub settings: YTMRSettings,

    cache: YtmrsCache,
}

#[derive(Debug, Clone)]
pub enum YtmrsMsg {
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),

    CacheTick,
    BackendStatusTick,
    BackendStatusPollSuccess,
    BackendStatusPollFailure(String),

    PlayingStatusTick,

    CachingSuccess(HashSet<String>),
    CachingFailure,

    RequestRecieved(RequestResult),
    RequestParsed(Box<YTResponseType>),
    RequestParseFailure(YTResponseError),

    SetNewBackground(String, BasicYtmrsScheme),

    SearchWindowMessage(SWMessage),
    PlaylistMsg(PlaylistMessage),
    AudioTrackerMessage(TrackerMsg),

    ModifierChanged(keyboard::Modifiers),
}

impl Ytmrs {
    pub fn new(settings: YTMRSettings, backend_handler: Arc<Mutex<BackendHandler>>) -> Self {
        Self {
            settings,
            backend_handler,
            ..Self::default()
        }
    }

    pub fn load(&mut self) -> Cm<YtmrsMsg> {
        self.settings
            .playlist
            .constructor
            .update_cache(&mut self.cache.songs);

        let mut backend = self.backend_handler.lock().unwrap();

        if let BackendLaunchStatus::Unknown = backend.status {
            *backend = BackendHandler::load(None);
        }

        Cm::none()
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::batch([
            self.tickers.subscription(),
            // Handle tracking modifiers
            keyboard::on_key_press(|_, m| Some(YtmrsMsg::ModifierChanged(m))),
            keyboard::on_key_release(|_, m| Some(YtmrsMsg::ModifierChanged(m))),
        ])
    }

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let backend = self.backend_handler.lock().unwrap();
        let backend_status = backend.status.as_string();

        let search = self.search.view(&scheme).map(YtmrsMsg::SearchWindowMessage);

        let current_playlist = scrollable(
            self.settings
                .playlist
                .view(&scheme)
                .map(YtmrsMsg::PlaylistMsg),
        )
        .style(scheme.scrollable_style.clone().update())
        .width(Length::Fill);

        let base_drop_target = Container::new(Space::with_height(Length::Fill))
            .width(Length::Fill)
            .id(CId::new("base_drop_target"));

        let tracker = self
            .audio_tracker
            .view(&scheme)
            .map(YtmrsMsg::AudioTrackerMessage);

        Element::new(
            column![
                backend_status,
                row![search, column![current_playlist, base_drop_target]],
                tracker
            ]
            .align_items(Alignment::Center)
            .spacing(20),
        )
        // .explain(Color::WHITE)
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Cm<YtmrsMsg> {
        let command = match message {
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
                            duration: entry.duration as f64,
                            artists: Some(vec![entry.channel.clone()]),
                            ..Default::default()
                        })
                        .collect();

                    let keys: Vec<_> = songs.iter().map(|s| &s.id).cloned().collect();

                    self.search.search_type = SearchType::new_tab(keys.clone());

                    let keyset: HashSet<_> = keys.into_iter().collect();

                    // Add the songs to the file cache
                    Cm::perform(
                        FileCache::extend(
                            self.cache.songs.filepath.clone(),
                            songs.into_iter(),
                            true,
                        ),
                        move |s| match s {
                            Ok(_) => YtmrsMsg::CachingSuccess(keyset),
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
            YtmrsMsg::SearchWindowMessage(msg) => {
                match msg {
                    SWMessage::SearchQuerySubmitted => {
                        // Check if URL is valid
                        match Url::parse(&self.search.query) {
                            Ok(_) => Cm::perform(
                                self.backend_handler
                                    .lock()
                                    .unwrap()
                                    .request_info(self.search.query.clone())
                                    .unwrap(),
                                YtmrsMsg::RequestRecieved,
                            ),
                            // URL failed to parse, try to search Youtube
                            Err(e) => {
                                println![
                                    "Failed to parse: \"{}\". assuming it's a search query",
                                    e
                                ];
                                Cm::perform(
                                    self.backend_handler
                                        .lock()
                                        .unwrap()
                                        .request_search(self.search.query.clone())
                                        .unwrap(),
                                    YtmrsMsg::RequestRecieved,
                                )
                            }
                        }
                    }
                    _ => self
                        .search
                        .update(msg, &self.inputs.modifiers)
                        .map(|msg| match msg {
                            SWMessage::HandleZones(k, z) => YtmrsMsg::HandleZones(k, z),
                            _ => YtmrsMsg::SearchWindowMessage(msg),
                        }),
                }
            }
            YtmrsMsg::PlaylistMsg(msg) => {
                match msg {
                    PlaylistMessage::ConstructorMessage(msg) => {
                        match self.settings.playlist.constructor.update(msg) {
                            UpdateResult::Cm(cm) => cm.map(|m| {
                                YtmrsMsg::PlaylistMsg(PlaylistMessage::ConstructorMessage(m))
                            }),
                            UpdateResult::SongClicked(wid) => {
                                self.song_clicked(wid);
                                Cm::none()
                            }
                            UpdateResult::Move(from, to) => {
                                // Remove item at `from` and place it to `to`
                                let from_path =
                                    self.settings.playlist.constructor.path_to_id(&from);
                                let to_path = self.settings.playlist.constructor.path_to_id(&to);
                                if from_path.is_none() || to_path.is_none() {
                                    return Cm::none();
                                }
                                let from_path = from_path.unwrap();
                                let to_path = to_path.unwrap();

                                self.so_move(from_path, to_path);

                                Cm::none()
                            }
                            UpdateResult::None => Cm::none(),
                        }
                    }
                    _ => self
                        .settings
                        .playlist
                        .update(msg)
                        .map(YtmrsMsg::PlaylistMsg),
                }
            }
            YtmrsMsg::AudioTrackerMessage(msg) => match &msg {
                TrackerMsg::Pause => todo!(),
                TrackerMsg::Play => todo!(),
                TrackerMsg::Next => todo!(),
                TrackerMsg::Previous => todo!(),
                TrackerMsg::UpdateVolume(_) => todo!(),
                TrackerMsg::ProgressSliderChanged(_) => self
                    .audio_tracker
                    .update(msg)
                    .map(YtmrsMsg::AudioTrackerMessage),
                TrackerMsg::ProgressSliderReleased(v) => {
                    self.audio_manager.seek(v);
                    self.audio_tracker
                        .update(msg)
                        .map(YtmrsMsg::AudioTrackerMessage)
                }
            },
            YtmrsMsg::HandleZones(song_key, zones) => {
                if zones.is_empty() {
                    return Cm::none();
                }

                let top = &mut self.settings.playlist.constructor;
                println!["{:?}", zones];

                if let Some((id, _)) = zones.iter().rev().find(|(id, _r)| top.item_has_id(id)) {
                    println!["Target: {:#?}", id];

                    let mut path = top.path_to_id(id).unwrap();
                    println!["{:?}", path];
                    let selected_items = self
                        .search
                        .selected_keys()
                        .into_iter()
                        .map(|k| ConstructorItem::from(k.clone()));
                    let mut idx = path.pop().unwrap_or(0);
                    for item in selected_items {
                        path.push(idx);
                        top.push_to_path(VecDeque::from(path.clone()), item);
                        path.pop();
                        idx += 1;
                    }
                } else if let Some((id, _)) = zones.last() {
                    if *id == WId::new("base_drop_target") {
                        top.push_to_path(VecDeque::new(), song_key.into());
                        self.settings
                            .playlist
                            .constructor
                            .update_cache(&mut self.cache.songs);
                    }
                }
                self.settings
                    .playlist
                    .constructor
                    .update_cache(&mut self.cache.songs);

                Cm::none()
            }
            YtmrsMsg::CachingSuccess(keys) => {
                println!["Caching success!"];
                let new_songs = self.cache.songs.fetch(&keys);
                self.search.cache.extend(new_songs);

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
                self.backend_handler.lock().unwrap().poll();
                Cm::none()
            }
            YtmrsMsg::BackendStatusPollSuccess => Cm::none(),
            YtmrsMsg::BackendStatusPollFailure(e) => {
                println!["Polling failure: {:?}", e];
                let mut backend = self.backend_handler.lock().unwrap();
                backend.status = BackendLaunchStatus::Unknown;
                todo!()
            }
            YtmrsMsg::PlayingStatusTick => {
                self.audio_tracker.update_from_manager(&self.audio_manager);
                Cm::none()
            }
        };
        command
    }

    /// Moves an item in the constructor from one position to another
    fn so_move(&mut self, from: Vec<usize>, to: Vec<usize>) {
        let item = self
            .settings
            .playlist
            .constructor
            .pop_path(from.clone().into());
        if item.is_none() {
            return;
        }
        let item = item.unwrap();

        self.settings
            .playlist
            .constructor
            .push_to_path(to.clone().into(), item);
        let mut parent_path = to.clone();
        parent_path.pop();

        let item_at_id: Option<&mut SongOpConstructor> = if parent_path.is_empty() {
            Some(&mut self.settings.playlist.constructor)
        } else {
            let item_at_id = self
                .settings
                .playlist
                .constructor
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
            parent.update_cache(&mut self.cache.songs);
        } else {
            self.settings
                .playlist
                .constructor
                .update_cache(&mut self.cache.songs);
        }
    }

    /// Generate the song tracker when a song is clicked
    fn song_clicked(&mut self, wid: WId) {
        let path = self.settings.playlist.constructor.path_to_id(&wid).unwrap();

        println!["Given path: {:?}", path];
        let song_op = self.settings.playlist.constructor.build();
        let tracker = SongOpTracker::from_song_op(&song_op, path.into());
        println!["SongOPTracker: {:?}", tracker];
        let generated_path: VecDeque<usize> = tracker.get_current().collect();
        println!["Generated path: {:?}", generated_path];
        let item = self
            .settings
            .playlist
            .constructor
            .item_at_path(generated_path.clone());
        println!["Estimated item at path: {:?}", item];
        println!["Infinite loop type: {:?}", song_op.loop_type()];
        println!["Is valid: {:?}", song_op.is_valid()];
        if let Some(ConstructorItem::Song(k, _)) = item {
            let songs = self.cache.songs.fetch(&HashSet::from([k.clone()]));
            if !songs.is_empty() {
                println!["Song found in cache"];

                // self.cache.songs.update(generated_path, songs);
            }
        }
    }

    /// Cache cleanup every fer seconds
    fn clean_cache(&mut self) {
        println!["CACHE TICK:"];
        let statistics = {
            let queue: HashSet<String> = self.search.used_keys().into_iter().cloned().collect();
            let qarcs: HashSet<String> = self.search.cache.get_keys().clone();
            let queue_count = queue.len();
            let qarcs_count = qarcs.len();
            let deleted_count = if queue != qarcs {
                let used_arcs: HashSet<String> = qarcs.intersection(&queue).cloned().collect();

                self.search
                    .cache
                    .replace(self.search.cache.get(&used_arcs).collect());
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
            let opcache = self.settings.playlist.constructor.cache_size();
            self.settings
                .playlist
                .constructor
                .update_cache(&mut self.cache.songs);
            let new_opcache = self.settings.playlist.constructor.cache_size();
            let diff = new_opcache as isize - opcache as isize;
            println!["   {:?} arcs changed in constructor", diff];
        }

        let unused: Vec<String> = self.cache.songs.find_unused_items().collect();
        let unused_count = unused.len();
        self.cache.songs.drop_from_cache(unused);
        println!["   {:?} songs dropped from cache", unused_count];
        println![
            "   {:?} songs currently in cache",
            self.cache.songs.cache_size()
        ]
    }

    async fn parse_request(response: String) -> Result<YTResponseType, YTResponseError> {
        YTResponseType::new(response)
    }
}
