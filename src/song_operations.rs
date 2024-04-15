use std::iter::{once, repeat};

use rand::{
    seq::{index::sample, SliceRandom},
    thread_rng, Rng,
};
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

pub trait OperationTracker {
    fn move_back(&mut self) -> BackResult;

    /// Returns the current tree of indexes to get to the current song.
    fn get_current(&self) -> Box<dyn Iterator<Item = usize>>;
    fn move_next(&mut self) -> NextResult;

    // Moves the tracker to its start of the sequence.
    fn to_start(&mut self) {}
    // Moves the tracker to the end of the sequence.
    fn to_end(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Stretch {
        current: usize,
        length: usize,
        children: Vec<SongOpTracker>,
    },
    InfiniteLoop {
        current: usize,
        children: Vec<SongOpTracker>,
    },
    RandomPlay {
        current: usize,
        randomized_indices: Vec<usize>,
        children: Vec<SongOpTracker>,
    },
    SingleRandom {
        index: usize,
        children: Vec<SongOpTracker>,
    },
    InfiniteRandom {
        index: usize,
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
                total_loops: _,
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
            SongOpTracker::Stretch {
                current,
                length,
                children,
            } => match children[*current / *length].move_back() {
                BackResult::Rewound => {
                    if *current == 0 {
                        BackResult::Rewound
                    } else {
                        *current -= 1;
                        children[*current / *length].to_end();
                        BackResult::Current
                    }
                }
                BackResult::Current => BackResult::Current,
            },
            SongOpTracker::InfiniteLoop { current, children } => {
                match children[*current].move_back() {
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
                }
            }
            SongOpTracker::RandomPlay {
                current,
                randomized_indices,
                children,
            } => match children[randomized_indices[*current]].move_back() {
                BackResult::Rewound => {
                    if *current == 0 {
                        BackResult::Rewound
                    } else {
                        *current -= 1;
                        children[randomized_indices[*current]].to_end();
                        BackResult::Current
                    }
                }
                BackResult::Current => BackResult::Current,
            },
            SongOpTracker::SingleRandom { index, children } => match children[*index].move_back() {
                BackResult::Rewound => {
                    // make a new selection
                    *index = thread_rng().gen_range(0..children.len());
                    children[*index].to_end();
                    BackResult::Rewound
                }
                BackResult::Current => BackResult::Current,
            },
            SongOpTracker::InfiniteRandom { index, children } => children[*index].move_back(),
        }
    }

    fn get_current(&self) -> Box<dyn Iterator<Item = usize>> {
        match self {
            SongOpTracker::SinglePlay => Box::new(vec![0_usize].into_iter()),
            SongOpTracker::PlayOnce { current, children } => Box::new(
                vec![*current]
                    .into_iter()
                    .chain(children[*current].get_current()),
            ),
            SongOpTracker::LoopNTimes {
                current,
                total_loops: _,
                children,
            } => Box::new(
                [current % children.len()]
                    .into_iter()
                    .chain(children[*current % children.len()].get_current()),
            ),
            SongOpTracker::Stretch {
                current,
                length,
                children,
            } => Box::new(
                vec![*current / *length]
                    .into_iter()
                    .chain(children[*current / *length].get_current()),
            ),
            SongOpTracker::InfiniteLoop { current, children } => Box::new(
                vec![*current]
                    .into_iter()
                    .chain(children[*current].get_current()),
            ),
            SongOpTracker::RandomPlay {
                current,
                randomized_indices,
                children,
            } => Box::new(
                vec![randomized_indices[*current]]
                    .into_iter()
                    .chain(children[randomized_indices[*current]].get_current()),
            ),
            SongOpTracker::SingleRandom { index, children } => Box::new(
                vec![*index]
                    .into_iter()
                    .chain(children[*index].get_current()),
            ),
            SongOpTracker::InfiniteRandom { index, children } => Box::new(
                vec![*index]
                    .into_iter()
                    .chain(children[*index].get_current()),
            ),
        }
    }

    fn move_next(&mut self) -> NextResult {
        match self {
            SongOpTracker::SinglePlay => NextResult::Ended,
            SongOpTracker::PlayOnce { current, children } => match children[*current].move_next() {
                NextResult::Current => NextResult::Current,
                NextResult::Ended => {
                    if *current == children.len() - 1 {
                        NextResult::Ended
                    } else {
                        *current += 1;
                        children[*current].to_start();
                        NextResult::Current
                    }
                }
            },
            SongOpTracker::LoopNTimes {
                current,
                total_loops,
                children,
            } => {
                let length = children.len();
                match children[*current % length].move_next() {
                    NextResult::Current => NextResult::Current,
                    NextResult::Ended => {
                        if *current == (length * *total_loops - 1) {
                            NextResult::Ended
                        } else {
                            *current += 1;
                            children[*current % length].to_start();
                            NextResult::Current
                        }
                    }
                }
            }
            SongOpTracker::Stretch {
                current,
                length,
                children,
            } => match children[*current / *length].move_next() {
                NextResult::Current => NextResult::Current,
                NextResult::Ended => {
                    if *current >= children.len() * *length - 1 {
                        NextResult::Ended
                    } else {
                        *current += 1;
                        children[*current / *length].to_start();
                        NextResult::Current
                    }
                }
            },
            SongOpTracker::InfiniteLoop { current, children } => {
                match children[*current].move_next() {
                    NextResult::Current => NextResult::Current,
                    NextResult::Ended => {
                        *current = (*current + 1) % children.len();
                        children[*current].to_start();
                        NextResult::Current
                    }
                }
            }
            SongOpTracker::RandomPlay {
                current,
                randomized_indices,
                children,
            } => match children[randomized_indices[*current]].move_next() {
                NextResult::Current => NextResult::Current,
                NextResult::Ended => {
                    if *current + 1 >= children.len() {
                        NextResult::Ended
                    } else {
                        *current += 1;
                        children[randomized_indices[*current]].to_start();
                        NextResult::Current
                    }
                }
            },
            SongOpTracker::SingleRandom { index, children } => children[*index].move_next(),
            SongOpTracker::InfiniteRandom { index, children } => {
                match children[*index].move_next() {
                    NextResult::Ended => {
                        *index = rand::thread_rng().gen_range(0..children.len());
                        children[*index].to_start();
                        NextResult::Current
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
                total_loops: _,
                children: _,
            } => {
                *current = 0;
            }
            SongOpTracker::Stretch {
                current,
                length: _,
                children: _,
            } => *current = 0,
            SongOpTracker::InfiniteLoop { current, children } => {
                *current = 0;
                children[*current].to_start();
            }
            SongOpTracker::RandomPlay {
                current,
                randomized_indices,
                children,
            } => {
                *current = 0;
                randomized_indices.shuffle(&mut thread_rng());
                children[randomized_indices[*current]].to_start();
            }
            SongOpTracker::SingleRandom { index, children } => {
                *index = thread_rng().gen_range(0..children.len());
                children[*index].to_start();
            }
            SongOpTracker::InfiniteRandom { index, children } => {
                *index = thread_rng().gen_range(0..children.len());
                children[*index].to_start();
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
            } => *current = *total_loops * children.len(),
            SongOpTracker::Stretch {
                current,
                length,
                children,
            } => *current = *length * children.len() - 1,
            SongOpTracker::InfiniteLoop { current, children } => {
                *current = children.len();
                children[*current].to_end();
            }
            SongOpTracker::RandomPlay {
                current,
                randomized_indices,
                children,
            } => {
                *current = randomized_indices.len();
                children[randomized_indices[*current]].to_end();
            }
            SongOpTracker::SingleRandom { index, children } => children[*index].to_end(),
            SongOpTracker::InfiniteRandom { index, children } => children[*index].to_end(),
        }
    }
}
impl SongOpTracker {
    fn map(ops: &[RecursiveSongOp]) -> Vec<SongOpTracker> {
        ops.iter().map(Self::from).collect()
    }
}
impl From<&RecursiveSongOp> for SongOpTracker {
    fn from(value: &RecursiveSongOp) -> Self {
        match value {
            RecursiveSongOp::SinglePlay(_) => Self::SinglePlay,
            RecursiveSongOp::PlayOnce(ops) => Self::PlayOnce {
                current: 0,
                children: Self::map(ops),
            },
            RecursiveSongOp::LoopNTimes(ops, n) => Self::LoopNTimes {
                current: 0,
                total_loops: *n as usize,
                children: Self::map(ops),
            },

            RecursiveSongOp::Stretch(ops, n) => Self::Stretch {
                current: 0,
                length: *n as usize,
                children: Self::map(ops),
            },
            RecursiveSongOp::InfiniteLoop(ops) => Self::InfiniteLoop {
                current: 0,
                children: Self::map(ops),
            },
            RecursiveSongOp::RandomPlay(ops) => {
                let mut randomized_indices = (0..ops.len()).collect::<Vec<usize>>();
                randomized_indices.shuffle(&mut thread_rng());
                Self::RandomPlay {
                    current: 0,
                    randomized_indices,
                    children: Self::map(ops),
                }
            }
            RecursiveSongOp::SingleRandom(ops) => Self::SingleRandom {
                index: thread_rng().gen_range(0..ops.len()),
                children: Self::map(ops),
            },
            RecursiveSongOp::InfiniteRandom(ops) => Self::InfiniteRandom {
                index: thread_rng().gen_range(0..ops.len()),
                children: Self::map(ops),
            },
        }
    }
}

#[test]
pub fn tester() {
    use crate::song_operations::RecursiveSongOp as RSO;

    let ops: RSO = RSO::SingleRandom(vec![
        RSO::SinglePlay("".to_string()),
        RSO::RandomPlay(vec![
            RSO::SinglePlay("".to_string()),
            RSO::SingleRandom(vec![
                RSO::SinglePlay("".to_string()),
                RSO::SinglePlay("".to_string()),
            ]),
        ]),
    ]);

    let mut tracker = SongOpTracker::from(&ops);

    println!["Hello, {}!", "World"];

    println!["{:?}", tracker];
    println!["{:?}", tracker.get_current().collect::<Vec<_>>()];

    let limit = 100;
    let mut n = 0;
    while NextResult::Current == tracker.move_next() && n < limit {
        // println!["{:?}", tracker];
        println!["{:?}", tracker.get_current().collect::<Vec<_>>()];
        n += 1;
    }
    println!["Finished going forwards"];

    while BackResult::Current == tracker.move_back() {
        println!["{:?}", tracker.get_current().collect::<Vec<_>>()];
    }
}
