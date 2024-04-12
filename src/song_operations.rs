use std::{
    iter::{self, once},
    str::FromStr,
};

use rand::{seq::index::sample, seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

use crate::{response_types::IDKey, song::Song};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum InfLoopType {
    Never,
    Maybe,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongOp {
    // Plays a song.
    SinglePlay(IDKey),
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
    // plays a single random song from the provided box.
    SingleRandom(Vec<SongOp>),
    // Plays random songs indefinitely until stopped.
    InfiniteRandom(Vec<SongOp>),
}

impl IntoIterator for SongOp {
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
                    .flat_map(move |op| iter::repeat(op).take(n as usize).flatten()),
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
                iter::repeat(ops).flat_map(|ops| ops.choose(&mut thread_rng()).unwrap().clone())
            }),
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
            | Self::SingleRandom(ops)
            | Self::InfiniteRandom(ops) => ops.iter().all(|so| so.is_valid()),
        }
    }

    pub fn is_infinite(&self) -> InfLoopType {
        match self {
            Self::InfiniteLoop(_) | Self::InfiniteRandom(_) => InfLoopType::Always,
            Self::SinglePlay(_) => InfLoopType::Never,
            Self::PlayOnce(ops)
            | Self::LoopNTimes(ops, _)
            | Self::Stretch(ops, _)
            | Self::RandomPlay(ops) => {
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
            Self::SingleRandom(ops) => {
                match ops.iter().any(|so| so.is_infinite() != InfLoopType::Never) {
                    true => InfLoopType::Maybe,
                    false => InfLoopType::Never,
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn test_str() -> String {
        "fffffff".to_string()
    }

    fn test_str2() -> String {
        "0000000".to_string()
    }

    fn test_obj() -> SongOp {
        SongOp::SinglePlay(test_str())
    }

    fn test_obj2() -> SongOp {
        SongOp::SinglePlay(test_str2())
    }

    #[test]
    fn test_songop_creation() {
        let single = test_obj();
        assert_eq!(single.is_valid(), true);
        let songs: Vec<IDKey> = single.into_iter().collect();
        assert_eq!(songs, vec![test_str()]);
    }

    // Test for is_infinite()
    #[test]
    fn test_is_infinite() {
        let single = test_obj();
        assert_eq!(single.is_infinite(), InfLoopType::Never);
    }

    // Test for PlayOnce
    #[test]
    fn test_playonce() {
        let songs: Vec<SongOp> = vec![test_obj(), test_obj()];
        let play_once = SongOp::PlayOnce(songs);
        assert_eq!(play_once.is_valid(), true);
        let song_keys: Vec<IDKey> = play_once.into_iter().collect();
        for key in &song_keys {
            assert_eq!(*key, test_str());
        }
    }

    // Test for LoopNTimes
    #[test]
    fn test_looptimes() {
        let songs: Vec<SongOp> = vec![test_obj()];
        let loop_times = SongOp::LoopNTimes(songs, 3);
        assert_eq!(loop_times.is_valid(), true);
        let song_keys: Vec<IDKey> = loop_times.into_iter().collect();
        for key in &song_keys {
            assert_eq!(*key, test_str());
        }
    }

    // Test for Stretch
    #[test]
    fn test_stretch() {
        let songs: Vec<SongOp> = vec![test_obj(), test_obj2()];
        let stretch = SongOp::Stretch(songs, 3);
        assert_eq!(stretch.is_valid(), true);
        let song_keys: Vec<IDKey> = stretch.into_iter().collect();
        assert_eq!(
            song_keys,
            vec![
                test_str(),
                test_str(),
                test_str(),
                test_str2(),
                test_str2(),
                test_str2()
            ]
        )
    }
}
