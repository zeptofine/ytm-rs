use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
    time,
};

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
use kira::sound::static_sound::StaticSoundData;
use parking_lot::{Mutex, RwLock};
use reqwest::Url;

use crate::{
    audio::{AudioProgressTracker, TrackerMsg, YTMRSAudioManager},
    backend_handler::{BackendHandler, BackendLaunchStatus, RequestResult},
    caching::{
        readers::{
            folder_based_reader::read_file, CacheReader, FileData, FolderBasedReader,
            LazyFolderBasedReader, LineBasedReader,
        },
        BasicSoundData, BufferedCache, FolderCache, NDJsonCache, RwMap, SoundData,
    },
    playlist::PlaylistMessage,
    response_types::{YTResponseError, YTResponseType},
    search_window::{SWMessage, SearchType, SearchWindow},
    settings::{project_cache_dir, project_data_dir, YTMRSettings},
    song::Song,
    song_operations::{
        ConstructorItem, OperationTracker, SongOpTracker, TreeDirected, UpdateResult,
    },
    styling::{BasicYtmrsScheme, FullYtmrsScheme},
    thumbnails::get_images,
    user_input::UserInputs,
};

pub type RwArc<T> = Arc<RwLock<T>>;

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

fn song_metadata_path() -> PathBuf {
    let mut path = project_data_dir();
    path.push("songs.ndjson");
    path
}

fn song_audio_path() -> PathBuf {
    let mut path = project_cache_dir();
    path.push("songs");
    path
}

fn thumbnails_directory() -> PathBuf {
    let mut path = project_cache_dir();
    path.push("thumbs");
    path
}

// fn playlists_directory() -> PathBuf {
//     let mut path = project_data_dir();
//     path.push("playlists");
//     path
// }

#[derive(Debug)]
pub struct YtmrsCache {
    pub song_metadata: RwArc<NDJsonCache<Song>>,
    pub sounds: FolderCache<BasicSoundData>,
    pub thumbnails: LazyFolderBasedReader,
}

impl Default for YtmrsCache {
    fn default() -> Self {
        Self {
            song_metadata: Arc::new(RwLock::new(NDJsonCache::new(LineBasedReader::new(
                song_metadata_path(),
            )))),
            sounds: FolderCache::new(FolderBasedReader::new(song_audio_path())),
            thumbnails: LazyFolderBasedReader::new(thumbnails_directory()),
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

    RequestRecieved(RequestResult),
    RequestParsed(Box<YTResponseType>),
    RequestParseFailure(YTResponseError),

    SetNewBackground(String, BasicYtmrsScheme),
    SearchWindowMessage(SWMessage),
    PlaylistMsg(PlaylistMessage),
    AudioTrackerMessage(TrackerMsg),

    KeysChanged(keyboard::Key, keyboard::Modifiers),

    DownloadSong(String, bool),
    ImagesFetched {
        map: HashMap<String, Handle>,
    },
    SongsFetched {
        map: RwMap<String, Song>,
    },
    SoundsFetched {
        map: HashMap<String, StaticSoundData>,
        play: Option<String>,
    },
    SongDownloaded {
        song: Song,
        play: bool,
    },
    SongDownloadFinished {
        id: String,
        data: Box<BasicSoundData>,
    },

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

        Cm::batch([
            Cm::perform(
                async move {
                    futures::future::join_all(metadata_reader.read_from_ids(&keys).await)
                        .await
                        .into_iter()
                        .collect()
                },
                |map| YtmrsMsg::SongsFetched { map },
            ),
            // Cm::perform(
            //     async move {
            //         let items =
            //             futures::future::join_all(thumb_reader.read_filter(&keys2).await.unwrap())
            //                 .await;
            //         println!["ITEMS: {:?}", items];
            //         items
            //             .into_iter()
            //             .map(|x| (x.0, Handle::from_path(x.1.into_data())))
            //             .collect()
            //     },
            //     |map| YtmrsMsg::ImagesFetched { map },
            // ),
        ])
    }

    pub fn prepare_to_save(&mut self) {}

    pub fn subscription(&self) -> Subscription<YtmrsMsg> {
        Subscription::batch([
            self.tickers.subscription(),
            // Handle tracking modifiers
            keyboard::on_key_press(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
            keyboard::on_key_release(|k, m| Some(YtmrsMsg::KeysChanged(k, m))),
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
        match message {
            YtmrsMsg::KeysChanged(_, m) => {
                println!["{:?}", m];
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
                Ok(s) => Cm::perform(Ytmrs::parse_request(s), |result| match result {
                    Ok(r) => YtmrsMsg::RequestParsed(Box::new(r)),
                    Err(e) => YtmrsMsg::RequestParseFailure(e),
                }),
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
                        .into_iter()
                        .map(|entry| Song {
                            id: entry.id,
                            title: entry.title,
                            channel: entry.channel.clone(),
                            view_count: entry.view_count,
                            webpage_url: entry.url,
                            duration: entry.duration as f64,
                            thumbnail: entry.thumbnails[0].url.clone(),
                            artists: Some(vec![entry.channel.clone()]),
                            ..Default::default()
                        })
                        .collect();

                    let keys: Vec<_> = songs.iter().map(|s| s.id.clone()).collect();

                    self.search.search_type = SearchType::new_tab(keys.clone());

                    let map = songs
                        .iter()
                        .map(|s| (s.id.clone(), Arc::new(RwLock::new(s.clone()))))
                        .collect::<RwMap<_, _>>();

                    let thumb_urls: HashMap<_, _> = songs
                        .iter()
                        .map(|s| (s.id.clone(), Url::parse(&s.thumbnail).unwrap()))
                        .collect();

                    // Add the songs to the file cache
                    let new_cache = self.cache.song_metadata.clone();
                    let song_reader = new_cache.write();
                    let song_reader = song_reader.clone();
                    let thumb_reader = self.cache.thumbnails.clone();
                    let thumb_reader = thumb_reader.clone();

                    Cm::batch([
                        Cm::perform(
                            async move {
                                println!(
                                    "extend songs {:?}",
                                    song_reader.reader.extend(songs, true).await
                                );
                            },
                            move |_| YtmrsMsg::SongsFetched { map },
                        ),
                        Cm::perform(
                            async move {
                                let thumbnails =
                                    get_images(thumb_reader, thumb_urls.into_iter().collect())
                                        .await;

                                thumbnails.into_iter().collect()
                            },
                            move |map| YtmrsMsg::ImagesFetched { map },
                        ),
                    ])
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
            YtmrsMsg::HandleZones(song_key, zones) => {
                if zones.is_empty() {
                    return Cm::none();
                }
                self.handle_zones(song_key, zones);

                Cm::none()
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
                                    let from_path =
                                        self.settings.playlist.constructor.path_to_id(&from);
                                    let to_path =
                                        self.settings.playlist.constructor.path_to_id(&to);
                                    if from_path.is_none() || to_path.is_none() {
                                        return Cm::none();
                                    }
                                    let from_path = from_path.unwrap();
                                    let to_path = to_path.unwrap();

                                    self.so_move(from_path, to_path);

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
                TrackerMsg::Next => todo!(),
                TrackerMsg::Previous => todo!(),
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

            YtmrsMsg::ImagesFetched { map } => {
                self.push_image_handles(map);
                Cm::none()
            }
            YtmrsMsg::SongsFetched { map } => {
                {
                    let mut lock = self.cache.song_metadata.write();
                    lock.push_cache(map);
                }
                Cm::none()
            }
            YtmrsMsg::SoundsFetched { map, play } => {
                println!["Sounds fetched."];
                let map: HashMap<_, _> = map
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            Arc::new(RwLock::new(BasicSoundData::from((k, v)))),
                        )
                    })
                    .collect();

                self.cache.sounds.push_cache(map.clone());

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

            YtmrsMsg::DownloadSong(s, play) => {
                let metadata = self.cache.song_metadata.read();
                let songs = metadata.fetch_existing(&HashSet::from([s.clone()]));
                if songs.is_empty() {
                    return Cm::none();
                }
                let song = songs[&s].read();
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
            YtmrsMsg::SongDownloaded { song, play } => {
                if let Some(recdown) = song.requested_downloads {
                    let recdown = recdown[0].clone();
                    let filepath = PathBuf::from(recdown.filepath);
                    let id = song.id.clone();
                    let id2 = id.clone();
                    let reader = self.cache.sounds.reader.clone();
                    Cm::perform(
                        async move {
                            let data = read_file(filepath).await;

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
                    Cm::none()
                }
            }
            YtmrsMsg::SongDownloadFinished { id, data } => {
                self.play(SoundData::from(*data));

                self.set_background(id)
            }
            YtmrsMsg::CacheTick => {
                let used_meta_keys: HashSet<String> = self.all_used_keys();
                {
                    let mut metadata = self.cache.song_metadata.write();
                    let available_keys: HashSet<String> = metadata.keys().cloned().collect();

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

            YtmrsMsg::Null => Cm::none(),
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

    /// Handles zones
    fn handle_zones(
        &mut self,
        key: String,
        zones: Vec<(iced::advanced::widget::Id, iced::Rectangle)>,
    ) {
        let top = &mut self.settings.playlist.constructor;

        if let Some((id, _)) = zones.iter().rev().find(|(id, _)| top.item_has_id(id)) {
            println!["Target: {:#?}", id];

            let mut path = top.path_to_id(id).unwrap();
            println!["{:?}", path];
            if let Some(v) = self.search.selected_keys() {
                let mut idx = path.pop().unwrap_or(0);

                for item in v.into_iter().map(|k| ConstructorItem::from(k.clone())) {
                    path.push(idx);
                    top.push_to_path(VecDeque::from(path.clone()), item);
                    path.pop();
                    idx += 1;
                }
            }
        } else if let Some((id, _)) = zones.last() {
            if *id == WId::new("base_drop_target") {
                top.push_to_path(VecDeque::new(), key.into());
            }
        }
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
        let item = self
            .settings
            .playlist
            .constructor
            .item_at_path(generated_path);
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
                    let reader = self.cache.sounds.reader.clone();

                    let mut cms = Vec::with_capacity(2);

                    cms.push(Cm::perform(
                        async move {
                            let futures = reader.read_from_ids(&hashset).await;
                            // there should only be one future in the list
                            let future = futures.into_iter().take(1).next();
                            match future {
                                Some(item) => {
                                    let actual_item: (String, Arc<RwLock<BasicSoundData>>) = {
                                        let item = item.await;
                                        let l = item.1.read();
                                        (
                                            item.0.clone(),
                                            Arc::new(RwLock::new(BasicSoundData::from((
                                                item.0,
                                                l.clone().into_data(),
                                            )))),
                                        )
                                    };
                                    let arc = Some(Arc::new(HashMap::from([actual_item])));
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
                                play: Some(key),
                            },
                            None => YtmrsMsg::DownloadSong(key, true),
                        },
                    ));

                    Cm::batch(cms)
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
                let thumbnails: HashMap<_, _> =
                    futures::future::join_all(reader.read_from_ids(&hashset).await)
                        .await
                        .into_iter()
                        .collect();
                println!["THUMBNAILS: {:?}", thumbnails];

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

    fn play(&mut self, sd: SoundData) {
        println!["Playing sound."];
        self.audio_manager.play_once(sd);
        self.audio_manager
            .set_volume(self.settings.user.volume as f64);
        self.audio_tracker.update_from_manager(&self.audio_manager);
        self.tickers.playing_status.0 = true;
        println!["Played sound."];
    }

    async fn parse_request(response: String) -> Result<YTResponseType, YTResponseError> {
        YTResponseType::new(response)
    }
}
