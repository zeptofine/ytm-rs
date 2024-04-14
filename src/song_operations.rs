use std::iter::{once, repeat};

use rand::{seq::index::sample, seq::SliceRandom, thread_rng};
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

impl IntoIterator for RecursiveSongOp {
    type Item = IDKey;
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::SinglePlay(song_id) => Box::new(once(song_id)),
            Self::PlayOnce(ops) => Box::new(ops.into_iter().flatten()),
            Self::LoopNTimes(ops, n) => {
                let length = ops.len();
                Box::new(ops.into_iter().cycle().take(length * n as usize).flatten())
            }
            Self::Stretch(ops, n) => Box::new(
                ops.into_iter()
                    .flat_map(move |op| repeat(op).take(n as usize).flatten()),
            ),
            Self::InfiniteLoop(ops) => Box::new(ops.into_iter().cycle().flatten()),
            Self::RandomPlay(ops) => {
                let length = ops.len();
                Box::new(
                    sample(&mut thread_rng(), length, length)
                        .into_iter()
                        .flat_map(move |idx| ops[idx].clone()),
                )
            }
            Self::SingleRandom(ops) => Box::new({
                let operation = ops.choose(&mut thread_rng()).unwrap();
                operation.clone().into_iter()
            }),
            Self::InfiniteRandom(ops) => Box::new({
                repeat(ops).flat_map(|ops| ops.choose(&mut thread_rng()).unwrap().clone())
            }),
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackResult {
    Rewound,
    Current,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NextResult {
    Current,
    Ended,
}

pub enum SeekingError {
    TooShort,
    TooLong,
}

pub trait OperationTracker {
    fn move_back(&mut self) -> BackResult;

    /// Returns the current tree of indexes to get to the current song.
    fn get_current(&self) -> Vec<&usize>;
    fn move_next(&mut self) -> NextResult;

    // Moves the tracker to its start of the sequence.
    fn to_start(&mut self) {}
    // Moves the tracker to the end of the sequence.
    fn to_end(&mut self) {}
}

#[derive(Debug, Clone)]
pub enum SongOpTracker {
    SinglePlay,
    PlayOnce {
        current: usize,
        children: Vec<SongOpTracker>,
    },
    LoopNTimes {
        current: usize,
        total_loops: usize,
        children: Vec<SongOpTracker>,
    },
}
impl OperationTracker for SongOpTracker {
    fn move_back(&mut self) -> BackResult {
        match self {
            SongOpTracker::SinglePlay => BackResult::Rewound,
            SongOpTracker::PlayOnce { current, children } => match children[*current].move_back() {
                BackResult::Rewound => {
                    if *current == 0 {
                        BackResult::Rewound
                    } else {
                        *current -= 1;
                        children[*current].to_end();
                        BackResult::Current
                    }
                }
                BackResult::Current => BackResult::Current,
            },
            SongOpTracker::LoopNTimes {
                current,
                total_loops,
                children,
            } => {
                let len = children.len();
                match children[*current % len].move_back() {
                    BackResult::Rewound => {
                        if *current == 0 {
                            BackResult::Rewound
                        } else {
                            *current -= 1;
                            children[*current % len].to_end();
                            BackResult::Current
                        }
                    }
                    BackResult::Current => BackResult::Current,
                }
            }
        }
    }

    fn get_current(&self) -> Vec<&usize> {
        match self {
            SongOpTracker::SinglePlay => vec![&0],
            SongOpTracker::PlayOnce { current, children } => {
                let mut result = vec![current];
                result.extend(children[*current].get_current());
                result
            }
            SongOpTracker::LoopNTimes {
                current,
                total_loops: _,
                children,
            } => {
                let mut result = vec![current];
                result.extend(children[*current % children.len()].get_current());
                result
            }
        }
    }

    fn move_next(&mut self) -> NextResult {
        match self {
            SongOpTracker::SinglePlay => NextResult::Ended,
            SongOpTracker::PlayOnce { current, children } => match children[*current].move_next() {
                NextResult::Ended => {
                    if *current == children.len() - 1 {
                        NextResult::Ended
                    } else {
                        *current += 1;
                        children[*current].to_start();
                        NextResult::Current
                    }
                }
                NextResult::Current => NextResult::Current,
            },
            SongOpTracker::LoopNTimes {
                current,
                total_loops,
                children,
            } => {
                let length = children.len();
                match children[*current % length].move_next() {
                    NextResult::Ended => {
                        if *current == (length * *total_loops) - 1 {
                            NextResult::Ended
                        } else {
                            *current += 1;
                            children[*current % length].to_start();
                            NextResult::Current
                        }
                    }
                    NextResult::Current => NextResult::Current,
                }
            }
        }
    }

    fn to_start(&mut self) {
        match self {
            SongOpTracker::SinglePlay => {}
            SongOpTracker::PlayOnce {
                current,
                children: _,
            } => *current = 0,
            SongOpTracker::LoopNTimes {
                current,
                total_loops,
                children,
            } => {
                *current = 0;
            }
        }
    }

    fn to_end(&mut self) {
        match self {
            SongOpTracker::SinglePlay => {}
            SongOpTracker::PlayOnce { current, children } => *current = children.len(),
            SongOpTracker::LoopNTimes {
                current,
                total_loops,
                children,
            } => todo!(),
        }
    }
}

#[test]
pub fn tester() {
    use crate::song_operations::SongOpTracker as SOT;

    let mut tracker = SOT::PlayOnce {
        current: 0,
        children: vec![
            SOT::SinglePlay,
            SOT::SinglePlay,
            SOT::LoopNTimes {
                current: 0,
                total_loops: 2,
                children: vec![SOT::SinglePlay],
            },
        ],
    };

    println!["{:#?}", tracker];
    println!["{:?}", tracker.get_current()];

    while NextResult::Current == tracker.move_next() {
        println!["{:?}", tracker.get_current()];
    }

    while BackResult::Current == tracker.move_back() {
        println!["{:?}", tracker.get_current()];
    }
}
