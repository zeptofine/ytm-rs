use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
    time,
};

use futures::future::join_all;
use iced::{
    advanced::widget::Id as WId,
    keyboard,
    widget::{
        column,
        container::{Container, Id as CId},
        image::Handle,
        row, Space,
    },
    Alignment, Command as Cm, Element, Length, Subscription,
};
use kira::sound::{static_sound::StaticSoundData, PlaybackState};
use parking_lot::Mutex;
use reqwest::Url;

use crate::{
    audio::{AudioProgressTracker, ChangeSong, TrackerMsg, YTMRSAudioManager},
    backend_handler::{BackendHandler, BackendLaunchStatus, RequestResult},
    caching::{
        readers::{folder_based_reader::read_file, CacheReader, FileData},
        BasicSoundData, BufferedCache, IDed, RwMap, SoundData, ToRwMapExt, YtmrsCache,
    },
    playlist::PlaylistMessage,
    response_types::YTResponseType,
    search_window::{SWMessage, SearchEntry, SearchType, SearchWindow},
    settings::YTMRSettings,
    song::{Song, SongState},
    song_operations::{
        self, ConstructorItem, OperationTracker, RecursiveSongOp, SongOpTracker, TreeDirected,
        UpdateResult,
    },
    styling::{BasicYtmrsScheme, FullYtmrsScheme},
    thumbnails::get_images,
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
        let mut subs = Vec::with_capacity(3);
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

#[derive(Debug)]
pub struct PlayerState {
    tracker: SongOpTracker,
}

#[derive(Debug, Default)]
pub struct Ytmrs {
    inputs: UserInputs,
    search: SearchWindow,

    audio_manager: YTMRSAudioManager,
    audio_tracker: AudioProgressTracker,
    player_state: Option<PlayerState>,

    tickers: Tickers,
    backend_handler: Arc<Mutex<BackendHandler>>,
    pub settings: YTMRSettings,

    cache: YtmrsCache,
}

#[derive(Debug, Clone)]
pub enum YtmrsMsg {
    // User input
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),
    KeysChanged(keyboard::Key, keyboard::Modifiers),

    // Ticks
    CacheTick,
    BackendStatusTick,
    BackendStatusPollSuccess,
    BackendStatusPollFailure(String),
    PlayingStatusTick,

    RequestRecieved(RequestResult),

    // Searching
    SearchedKeysReceived {
        existing: RwMap<String, Song>,
        missing: Vec<String>,
    },

    ManagerMsg(ChangeSong),
    SearchWindowMessage(SWMessage),
    PlaylistMsg(PlaylistMessage),
    AudioTrackerMessage(TrackerMsg),

    ImagesFetched {
        map: HashMap<String, Handle>,
        missing: Option<HashSet<String>>,
    },
    SongsFetched {
        map: RwMap<String, Song>,
        get_existing_thumbnails: bool,
    },
    SoundsFetched {
        map: HashMap<String, StaticSoundData>,
        play: Option<String>,
    },
    DownloadSong(String, bool),
    SongDownloaded {
        song: Song,
        play: bool,
    },
    SongDownloadFinished {
        id: String,
        data: Box<BasicSoundData>,
    },

    SetNewBackground(String, BasicYtmrsScheme),
    Null,
}

impl Ytmrs {
    pub fn new(settings: YTMRSettings, backend_handler: Arc<Mutex<BackendHandler>>) -> Self {
        Self {
            audio_tracker: AudioProgressTracker::new(&settings.user),
            settings,
            backend_handler,
            ..Self::default()
        }
    }

    pub fn load(&mut self) -> Cm<YtmrsMsg> {
        // Add the cache to required places
        self.settings
            .playlist
            .constructor
            .set_cache(Arc::clone(&self.cache.song_metadata));
        self.search.cache = Some(Arc::clone(&self.cache.song_metadata));

        let mut backend = self.backend_handler.lock();

        if let BackendLaunchStatus::Unknown = backend.status {
            *backend = BackendHandler::load(None);
        }
        let cache = self.cache.song_metadata.write();
        let metadata_reader = cache.reader.clone();
        let keys = self.all_used_keys();

        Cm::perform(
            async move {
                join_all(metadata_reader.read_from_ids(&keys).await)
                    .await
                    .into_iter()
                    .collect()
            },
            |map| YtmrsMsg::SongsFetched {
                map,
                get_existing_thumbnails: true,
            },
        )
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::batch([
            self.tickers.subscription(),
            // Handle tracking modifiers
            keyboard::on_key_press(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
            keyboard::on_key_release(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
            // Checking when songs finish
            self.audio_manager.subscription().map(YtmrsMsg::ManagerMsg),
        ])
    }

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let backend = self.backend_handler.lock();
        let backend_status = backend.status.as_string();

        let search = self.search.view(&scheme).map(YtmrsMsg::SearchWindowMessage);

        let current_playlist = self
            .settings
            .playlist
            .view(&scheme)
            .map(YtmrsMsg::PlaylistMsg);

        let base_drop_target = Container::new(Space::with_height(Length::Fill))
            .width(Length::Fill)
            .id(CId::new("base_drop_target"));

        let tracker = self
            .audio_tracker
            .view(&scheme)
            .map(YtmrsMsg::AudioTrackerMessage);

        Element::new(
            column![
                column![
                    backend_status,
                    row![search, column![current_playlist, base_drop_target]],
                ]
                .spacing(20),
                tracker
            ]
            .align_items(Alignment::Center),
        )
    }

    pub fn parse_search_request(&mut self, response_type: YTResponseType) -> Cm<YtmrsMsg> {
        match response_type {
            YTResponseType::Song(_song) => {
                println!["Request is a song"];
                Cm::none()
            }
            YTResponseType::Tab(t) => {
                println!["Request is a 'tab'"];

                let songs: Vec<Song> = t
                    .entries
                    .into_iter()
                    .map(|entry| Song {
                        id: entry.id,
                        title: entry.title,
                        channel: entry.channel.clone(),
                        view_count: entry.view_count,
                        webpage_url: entry.url,
                        duration: entry.duration,
                        thumbnail: entry.thumbnails[0].url.clone(),
                        artists: Some(vec![entry.channel.clone()]),
                        ..Default::default()
                    })
                    .collect();

                let keys: Vec<_> = songs.iter().map(|s| s.id.clone()).collect();

                self.search.search_type = SearchType::new_tab(keys.clone());

                let map = songs.iter().map(|s| (s.id.clone(), s.clone())).to_rwmap();

                let ids: HashSet<String> = songs.iter().map(|s| s.id.clone()).collect();

                // Add the songs to the file cache
                let reader = { self.cache.song_metadata.write().reader.clone() };

                Cm::batch([
                    self.download_images_for_ids(ids),
                    Cm::perform(
                        async move {
                            println!("extend songs {:?}", reader.extend(songs, true).await);
                        },
                        move |_| YtmrsMsg::SongsFetched {
                            map,
                            get_existing_thumbnails: true,
                        },
                    ),
                ])
            }
            YTResponseType::Search(s) => {
                println!["Request is a search"];
                // println!["{:?}", s];

                let mut song_keys: HashSet<String> = HashSet::new();
                let mut entries: Vec<SearchEntry> = vec![];
                // Get links for each entry
                for entry in s.entries {
                    let search_entry: SearchEntry = SearchEntry::new(entry).unwrap();
                    if let SearchEntry::Song { id, .. } = &search_entry {
                        song_keys.insert(id.clone());
                    }

                    entries.push(search_entry);
                }

                let existing_keys: HashSet<String> = self
                    .cache
                    .song_metadata
                    .read()
                    .fetch_existing(&song_keys)
                    .keys()
                    .cloned()
                    .collect();

                self.search.search_type = SearchType::Search(entries);

                println!["{:?}, {:?}", existing_keys.len(), song_keys.len()];
                if existing_keys.len() != song_keys.len() {
                    // We need to fetch the metadata for these songs.
                    let missing: HashSet<String> =
                        song_keys.difference(&existing_keys).cloned().collect();

                    let reader = self.cache.song_metadata.write().reader.clone();

                    Cm::perform(
                        async move {
                            let new_songs: RwMap<String, Song> =
                                join_all(reader.read_from_ids(&missing).await)
                                    .await
                                    .into_iter()
                                    .collect();
                            let ids: HashSet<String> = new_songs.keys().cloned().collect();

                            // Find keys that are still missing after fetching
                            let missing = missing.difference(&ids);

                            (new_songs, missing.cloned().collect())
                        },
                        |(existing, missing)| YtmrsMsg::SearchedKeysReceived { existing, missing },
                    )
                } else {
                    self.download_images_for_ids(existing_keys)
                }
            }
        }
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Cm<YtmrsMsg> {
        match message {
            // * User input
            YtmrsMsg::HandleZones(song_key, zones) => {
                if !zones.is_empty() {
                    self.handle_zones(song_key, zones);
                }

                Cm::none()
            }
            YtmrsMsg::KeysChanged(_, m) => {
                println!["{:?}", m];
                self.inputs.modifiers = m;

                Cm::none()
            }

            // * Ticks
            YtmrsMsg::CacheTick => {
                let used_meta_keys: HashSet<String> = self.all_used_keys();
                {
                    let mut metadata = self.cache.song_metadata.write();
                    let available_keys: HashSet<String> =
                        metadata.items().keys().cloned().collect();

                    let unused_keys = available_keys
                        .difference(&used_meta_keys)
                        .cloned()
                        .collect::<Vec<String>>();

                    metadata.drop_from_cache(unused_keys);
                }
                Cm::none()
            }
            YtmrsMsg::BackendStatusTick => {
                self.backend_handler.lock().poll();
                Cm::none()
            }
            YtmrsMsg::BackendStatusPollSuccess => Cm::none(),
            YtmrsMsg::BackendStatusPollFailure(e) => {
                println!["Polling failure: {:?}", e];
                let mut backend = self.backend_handler.lock();
                backend.status = BackendLaunchStatus::Unknown;
                todo!()
            }
            YtmrsMsg::PlayingStatusTick => {
                self.audio_tracker.update_from_manager(&self.audio_manager);
                Cm::none()
            }

            YtmrsMsg::RequestRecieved(response) => match response {
                Ok(s) => {
                    let response_type = YTResponseType::new(s);
                    match response_type {
                        Ok(response_type) => self.parse_search_request(response_type),
                        Err(e) => {
                            println!["Error: {:?}", e];
                            Cm::none()
                        }
                    }
                }
                _ => {
                    println!["{:?}", response];
                    Cm::none()
                }
            },

            // * Searching
            YtmrsMsg::SearchedKeysReceived { existing, missing } => {
                let ids = existing.keys().cloned().collect();
                let reader = {
                    let mut metadata = self.cache.song_metadata.write();
                    metadata.items_mut().extend(existing);
                    metadata.reader.clone()
                };
                // if missing is not empty, then we need to fetch the missing songs
                let backend_handler = self.backend_handler.clone();

                let request_command = async move {
                    let urls = missing.into_iter().map(BackendHandler::request_url_from_id);

                    let requests =
                        join_all(urls.map(|url| backend_handler.lock().request_info(url).unwrap()))
                            .await;
                    let songs: Vec<Song> = requests
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter_map(|s| serde_json::from_str(&s).ok())
                        .collect();

                    println!["485: {:?}", reader.extend(&songs, true).await];

                    songs
                };

                Cm::batch([
                    Cm::perform(request_command, |songs| {
                        let map: HashMap<String, _> =
                            songs.into_iter().map(|s| (s.id().clone(), s)).to_rwmap();

                        YtmrsMsg::SongsFetched {
                            map,
                            get_existing_thumbnails: true,
                        }
                    }),
                    self.download_images_for_ids(ids),
                ])
            }

            YtmrsMsg::ManagerMsg(_) => {
                println!["{:?}", self.audio_manager.playback_state()];
                if let PlaybackState::Playing | PlaybackState::Stopped | PlaybackState::Stopping =
                    self.audio_manager.playback_state()
                {
                    println!["CHANGE SONG!"];
                    println!["STATE: {:#?}", self.player_state];
                    self.play_next_song()
                } else {
                    Cm::none()
                }
            }
            YtmrsMsg::SearchWindowMessage(msg) => {
                match msg {
                    SWMessage::SearchQuerySubmitted => {
                        // Check if URL is valid
                        match Url::parse(&self.search.query) {
                            Ok(_) => Cm::perform(
                                self.backend_handler
                                    .lock()
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
                            Some(msg) => match msg {
                                UpdateResult::Cm(cm) => cm.map(|m| {
                                    YtmrsMsg::PlaylistMsg(PlaylistMessage::ConstructorMessage(m))
                                }),
                                UpdateResult::SongClicked(wid) => self.song_clicked(wid),
                                UpdateResult::Move(from, to) => {
                                    // Remove item at `from` and place it to `to`
                                    println!["MOVE FROM {:?} TO {:?}", from, to];
                                    let from_path =
                                        self.settings.playlist.constructor.path_to_id(&from);

                                    if from_path.is_none() {
                                        return Cm::none();
                                    }
                                    let from_path = from_path.unwrap();
                                    println!["FROM:{:?}", from_path];

                                    let item = self
                                        .settings
                                        .playlist
                                        .constructor
                                        .pop_path(from_path.into());

                                    if item.is_none() {
                                        return Cm::none();
                                    }
                                    let item = item.unwrap();

                                    let to_path =
                                        self.settings.playlist.constructor.path_to_id(&to);

                                    if to_path.is_none() {
                                        return Cm::none();
                                    }

                                    let to_path = to_path.unwrap();
                                    println!["TO:{:?}", to_path];

                                    self.settings
                                        .playlist
                                        .constructor
                                        .push_to_path(to_path.into(), item);

                                    Cm::none()
                                }
                            },
                            None => Cm::none(),
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
                TrackerMsg::Pause => {
                    self.audio_manager.pause();
                    self.audio_tracker.paused = true;
                    self.tickers.playing_status.0 = false;
                    Cm::none()
                }
                TrackerMsg::Play => {
                    self.audio_manager.play();
                    self.audio_tracker.paused = false;
                    self.tickers.playing_status.0 = true;
                    Cm::none()
                }
                TrackerMsg::Next => self.play_next_song(),
                TrackerMsg::Previous => self.rewind(),
                TrackerMsg::UpdateVolume(v) => {
                    println!["{:?}", v];
                    let float_vol = v / 1000_f64;
                    self.audio_manager.set_volume(float_vol);
                    self.audio_tracker.volume = *v;
                    self.settings.user.volume = float_vol as f32;
                    Cm::none()
                }
                TrackerMsg::ProgressSliderChanged(_) => self
                    .audio_tracker
                    .update(msg)
                    .map(YtmrsMsg::AudioTrackerMessage),
                TrackerMsg::ProgressSliderReleased(v) => {
                    self.audio_manager.seek(*v);
                    self.audio_tracker
                        .update(msg)
                        .map(YtmrsMsg::AudioTrackerMessage)
                }
            },

            YtmrsMsg::ImagesFetched { map, missing } => {
                self.push_image_handles(map);

                match missing {
                    None => Cm::none(),
                    Some(missing) => {
                        let thumb_reader = self.cache.thumbnails.clone();
                        let thumb_urls: Vec<(String, Url)> = self
                            .cache
                            .song_metadata
                            .read()
                            .fetch_existing(&missing)
                            .into_iter()
                            .map(|(id, s)| (id, Url::parse(&s.read().thumbnail).unwrap()))
                            .collect();

                        Cm::perform(
                            async move {
                                let thumbnails = get_images(thumb_reader, thumb_urls).await;

                                thumbnails.into_iter().collect()
                            },
                            |map| YtmrsMsg::ImagesFetched { map, missing: None },
                        )
                    }
                }
            }
            YtmrsMsg::SongsFetched {
                map,
                get_existing_thumbnails,
            } => {
                let keys: HashSet<String> = map.keys().cloned().collect();
                {
                    let mut lock = self.cache.song_metadata.write();
                    lock.items_mut().extend(map);
                }
                match get_existing_thumbnails {
                    false => Cm::none(),
                    true => self.download_images_for_ids(keys),
                }
            }
            YtmrsMsg::SoundsFetched { map, play } => {
                println!["Sounds fetched."];

                let map: HashMap<_, _> = map
                    .into_iter()
                    .map(|(k, v)| (k.clone(), BasicSoundData::from((k, v))))
                    .to_rwmap();

                self.cache.sounds.items_mut().extend(map.clone());

                if let Some(k) = play {
                    if map.contains_key(&k) {
                        let sound = map[&k].clone();
                        let sound = sound.read();
                        self.play(SoundData::from(sound.clone()));

                        return self.set_background(k);
                    }
                }

                Cm::none()
            }
            YtmrsMsg::DownloadSong(s, play) => self.download_song(s, play),
            YtmrsMsg::SongDownloaded { song, play } => {
                if let Some(recdown) = song.requested_downloads {
                    let recdown = recdown[0].clone();
                    let filepath = PathBuf::from(recdown.filepath);
                    let id = song.id.clone();
                    let id2 = id.clone();
                    let reader = self.cache.sounds.reader.clone();
                    Cm::perform(
                        async move {
                            let data = read_file(&filepath).await;

                            match data {
                                Ok(data) => {
                                    let file_data = FileData::new(id2.clone(), data);
                                    {
                                        println![
                                            "{:?}",
                                            reader.extend(vec![&file_data], true).await
                                        ];
                                        println!["531 Extended."];
                                    }
                                    if play {
                                        println!["Creating sound from bytes..."];
                                        let bsd =
                                            BasicSoundData::from((id2, file_data.into_data()));
                                        println!["Created sound from bytes."];

                                        // Delete the original file
                                        println!["Deleting {:?}...", filepath];
                                        match async_std::fs::remove_file(&filepath).await {
                                            Ok(_) => {
                                                println!["Deleted {:?}.", filepath];
                                            }
                                            Err(e) => {
                                                println!["Error deleting {:?}: {:?}", filepath, e];
                                            }
                                        }

                                        Some(Box::new(bsd))
                                    } else {
                                        None
                                    }
                                }
                                Err(e) => {
                                    println!["{:?}", e];
                                    None
                                }
                            }
                        },
                        move |data| match data {
                            Some(data) => YtmrsMsg::SongDownloadFinished { id, data },
                            None => YtmrsMsg::Null,
                        },
                    )
                } else {
                    // Add the song to the filecache
                    let map = [(song.id.clone(), song.clone())].to_rwmap();
                    let mut metadata = self.cache.song_metadata.write();
                    metadata.items_mut().extend(map);

                    let reader = metadata.reader.clone();

                    Cm::perform(
                        async move {
                            println![
                                "Adding song to cache: {:?}",
                                reader.extend(Vec::from([song]), true).await
                            ];
                        },
                        |_| YtmrsMsg::Null,
                    )
                }
            }
            YtmrsMsg::SongDownloadFinished { id, data } => {
                self.play(SoundData::from(*data));
                self.set_background(id)
            }

            YtmrsMsg::SetNewBackground(_, _) => Cm::none(),
            YtmrsMsg::Null => Cm::none(),
        }
    }

    fn download_images_for_ids(&self, ids: HashSet<String>) -> Cm<YtmrsMsg> {
        // get existing songs which already have thumbnails
        let songs: HashMap<_, _> = self
            .cache
            .song_metadata
            .read()
            .fetch_existing(&ids)
            .into_iter()
            .filter_map(|(id, song)| {
                let s = song.read();
                match s.thumbnail_handle {
                    Some(_) => None,
                    None => Some((id, song.clone())),
                }
            })
            .collect();
        let song_ids: HashSet<String> = songs.keys().cloned().collect();

        let missing_ids: HashSet<String> = ids.difference(&song_ids).cloned().collect();

        let thumb_reader = self.cache.thumbnails.clone();

        Cm::perform(
            async move {
                let items = thumb_reader.read_filter(&missing_ids).await.unwrap();
                let ids: Vec<_> = items.iter().map(|(k, _)| k.clone()).collect();
                let futures = items.into_iter().map(|(_, f)| f);
                let items: HashMap<String, _> =
                    ids.into_iter().zip(join_all(futures).await).collect();

                items
                    .into_values()
                    .map(|x| (x.1.id().clone(), Handle::from_path(x.1.into_data())))
                    .collect()
            },
            move |map: HashMap<String, Handle>| YtmrsMsg::ImagesFetched {
                missing: {
                    let collected_ids: HashSet<_> = map.keys().cloned().collect();

                    let actually_missing: HashSet<String> =
                        ids.difference(&collected_ids).cloned().collect();
                    match actually_missing.len() {
                        0 => None,
                        _ => Some(actually_missing),
                    }
                },
                map,
            },
        )
    }

    fn all_used_keys(&self) -> HashSet<String> {
        let from_search: HashSet<&String> = self.search.used_keys().into_iter().collect();
        let from_constr: HashSet<&String> = self
            .settings
            .playlist
            .constructor
            .all_song_keys_rec()
            .collect();

        from_search.union(&from_constr).cloned().cloned().collect()
    }

    /// Handles zones
    fn handle_zones(
        &mut self,
        key: String,
        zones: Vec<(iced::advanced::widget::Id, iced::Rectangle)>,
    ) {
        let top = &mut self.settings.playlist.constructor;
        println!["KEY: {:?}", key];

        let targets = if let Some(v) = self.search.selected_keys() {
            v
        } else {
            vec![&key]
        };

        if let Some((id, _)) = zones.iter().rev().find(|(id, _)| top.item_has_id(id)) {
            println!["Target: {:#?}", id];

            let mut path = top.path_to_id(id).unwrap();
            println!["{:?}", path];

            let mut idx = path.pop().unwrap_or(0);
            for item in targets
                .into_iter()
                .map(|k| ConstructorItem::from(k.clone()))
            {
                path.push(idx);
                top.push_to_path(VecDeque::from(path.clone()), item);
                path.pop();
                idx += 1;
            }
        } else if let Some((id, _)) = zones.last() {
            if *id == WId::new("base_drop_target") {
                let mut idx = top.list.len();

                for item in targets
                    .into_iter()
                    .map(|k| ConstructorItem::from(k.clone()))
                {
                    top.push_to_path(VecDeque::from([idx]), item);
                    idx += 1;
                }
            }
        }
    }

    fn push_image_handles(&mut self, map: HashMap<String, Handle>) {
        let used_keys = self.all_used_keys();
        let lock = self.cache.song_metadata.write();
        let mut song_cache = lock.fetch_existing(&used_keys);
        for (key, song) in song_cache.iter_mut() {
            let mut lock = song.write();
            if lock.thumbnail_handle.is_none() {
                if let Some(handle) = map.get(key) {
                    lock.thumbnail_handle = Some(handle.clone());
                }
            }
        }
    }

    /// Restarts the current song if it is playing after 2s, otherwise plays the previous song
    fn rewind(&mut self) -> Cm<YtmrsMsg> {
        match self.audio_manager.elapsed() {
            Some(elapsed) => {
                if elapsed > 2.0 {
                    self.audio_manager.seek_to_start();
                    Cm::none()
                } else {
                    self.play_previous_song()
                }
            }
            None => Cm::none(),
        }
    }

    fn play_previous_song(&mut self) -> Cm<YtmrsMsg> {
        if let Some(state) = &mut self.player_state {
            match state.tracker.move_back() {
                song_operations::BackResult::Rewound => {
                    self.audio_manager.seek_to_start();
                    self.audio_manager.play();
                    self.audio_tracker.paused = false;
                    self.tickers.playing_status.0 = true;
                    Cm::none()
                }
                song_operations::BackResult::Current => {
                    let path: VecDeque<usize> = state.tracker.get_current().collect();
                    self.play_at_path(path)
                }
            }
        } else {
            Cm::none()
        }
    }

    fn play_next_song(&mut self) -> Cm<YtmrsMsg> {
        if let Some(state) = &mut self.player_state {
            match state.tracker.move_next() {
                song_operations::NextResult::Current => {
                    let path: VecDeque<usize> = state.tracker.get_current().collect();
                    self.play_at_path(path)
                }
                song_operations::NextResult::Ended => {
                    // Pause
                    Cm::none()
                }
            }
        } else {
            Cm::none()
        }
    }

    fn set_background(&self, key: String) -> Cm<YtmrsMsg> {
        let hashset = HashSet::from([key.clone()]);
        let reader = self.cache.thumbnails.clone();
        let key2 = key.clone();
        Cm::perform(
            async move {
                let hashset = hashset;
                let thumbnails: HashMap<_, _> = join_all(reader.read_from_ids(&hashset).await)
                    .await
                    .into_iter()
                    .collect();

                match thumbnails.is_empty() {
                    true => None,
                    false => {
                        let data = {
                            let thumbnail = thumbnails[&key].read();
                            thumbnail.clone().into_data().clone()
                        };
                        Some(BasicYtmrsScheme::from_handle(Handle::from_path(data)).await)
                    }
                }
            },
            |ms| match ms {
                Some(scheme) => YtmrsMsg::SetNewBackground(key2, scheme),
                None => YtmrsMsg::Null,
            },
        )
    }

    /// Generate the song tracker when a song is clicked
    fn song_clicked(&mut self, wid: WId) -> Cm<YtmrsMsg> {
        let path = self.settings.playlist.constructor.path_to_id(&wid).unwrap();

        println!["Given path: {:?}", path];
        let song_op = self.settings.playlist.constructor.build();
        println!["Song op: {:?}", song_op];
        println!["Is valid: {:?}", song_op.is_valid()];
        println!["Loop type: {:?}", song_op.loop_type()];
        if !song_op.is_valid() {
            return Cm::none();
        }
        let tracker = SongOpTracker::from_song_op(&song_op, path.into());
        let generated_path: VecDeque<usize> = tracker.get_current().collect();
        self.player_state = Some(PlayerState { tracker });
        self.play_at_path(generated_path)
    }

    fn play_at_path(&mut self, pth: VecDeque<usize>) -> Cm<YtmrsMsg> {
        let item = self.settings.playlist.constructor.item_at_path(pth);
        if let Some(ConstructorItem::Song(k, _)) = item {
            println!["Estimated item at path: {:?}", item];

            let hashset = HashSet::from([k.clone()]);

            let key = k.clone();

            let sounds = self.cache.sounds.fetch_existing(&hashset);

            match sounds.is_empty() {
                false => {
                    // Song exists in the cache, just play it
                    let item = sounds[&key].read();
                    self.play(SoundData::from(item.clone()));

                    self.set_background(key)
                }
                true => {
                    // Song does not exist in the cache, add it to the cache and play it
                    self.fetch_song(key, true)
                }
            }
        } else {
            Cm::none()
        }
    }

    fn fetch_song(&self, id: String, play: bool) -> Cm<YtmrsMsg> {
        let set = HashSet::from([id.clone()]);
        let reader = self.cache.sounds.reader.clone();

        Cm::perform(
            async move {
                let futures = reader.read_from_ids(&set).await;
                // there should only be one future in the list
                let future = futures.into_iter().take(1).next();
                match future {
                    Some(item) => {
                        let actual_item = {
                            let item = item.await;
                            let l = item.1.read();
                            (
                                item.0.clone(),
                                BasicSoundData::from((item.0, l.clone().into_data())),
                            )
                        };
                        let arc = Some(Arc::new([actual_item].to_rwmap()));
                        println!["Created song arc."];
                        arc
                    }
                    None => None,
                }
            },
            move |map| match map {
                Some(map) => YtmrsMsg::SoundsFetched {
                    map: {
                        map.iter()
                            .map(|(k, v)| {
                                let data = {
                                    let item = v.read();
                                    item.data().clone()
                                };

                                (k.clone(), data)
                            })
                            .collect()
                    },
                    play: play.then_some(id),
                },
                None => YtmrsMsg::DownloadSong(id, true),
            },
        )
    }

    fn download_song(&self, id: String, play: bool) -> Cm<YtmrsMsg> {
        let metadata = self.cache.song_metadata.read();
        let songs = metadata.fetch_existing(&HashSet::from([id.clone()]));
        if songs.is_empty() {
            return Cm::none();
        }
        let song = songs[&id].write();
        let backend = self.backend_handler.lock();
        let url = song.webpage_url.clone();

        Cm::perform(
            backend.request_download_song(url).unwrap(),
            move |result| match result {
                Ok(s) => {
                    println!["{:?}", s];
                    let song: Song = serde_json::from_str(&s).unwrap();
                    YtmrsMsg::SongDownloaded { song, play }
                }
                Err(e) => {
                    println!["{:?}", e];
                    YtmrsMsg::Null
                }
            },
        )
    }

    fn play(&mut self, sd: SoundData) {
        println!["Playing sound."];
        self.audio_manager.play_once(sd);
        self.audio_manager
            .set_volume(self.settings.user.volume as f64);
        self.audio_tracker.update_from_manager(&self.audio_manager);
        self.tickers.playing_status.0 = true;
        println!["Played sound."];
    }
}
