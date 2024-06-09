use std::{fmt::Debug, io::Cursor, path::PathBuf, time::Duration};

use kira::sound::{
    static_sound::{StaticSoundData, StaticSoundHandle},
    streaming::{StreamingSoundData, StreamingSoundHandle},
    FromFileError, PlaybackState,
};

use super::IDed;

#[derive(Debug, Clone)]
pub struct BasicSoundData(String, StaticSoundData);
impl BasicSoundData {
    pub fn data(&self) -> &StaticSoundData {
        &self.1
    }
}

impl IDed<String> for BasicSoundData {
    #[inline]
    fn id(&self) -> &String {
        &self.0
    }
}

impl From<(String, Vec<u8>)> for BasicSoundData {
    fn from(value: (String, Vec<u8>)) -> Self {
        let sound = StaticSoundData::from_cursor(Cursor::new(value.1));
        println!["Created sound from bytes"];

        Self(value.0, sound.unwrap())
    }
}
impl From<(String, StaticSoundData)> for BasicSoundData {
    fn from(value: (String, StaticSoundData)) -> Self {
        Self(value.0, value.1)
    }
}

pub enum SoundDataType {
    Static(StaticSoundData),
    Stream(StreamingSoundData<FromFileError>),
}
impl Debug for SoundDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(arg0) => f.debug_tuple("Static").field(arg0).finish(),
            Self::Stream(_) => f.debug_tuple("Stream").finish(),
        }
    }
}
impl SoundDataType {
    pub fn duration(&self) -> Duration {
        match self {
            Self::Static(arg0) => arg0.duration(),
            Self::Stream(arg0) => arg0.duration(),
        }
    }
}

pub enum SoundDataHandleType {
    Static(StaticSoundHandle),
    Stream(StreamingSoundHandle<FromFileError>),
}
impl SoundDataHandleType {
    pub fn playback_state(&self) -> PlaybackState {
        match self {
            SoundDataHandleType::Static(d) => d.state(),
            SoundDataHandleType::Stream(d) => d.state(),
        }
    }
}

#[derive(Debug)]
pub struct SoundData(String, SoundDataType);

impl SoundData {
    pub fn into_data(self) -> SoundDataType {
        self.1
    }
}
impl From<(String, StaticSoundData)> for SoundData {
    fn from(value: (String, StaticSoundData)) -> Self {
        Self(value.0, SoundDataType::Static(value.1))
    }
}
impl From<BasicSoundData> for SoundData {
    fn from(value: BasicSoundData) -> Self {
        Self(value.0, SoundDataType::Static(value.1))
    }
}
impl From<(String, PathBuf)> for SoundData {
    fn from(value: (String, PathBuf)) -> Self {
        let sound = StreamingSoundData::from_file(value.1);
        println!["Created sound from file"];
        SoundData(value.0, SoundDataType::Stream(sound.unwrap()))
    }
}

impl IDed<String> for SoundData {
    #[inline]
    fn id(&self) -> &String {
        &self.0
    }
}
