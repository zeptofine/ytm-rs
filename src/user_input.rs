use std::{cmp::Ordering, fmt::Debug, ops::RangeInclusive};

use iced::{
    keyboard::{self, Modifiers},
    widget::{column, text_input},
    Command as Cm, Element,
};
use once_cell::sync::Lazy;

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Clone, Default)]
pub enum SelectionMode {
    #[default]
    None,
    Single(usize),
    Multiple(Vec<usize>),
    Range {
        first: usize,
        r: RangeInclusive<usize>,
    },
}
impl SelectionMode {
    /// Update the selection mode based on the current selection and the new click
    pub fn update_selection(self, clicked_idx: usize, modifiers: &Modifiers) -> Self {
        if modifiers.shift() {
            match self {
                Self::None => Self::Single(clicked_idx),
                Self::Single(idx) => match clicked_idx.cmp(&idx) {
                    Ordering::Equal => Self::None,
                    Ordering::Less => Self::Range {
                        first: idx,
                        r: clicked_idx..=idx,
                    },
                    Ordering::Greater => Self::Range {
                        first: idx,
                        r: idx..=clicked_idx,
                    },
                },
                Self::Multiple(v) => {
                    let last = v.last().unwrap();
                    match last.cmp(&clicked_idx) {
                        Ordering::Equal => Self::Single(clicked_idx),
                        Ordering::Less => Self::Range {
                            first: *last,
                            r: *last..=clicked_idx,
                        },
                        Ordering::Greater => Self::Range {
                            first: *last,
                            r: clicked_idx..=*last,
                        },
                    }
                }
                Self::Range { first, r: _ } => match first.cmp(&clicked_idx) {
                    Ordering::Equal => Self::Single(clicked_idx),
                    Ordering::Less => Self::Range {
                        first,
                        r: first..=clicked_idx,
                    },
                    Ordering::Greater => Self::Range {
                        first,
                        r: clicked_idx..=first,
                    },
                },
            }
        } else if modifiers.control() {
            match self {
                Self::None => Self::Multiple(vec![clicked_idx]),
                Self::Single(idx) => Self::Multiple(vec![idx, clicked_idx]),
                Self::Multiple(mut v) => {
                    match v.iter().position(|&x| x == clicked_idx) {
                        Some(idx) => {
                            v.remove(idx);
                        }
                        None => {
                            v.push(clicked_idx);
                        }
                    }
                    Self::Multiple(v)
                }
                Self::Range { first: _, r } => {
                    let mut v: Vec<usize> = r.into_iter().collect();
                    match v.iter().position(|&x| x == clicked_idx) {
                        Some(idx) => {
                            v.remove(idx);
                        }
                        None => {
                            v.push(clicked_idx);
                        }
                    }
                    Self::Multiple(v)
                }
            }
        } else {
            Self::Single(clicked_idx)
        }
    }

    pub fn contains(&self, idx: usize) -> bool {
        match self {
            Self::None => false,
            Self::Single(sidx) => *sidx == idx,
            Self::Multiple(v) => v.contains(&idx),
            Self::Range { first: _, r } => r.contains(&idx),
        }
    }
}

#[derive(Debug, Default)]
pub struct UserInputs {
    pub url: String,
    pub modifiers: keyboard::Modifiers,
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    UrlChanged(String),
    UrlSubmitted,
}

impl UserInputs {
    pub fn view(&self) -> Element<InputMessage> {
        column![text_input("", &self.url)
            .id(INPUT_ID.clone())
            .on_input(InputMessage::UrlChanged)
            .on_submit(InputMessage::UrlSubmitted)
            .size(20)
            .padding(15),]
        .into()
    }

    pub fn update(&mut self, message: InputMessage) -> Cm<InputMessage> {
        match message {
            InputMessage::UrlChanged(s) => self.url = s,
            InputMessage::UrlSubmitted => {}
        }
        Cm::none()
    }
}
