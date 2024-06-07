use std::collections::{HashMap, HashSet};

use iced::{
    alignment::Horizontal,
    keyboard::Modifiers,
    widget::{column, scrollable, text_input, Column, Container},
    Command as Cm, Element, Length,
};
use iced_drop::{droppable, zones_on_point};
use serde::{Deserialize, Serialize};

use crate::{
    caching::{BufferedCache, NDJsonCache, RwMap},
    song::{Song, SongData},
    styling::FullYtmrsScheme,
    user_input::SelectionMode,
    ytmrs::RwArc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    Song(String),
    Tab(Vec<String>, #[serde(skip)] SelectionMode),
    Search(Vec<String>),
}

impl SearchType {
    pub fn new_tab(songs: Vec<String>) -> Self {
        Self::Tab(songs, SelectionMode::None)
    }

    pub fn selected_keys(&self) -> Option<Vec<&String>> {
        match self {
            Self::Song(s) => Some(vec![&s]),
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
            Self::Search(_) => todo!(),
        }
    }

    pub fn used_keys(&self) -> Vec<&String> {
        match self {
            SearchType::Song(ref song) => vec![song],
            SearchType::Tab(ref v, _) | SearchType::Search(ref v) => v.iter().collect(),
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
                    droppable(
                        Container::new(
                            Element::new(match cached_map.get(key) {
                                Some(songc) => {
                                    let song = songc.read();
                                    song.as_data().row(true, false)
                                }
                                None => SongData::mystery_with_id(key.clone()).row(true, false),
                            })
                            .map(move |_| SWMessage::SelectSong(idx)),
                        )
                        .style(move |_| style),
                    )
                    .on_drop(move |pt, rec| SWMessage::Drop(key.clone(), pt, rec))
                    .on_click(SWMessage::SimpleSelectSong(idx))
                    .on_single_click(SWMessage::SelectSong(idx))
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
            SearchType::Search(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SWMessage {
    Drop(String, iced::Point, iced::Rectangle),
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),

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

    pub fn update(&mut self, msg: SWMessage, mods: &Modifiers) -> Cm<SWMessage> {
        match msg {
            SWMessage::SimpleSelectSong(idx) => {
                if let SearchType::Tab(_, ref mut mode) = self.search_type {
                    if let SelectionMode::None | SelectionMode::Single(_) = mode {
                        *mode = SelectionMode::Single(idx);
                    }
                }
                Cm::none()
            }
            SWMessage::SelectSong(idx) => {
                if let SearchType::Tab(_, ref mut mode) = self.search_type {
                    *mode = mode.clone().update_selection(idx, mods);
                }
                Cm::none()
            }

            // Handle dragndrop
            SWMessage::Drop(key, point, _) => zones_on_point(
                move |zones| SWMessage::HandleZones(key.clone(), zones),
                point,
                None,
                None,
            ),
            SWMessage::HandleZones(_, _) => unreachable!(),
            SWMessage::SearchQueryChanged(s) => {
                self.query = s;
                Cm::none()
            }
            SWMessage::SearchQuerySubmitted => Cm::none(),
        }
    }
}
