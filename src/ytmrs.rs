use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
    time,
};

use futures::future::join_all;
use iced::{
    advanced::widget::Id as WId,
    alignment::Horizontal,
    keyboard,
    widget::{
        column,
        container::{Container, Id as CId},
        image::Handle,
        row, Space,
    },
    Element, Length, Subscription, Task,
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
    song::Song,
    song_operations::{
        self, ConstructorItem, OperationTracker, SongOpTracker, TreeDirected, UpdateResult,
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

    SongsFetched {
        map: RwMap<String, Song>,
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

    pub fn load(&mut self) -> Task<YtmrsMsg> {
        // Add the cache to required places
        self.search.cache = Some(Arc::clone(&self.cache.song_metadata));

        let mut backend = self.backend_handler.lock();

        if let BackendLaunchStatus::Unknown = backend.status {
            *backend = BackendHandler::load(None);
        }
        let cache = self.cache.song_metadata.write();
        let metadata_reader = cache.reader.clone();
        let keys = self.all_used_keys();

        Task::perform(
            async move {
                join_all(metadata_reader.read_from_ids(&keys).await)
                    .await
                    .into_iter()
                    .collect()
            },
            |map| YtmrsMsg::SongsFetched { map },
        )
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::batch([
            self.tickers.subscription(),
            // Handle tracking modifiers
            // keyboard::on_key_press(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
            // keyboard::on_key_release(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
            // Checking when songs finish
            self.audio_manager.subscription().map(YtmrsMsg::ManagerMsg),
        ])
    }

    pub fn view(&self, scheme: FullYtmrsScheme) -> Element<YtmrsMsg> {
        let backend = self.backend_handler.lock();
        let backend_status = backend.status.as_string();

        let search = self.search.view(&scheme).map(YtmrsMsg::SearchWindowMessage);

        let current_playlist = {
            let map = self.cache.song_metadata.read();
            self.settings
                .playlist
                .view(map.items(), &scheme)
                .map(YtmrsMsg::PlaylistMsg)
        };

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
            .align_x(Horizontal::Center),
        )
    }

    pub fn parse_search_request(&mut self, response_type: YTResponseType) -> Task<YtmrsMsg> {
        match response_type {
            YTResponseType::Song(_song) => {
                println!["Request is a song"];
                Task::none()
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

                Task::batch([Task::perform(
                    async move {
                        println!("extend songs {:?}", reader.extend(songs, true).await);
                        map
                    },
                    move |map| YtmrsMsg::SongsFetched { map },
                )])
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

                    Task::perform(
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
                    Task::none()
                }
            }
        }
    }

    pub fn update(&mut self, message: YtmrsMsg) -> Task<YtmrsMsg> {
        match message {
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
                Task::none()
            }
            YtmrsMsg::BackendStatusTick => {
                self.backend_handler.lock().poll();
                Task::none()
            }
            YtmrsMsg::BackendStatusPollSuccess => Task::none(),
            YtmrsMsg::BackendStatusPollFailure(e) => {
                println!["Polling failure: {:?}", e];
                let mut backend = self.backend_handler.lock();
                backend.status = BackendLaunchStatus::Unknown;
                todo!()
            }
            YtmrsMsg::PlayingStatusTick => {
                self.audio_tracker.update_from_manager(&self.audio_manager);
                Task::none()
            }

            YtmrsMsg::RequestRecieved(response) => match response {
                Ok(s) => {
                    let response_type = YTResponseType::new(s);
                    match response_type {
                        Ok(response_type) => self.parse_search_request(response_type),
                        Err(e) => {
                            println!["Error: {:?}", e];
                            Task::none()
                        }
                    }
                }
                _ => {
                    println!["{:?}", response];
                    Task::none()
                }
            },

            // * Searching
            YtmrsMsg::SearchedKeysReceived { existing, missing } => {
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

                Task::batch([Task::perform(request_command, |songs| {
                    let map: HashMap<String, _> =
                        songs.into_iter().map(|s| (s.id().clone(), s)).to_rwmap();

                    YtmrsMsg::SongsFetched { map }
                })])
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
                    Task::none()
                }
            }
            YtmrsMsg::SearchWindowMessage(msg) => {
                match msg {
                    SWMessage::SearchQuerySubmitted => {
                        // Check if URL is valid
                        match Url::parse(&self.search.query) {
                            Ok(_) => Task::perform(
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
                                Task::perform(
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
                        .map(YtmrsMsg::SearchWindowMessage),
                }
            }
            YtmrsMsg::PlaylistMsg(msg) => {
                match msg {
                    PlaylistMessage::ConstructorMessage(msg) => {
                        match self.settings.playlist.constructor.update(msg) {
                            Some(msg) => match msg {
                                UpdateResult::Task(cm) => cm.map(|m| {
                                    YtmrsMsg::PlaylistMsg(PlaylistMessage::ConstructorMessage(m))
                                }),
                                UpdateResult::SongClicked(wid) => self.song_clicked(wid),
                                UpdateResult::Move(from, to) => {
                                    // Remove item at `from` and place it to `to`
                                    println!["MOVE FROM {:?} TO {:?}", from, to];
                                    let from_path =
                                        self.settings.playlist.constructor.path_to_id(&from);

                                    if from_path.is_none() {
                                        return Task::none();
                                    }
                                    let from_path = from_path.unwrap();
                                    println!["FROM:{:?}", from_path];

                                    let item =
                                        self.settings.playlist.constructor.pop_path(&from_path);

                                    if item.is_none() {
                                        return Task::none();
                                    }
                                    let item = item.unwrap();

                                    let to_path =
                                        self.settings.playlist.constructor.path_to_id(&to);

                                    if to_path.is_none() {
                                        return Task::none();
                                    }

                                    let to_path = to_path.unwrap();
                                    println!["TO:{:?}", to_path];

                                    self.settings
                                        .playlist
                                        .constructor
                                        .push_to_path(&to_path, item);

                                    Task::none()
                                }
                            },
                            None => Task::none(),
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
                    Task::none()
                }
                TrackerMsg::Play => {
                    self.audio_manager.play();
                    self.audio_tracker.paused = false;
                    self.tickers.playing_status.0 = true;
                    Task::none()
                }
                TrackerMsg::Next => self.play_next_song(),
                TrackerMsg::Previous => self.rewind(),
                TrackerMsg::UpdateVolume(v) => {
                    println!["{:?}", v];
                    let float_vol = v / 1000_f64;
                    self.audio_manager.set_volume(float_vol);
                    self.audio_tracker.volume = *v;
                    self.settings.user.volume = float_vol as f32;
                    Task::none()
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
            YtmrsMsg::SongsFetched { map } => {
                let keys: HashSet<String> = map.keys().cloned().collect();
                {
                    let mut lock = self.cache.song_metadata.write();
                    lock.items_mut().extend(map);
                }

                Task::none()
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
                    }
                }

                Task::none()
            }
            YtmrsMsg::DownloadSong(s, play) => self.download_song(s, play),
            YtmrsMsg::SongDownloaded { song, play } => {
                if let Some(recdown) = song.requested_downloads {
                    let recdown = recdown[0].clone();
                    let filepath = PathBuf::from(recdown.filepath);
                    let id = song.id.clone();
                    let id2 = id.clone();
                    let reader = self.cache.sounds.reader.clone();
                    Task::perform(
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
                            Some(data) => YtmrsMsg::SongDownloadFinished {
                                id: id.clone(),
                                data,
                            },
                            None => YtmrsMsg::Null,
                        },
                    )
                } else {
                    // Add the song to the filecache
                    let map = [(song.id.clone(), song.clone())].to_rwmap();
                    let mut metadata = self.cache.song_metadata.write();
                    metadata.items_mut().extend(map);

                    let reader = metadata.reader.clone();

                    Task::perform(
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
                Task::none()
            }

            YtmrsMsg::SetNewBackground(_, _) => Task::none(),
            YtmrsMsg::Null => Task::none(),
        }
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

    /// Restarts the current song if it is playing after 2s, otherwise plays the previous song
    fn rewind(&mut self) -> Task<YtmrsMsg> {
        match self.audio_manager.elapsed() {
            Some(elapsed) => {
                if elapsed > 2.0 {
                    self.audio_manager.seek_to_start();
                    Task::none()
                } else {
                    self.play_previous_song()
                }
            }
            None => Task::none(),
        }
    }

    fn play_previous_song(&mut self) -> Task<YtmrsMsg> {
        if let Some(state) = &mut self.player_state {
            match state.tracker.move_back() {
                song_operations::BackResult::Rewound => {
                    self.audio_manager.seek_to_start();
                    self.audio_manager.play();
                    self.audio_tracker.paused = false;
                    self.tickers.playing_status.0 = true;
                    Task::none()
                }
                song_operations::BackResult::Current => {
                    let path: Vec<_> = state.tracker.get_current().collect();
                    self.play_at_path(&path)
                }
            }
        } else {
            Task::none()
        }
    }

    fn play_next_song(&mut self) -> Task<YtmrsMsg> {
        if let Some(state) = &mut self.player_state {
            match state.tracker.move_next() {
                song_operations::NextResult::Current => {
                    let path: Vec<_> = state.tracker.get_current().collect();
                    self.play_at_path(&path)
                }
                song_operations::NextResult::Ended => {
                    // Pause
                    Task::none()
                }
            }
        } else {
            Task::none()
        }
    }
    /// Generate the song tracker when a song is clicked
    fn song_clicked(&mut self, wid: WId) -> Task<YtmrsMsg> {
        let path = self.settings.playlist.constructor.path_to_id(&wid).unwrap();

        println!["Given path: {:?}", path];
        let song_op = self.settings.playlist.constructor.build();
        println!["Song op: {:#?}", song_op];
        println!["Is valid: {:?}", song_op.is_valid()];
        println!["Loop type: {:?}", song_op.loop_type()];
        if !song_op.is_valid() {
            return Task::none();
        }
        let tracker = SongOpTracker::from_song_op(&song_op, &path);
        let generated_path: Vec<_> = tracker.get_current().collect();
        self.player_state = Some(PlayerState { tracker });
        self.play_at_path(&generated_path)
    }

    fn play_at_path(&mut self, pth: &[usize]) -> Task<YtmrsMsg> {
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

                    Task::none()
                }
                true => {
                    // Song does not exist in the cache, add it to the cache and play it
                    self.fetch_song(key, true)
                }
            }
        } else {
            Task::none()
        }
    }

    fn fetch_song(&self, id: String, play: bool) -> Task<YtmrsMsg> {
        let set = HashSet::from([id.clone()]);
        let reader = self.cache.sounds.reader.clone();

        Task::perform(
            async move {
                let futures = reader.read_from_ids(&set).await;
                // there should only be one future in the list
                let future = futures.into_iter().take(1).next();

                (
                    id,
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
                    },
                )
            },
            move |(id, map)| match map {
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

    fn download_song(&self, id: String, play: bool) -> Task<YtmrsMsg> {
        let metadata = self.cache.song_metadata.read();
        let songs = metadata.fetch_existing(&HashSet::from([id.clone()]));
        if songs.is_empty() {
            return Task::none();
        }
        let song = songs[&id].write();
        let backend = self.backend_handler.lock();
        let url = song.webpage_url.clone();

        Task::perform(
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
