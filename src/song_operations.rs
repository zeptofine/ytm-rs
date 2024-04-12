use std::iter;

use serde::{Deserialize, Serialize};

use crate::song::Song;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum InfLoopType {
    Never,
    Maybe,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongOp {
    // Plays a song.
    SinglePlay(Box<Song>),
    // Plays a list of songs once.
    PlayOnce(Vec<SongOp>),
    // Loops a list of songs entirely N times.
    LoopNTimes(Vec<SongOp>, u32),
    // Loops each song individually N times.
    Stretch(Vec<SongOp>, u32),
    // Loops list indefinitely.
    InfiniteLoop(Vec<SongOp>),
    // Plays a random selection of songs from the provided box
    RandomPlay(Vec<SongOp>),
    // Plays random songs indefinitely until stopped.
    InfiniteRandom(Vec<SongOp>),
}

impl IntoIterator for SongOp {
    type Item = Box<Song>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::SinglePlay(song) => Box::new(iter::once(song)),
            Self::PlayOnce(ops) => Box::new(ops.into_iter().flat_map(|op| op.into_iter())),
            Self::LoopNTimes(ops, n) => Box::new(
                iter::repeat_with(move || ops.clone().into_iter().flat_map(|op| op.into_iter()))
                    .take(n as usize)
                    .flatten(),
            ),
            Self::Stretch(ops, n) => Box::new(ops.into_iter().flat_map(move |op| {
                iter::repeat_with(move || op.clone().into_iter())
                    .take(n as usize)
                    .flatten()
            })),
            Self::InfiniteLoop(ops) => Box::new(
                iter::repeat_with(move || ops.clone().into_iter().flat_map(|op| op.into_iter()))
                    .flatten(),
            ),

            _ => todo!(),
        }
    }
}

impl SongOp {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SinglePlay(_) => true,
            Self::PlayOnce(ops)
            | Self::LoopNTimes(ops, _)
            | Self::Stretch(ops, _)
            | Self::InfiniteLoop(ops)
            | Self::RandomPlay(ops)
            | Self::InfiniteRandom(ops) => ops.iter().all(|so| so.is_valid()),
        }
    }

    pub fn is_infinite(&self) -> InfLoopType {
        match self {
            Self::InfiniteLoop(_) | Self::InfiniteRandom(_) => InfLoopType::Always,
            Self::SinglePlay(_) => InfLoopType::Never,
            Self::PlayOnce(ops) | Self::LoopNTimes(ops, _) | Self::Stretch(ops, _) => {
                match ops.iter().find_map(|op| {
                    let inftype = op.is_infinite();
                    match inftype != InfLoopType::Never {
                        true => Some(inftype),
                        false => None,
                    }
                }) {
                    Some(op) => op,
                    None => InfLoopType::Never,
                }
            }
            Self::RandomPlay(ops) => {
                match ops.iter().any(|so| so.is_infinite() != InfLoopType::Never) {
                    true => InfLoopType::Maybe,
                    false => InfLoopType::Never,
                }
            }
        }
    }
}
