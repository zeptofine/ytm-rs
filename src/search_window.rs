use std::collections::{HashMap, HashSet};

use iced::{
    alignment::Horizontal,
    keyboard::Modifiers,
    widget::{column, scrollable, text, text_input, Column, Container},
    Element, Length, Task,
};
use serde::{Deserialize, Serialize};

use crate::{
    caching::{BufferedCache, NDJsonCache, RwArc, RwMap},
    response_types::{YTIEKey, YTSearchEntry},
    song::{Song, SongData},
    styling::FullYtmrsScheme,
    user_input::SelectionMode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchEntry {
    Song {
        id: String,
        title: Option<String>,
        url: String,
    },
    Tab {
        id: String,
        title: Option<String>,
        url: String,
    },
}

#[derive(Debug)]
pub struct InvalidKind;

impl SearchEntry {
    pub fn new(entry: YTSearchEntry) -> Result<Self, InvalidKind> {
        match entry.ie_key {
            YTIEKey::Youtube => Ok(Self::Song {
                id: entry.id,
                title: entry.title,
                url: entry.url,
            }),
            YTIEKey::YoutubeTab => Ok(Self::Tab {
                id: entry.id,
                title: entry.title,
                url: entry.url,
            }),
            YTIEKey::YoutubeMusicSearchURL => Err(InvalidKind),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    Song(String),
    Tab(Vec<String>, #[serde(skip)] SelectionMode),
    Search(Vec<SearchEntry>),
}

impl SearchType {
    pub fn new_tab(songs: Vec<String>) -> Self {
        Self::Tab(songs, SelectionMode::None)
    }

    pub fn selected_keys(&self) -> Option<Vec<&String>> {
        match self {
            Self::Song(s) => Some(vec![s]),
            Self::Tab(s, mode) => match mode {
                SelectionMode::None => None,
                SelectionMode::Single(idx) => s.get(*idx).map(|k| vec![k]),
                SelectionMode::Multiple(v) => {
                    Some(v.iter().filter_map(|idx| s.get(*idx)).collect())
                }
                SelectionMode::Range { first: _, r } => {
                    Some(r.clone().filter_map(|idx| s.get(idx)).collect())
                }
            },
            Self::Search(_) => None,
        }
    }

    pub fn used_keys(&self) -> Vec<&String> {
        match self {
            SearchType::Song(ref song) => vec![song],
            SearchType::Tab(ref v, _) => v.iter().collect(),
            SearchType::Search(ref v) => v
                .iter()
                .filter_map(|e| match e {
                    SearchEntry::Song {
                        id,
                        title: _,
                        url: _,
                    } => Some(id),
                    SearchEntry::Tab {
                        id: _,
                        title: _,
                        url: _,
                    } => None,
                })
                .collect(),
        }
    }

    pub fn view(
        &self,
        scheme: &FullYtmrsScheme,
        cached_map: RwMap<String, Song>,
    ) -> Element<SWMessage> {
        match &self {
            SearchType::Song(_) => {
                todo!()
            }
            SearchType::Tab(v, mode) => {
                let songs = v.iter().enumerate().map(|(idx, key)| {
                    let selected = mode.contains(idx);
                    let style = scheme.song_appearance.update(selected);

                    Container::new(
                        Element::new(match cached_map.get(key) {
                            Some(songc) => {
                                let song = songc.read();
                                song.as_data().row(true, false)
                            }
                            None => SongData::mystery_with_title(key.clone()).row(true, false),
                        })
                        .map(move |_| SWMessage::SelectSong(idx)),
                    )
                    .style(move |_| style)
                    .into()
                });

                Element::new(
                    scrollable(
                        Container::new(Column::with_children(songs).width(Length::Fill))
                            .align_x(Horizontal::Left)
                            .max_width(400)
                            .padding(0),
                    )
                    .width(Length::Fill)
                    .style(scheme.scrollable_style.clone().update()),
                )
            }
            SearchType::Search(v) => {
                let items = v.iter().enumerate().map(|(idx, entry)| match entry {
                    SearchEntry::Song { id, title, url: _ } => {
                        Element::new(match cached_map.get(id) {
                            Some(song) => {
                                let song = song.read();
                                song.as_data().row(false, false)
                            }
                            None => {
                                SongData::mystery_with_title(title.clone().unwrap_or(id.clone()))
                                    .row(false, false)
                            }
                        })
                        .map(move |_| SWMessage::SelectSong(idx))
                    }

                    SearchEntry::Tab { id, title, url: _ } => {
                        Element::new(text(title.clone().unwrap_or(id.clone())))
                    }
                });

                Element::new(
                    scrollable(Column::with_children(items))
                        .width(Length::Fill)
                        .style(scheme.scrollable_style.clone().update()),
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SWMessage {
    SearchQueryChanged(String),
    SearchQuerySubmitted,
    SimpleSelectSong(usize),
    SelectSong(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWindow {
    pub query: String,
    pub search_type: SearchType,
    #[serde(skip)]
    pub cache: Option<RwArc<NDJsonCache<Song>>>,
}
impl Default for SearchWindow {
    fn default() -> Self {
        SearchWindow {
            query: String::new(),
            search_type: SearchType::Tab(vec![], SelectionMode::None),
            cache: None,
        }
    }
}
impl SearchWindow {
    pub fn used_keys(&self) -> Vec<&String> {
        self.search_type.used_keys()
    }

    pub fn selected_keys(&self) -> Option<Vec<&String>> {
        self.search_type.selected_keys()
    }

    pub fn view(&self, scheme: &FullYtmrsScheme) -> Element<SWMessage> {
        let keys: HashSet<String> = self.used_keys().into_iter().cloned().collect();

        let cached_map: HashMap<_, _> = match &self.cache {
            Some(lock) => {
                let c = lock.read();
                c.fetch_existing(&keys)
            }
            None => HashMap::new(),
        };

        let search_query = text_input("Enter query...", &self.query)
            .on_input(SWMessage::SearchQueryChanged)
            .on_submit(SWMessage::SearchQuerySubmitted);

        column![search_query, self.search_type.view(scheme, cached_map)].into()
    }

    pub fn update(&mut self, msg: SWMessage, mods: &Modifiers) -> Task<SWMessage> {
        match msg {
            SWMessage::SimpleSelectSong(idx) => {
                if let SearchType::Tab(_, ref mut mode) = self.search_type {
                    if let SelectionMode::None | SelectionMode::Single(_) = mode {
                        *mode = SelectionMode::Single(idx);
                    }
                }
                Task::none()
            }
            SWMessage::SelectSong(idx) => {
                if let SearchType::Tab(_, ref mut mode) = self.search_type {
                    *mode = mode.clone().update_selection(idx, mods);
                }
                Task::none()
            }

            // Handle dragndrop
            SWMessage::SearchQueryChanged(s) => {
                self.query = s;
                Task::none()
            }
            SWMessage::SearchQuerySubmitted => Task::none(),
        }
    }
}
