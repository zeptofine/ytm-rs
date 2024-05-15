use std::collections::{HashMap, HashSet};

use iced::{
    alignment::Horizontal,
    keyboard::Modifiers,
    widget::{scrollable, Column, Container},
    Command as Cm, Element, Length,
};
use iced_drop::{droppable, zones_on_point};
use serde::{Deserialize, Serialize};

use crate::{
    caching::CacheInterface,
    song::{Song, SongData},
    styling::FullYtmrsScheme,
    user_input::SelectionMode,
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
}

#[derive(Debug, Clone)]
pub enum SWMessage {
    Drop(String, iced::Point, iced::Rectangle),
    HandleZones(String, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),
    SimpleSelectSong(usize),
    SelectSong(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWindow {
    pub search_type: SearchType,
    #[serde(skip)]
    pub cache: CacheInterface<Song>,
}
impl Default for SearchWindow {
    fn default() -> Self {
        SearchWindow {
            search_type: SearchType::Tab(vec![], SelectionMode::None),
            cache: CacheInterface::default(),
        }
    }
}
impl SearchWindow {
    pub fn used_keys(&self) -> Vec<&String> {
        match self.search_type {
            SearchType::Song(ref song) => vec![song],
            SearchType::Tab(ref v, _) | SearchType::Search(ref v) => v.iter().collect(),
        }
    }

    pub fn selected_keys(&self) -> Vec<&String> {
        match &self.search_type {
            SearchType::Song(s) => vec![&s],
            SearchType::Tab(s, mode) => match mode {
                SelectionMode::None => vec![],
                SelectionMode::Single(idx) => {
                    let k = s.get(*idx);
                    match k {
                        Some(k) => vec![k],
                        None => vec![],
                    }
                }
                SelectionMode::Multiple(v) => v.iter().filter_map(|idx| s.get(*idx)).collect(),
                SelectionMode::Range { first: _, r } => {
                    r.clone().filter_map(|idx| s.get(idx)).collect()
                }
            },
            SearchType::Search(_) => vec![],
        }
    }

    pub fn view(&self, scheme: &FullYtmrsScheme) -> Element<SWMessage> {
        let keys: HashSet<String> = match self.search_type.clone() {
            SearchType::Song(song) => vec![song].into_iter().collect(),
            SearchType::Tab(v, _) | SearchType::Search(v) => v.into_iter().collect(),
        };

        let cached_map: HashMap<_, _> = self.cache.get(&keys).collect();

        match &self.search_type {
            SearchType::Song(_) => {
                todo!()
            }
            SearchType::Tab(v, mode) => {
                let songs = v.iter().enumerate().map(|(idx, key)| {
                    let selected = mode.contains(idx);
                    let style = scheme.song_appearance.update(selected);
                    droppable(
                        Container::new(match cached_map.get(key) {
                            Some(songc) => {
                                let song = songc.lock().unwrap();
                                song.as_data().row()
                            }
                            None => SongData::mystery_with_id(key.clone()).row(),
                        })
                        .style(move |_| style),
                    )
                    .on_drop(move |pt, rec| SWMessage::Drop(key.clone(), pt, rec))
                    .on_click(SWMessage::SimpleSelectSong(idx))
                    .on_single_click(SWMessage::SelectSong(idx))
                    .into()
                });

                scrollable(
                    Container::new(Column::with_children(songs).width(Length::Fill))
                        .align_x(Horizontal::Left)
                        .width(Length::Fill)
                        .max_width(400)
                        .padding(0),
                )
                .style(scheme.scrollable_style.clone().update())
                .into()
            }
            SearchType::Search(_) => todo!(),
        }
    }

    pub fn update(&mut self, msg: SWMessage, mods: &Modifiers) -> Cm<SWMessage> {
        match msg {
            SWMessage::SimpleSelectSong(idx) => {
                if let SearchType::Tab(_, ref mut mode) = self.search_type {
                    if let SelectionMode::None = mode {
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
            SWMessage::HandleZones(_, _) => todo!(),
        }
    }
}