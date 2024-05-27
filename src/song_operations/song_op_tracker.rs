use std::collections::VecDeque;

use rand::{seq::SliceRandom, thread_rng, Rng};

use super::RecursiveSongOp;

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

    /// Sets the current tree of indexes to get to the current song.
    fn set_current(&mut self, indices: VecDeque<usize>);

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
        current: usize,
        children: Vec<SongOpTracker>,
    },
    InfiniteRandom {
        current: usize,
        children: Vec<SongOpTracker>,
    },
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
                let mut randomized_indices = (0..ops.len()).collect::<Vec<_>>();
                randomized_indices.shuffle(&mut thread_rng());
                Self::RandomPlay {
                    current: 0,
                    randomized_indices,
                    children: Self::map(ops),
                }
            }
            RecursiveSongOp::SingleRandom(ops) => Self::SingleRandom {
                current: thread_rng().gen_range(0..ops.len()),
                children: Self::map(ops),
            },
            RecursiveSongOp::InfiniteRandom(ops) => Self::InfiniteRandom {
                current: thread_rng().gen_range(0..ops.len()),
                children: Self::map(ops),
            },
        }
    }
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
            SongOpTracker::SingleRandom {
                current: index,
                children,
            } => match children[*index].move_back() {
                BackResult::Rewound => {
                    // make a new selection
                    *index = thread_rng().gen_range(0..children.len());
                    children[*index].to_end();
                    BackResult::Rewound
                }
                BackResult::Current => BackResult::Current,
            },
            SongOpTracker::InfiniteRandom {
                current: index,
                children,
            } => children[*index].move_back(),
        }
    }

    fn get_current(&self) -> Box<dyn Iterator<Item = usize>> {
        match self {
            SongOpTracker::SinglePlay => Box::new(vec![].into_iter()),
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
            SongOpTracker::SingleRandom { current, children } => Box::new(
                vec![*current]
                    .into_iter()
                    .chain(children[*current].get_current()),
            ),
            SongOpTracker::InfiniteRandom { current, children } => Box::new(
                vec![*current]
                    .into_iter()
                    .chain(children[*current].get_current()),
            ),
        }
    }

    fn set_current(&mut self, mut indices: VecDeque<usize>) {
        match self {
            SongOpTracker::SinglePlay => {
                println!["indices: {:?}", indices];
            }
            SongOpTracker::PlayOnce {
                ref mut current,
                ref mut children,
            }
            | SongOpTracker::LoopNTimes {
                ref mut current,
                total_loops: _,
                ref mut children,
            }
            | SongOpTracker::Stretch {
                ref mut current,
                length: _,
                ref mut children,
            }
            | SongOpTracker::InfiniteLoop {
                ref mut current,
                ref mut children,
            }
            | SongOpTracker::RandomPlay {
                ref mut current,
                randomized_indices: _,
                ref mut children,
            }
            | SongOpTracker::SingleRandom {
                ref mut current,
                ref mut children,
            }
            | SongOpTracker::InfiniteRandom {
                ref mut current,
                ref mut children,
            } => {
                if let Some(idx) = indices.pop_front() {
                    *current = idx;
                    if !indices.is_empty() {
                        let selected_child = &mut children[*current];
                        selected_child.set_current(indices);
                    }
                }
            }
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
            SongOpTracker::SingleRandom {
                current: index,
                children,
            } => children[*index].move_next(),
            SongOpTracker::InfiniteRandom {
                current: index,
                children,
            } => match children[*index].move_next() {
                NextResult::Ended => {
                    *index = rand::thread_rng().gen_range(0..children.len());
                    children[*index].to_start();
                    NextResult::Current
                }
                NextResult::Current => NextResult::Current,
            },
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
            SongOpTracker::SingleRandom {
                current: index,
                children,
            } => {
                *index = thread_rng().gen_range(0..children.len());
                children[*index].to_start();
            }
            SongOpTracker::InfiniteRandom {
                current: index,
                children,
            } => {
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
                *current = randomized_indices.len() - 1;
                children[randomized_indices[*current]].to_end();
            }
            SongOpTracker::SingleRandom {
                current: index,
                children,
            } => children[*index].to_end(),
            SongOpTracker::InfiniteRandom {
                current: index,
                children,
            } => children[*index].to_end(),
        }
    }
}
impl SongOpTracker {
    fn map(ops: &[RecursiveSongOp]) -> Vec<SongOpTracker> {
        ops.iter().map(Self::from).collect()
    }

    pub fn from_song_op(song_op: &RecursiveSongOp, indices: VecDeque<usize>) -> Self {
        let mut s = Self::from(song_op);
        s.set_current(indices);
        s
    }
}

#[cfg(test)]
mod tests {
    use crate::song_operations::{
        BackResult, NextResult, OperationTracker, RecursiveSongOp as RSO, SongOpTracker,
    };

    #[test]
    pub fn tester() {
        let ops: RSO = RSO::PlayOnce(vec![
            RSO::SinglePlay("".to_string()),
            RSO::PlayOnce(vec![
                RSO::SinglePlay("".to_string()),
                RSO::RandomPlay(vec![
                    RSO::SinglePlay("".to_string()),
                    RSO::SinglePlay("".to_string()),
                ]),
            ]),
        ]);

        let mut tracker = SongOpTracker::from(&ops);

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
}
