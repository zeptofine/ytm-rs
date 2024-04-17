use serde::{Deserialize, Serialize};

use crate::response_types::IDKey;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InfLoopType {
    Never,
    Maybe,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecursiveSongOp {
    // Plays a song.
    SinglePlay(IDKey),
    // Plays a list of songs once.
    PlayOnce(Vec<RecursiveSongOp>),
    // Loops a list of songs entirely N times.
    LoopNTimes(Vec<RecursiveSongOp>, u32),
    // Loops each song individually N times.
    Stretch(Vec<RecursiveSongOp>, u32),
    // Loops list indefinitely.
    InfiniteLoop(Vec<RecursiveSongOp>),
    // Plays a random selection of songs from the provided box
    RandomPlay(Vec<RecursiveSongOp>),
    // plays a single random song from the provided box.
    SingleRandom(Vec<RecursiveSongOp>),
    // Plays random songs indefinitely until stopped.
    InfiniteRandom(Vec<RecursiveSongOp>),
}

impl RecursiveSongOp {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SinglePlay(_) => true,
            Self::PlayOnce(ops)
            | Self::LoopNTimes(ops, _)
            | Self::Stretch(ops, _)
            | Self::InfiniteLoop(ops)
            | Self::RandomPlay(ops)
            | Self::SingleRandom(ops)
            | Self::InfiniteRandom(ops) => ops.iter().all(|so| so.is_valid()),
        }
    }

    pub fn loop_type(&self) -> InfLoopType {
        match self {
            Self::InfiniteLoop(_) | Self::InfiniteRandom(_) => InfLoopType::Always,
            Self::SinglePlay(_) => InfLoopType::Never,
            Self::PlayOnce(ops)
            | Self::LoopNTimes(ops, _)
            | Self::Stretch(ops, _)
            | Self::RandomPlay(ops) => {
                match ops.iter().find_map(|op| {
                    let inftype = op.loop_type();
                    match inftype != InfLoopType::Never {
                        true => Some(inftype),
                        false => None,
                    }
                }) {
                    Some(op) => op,
                    None => InfLoopType::Never,
                }
            }
            Self::SingleRandom(ops) => {
                match ops.iter().any(|so| so.loop_type() != InfLoopType::Never) {
                    true => InfLoopType::Maybe,
                    false => InfLoopType::Never,
                }
            }
        }
    }
}
