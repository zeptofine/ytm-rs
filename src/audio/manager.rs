use std::{fmt::Debug, time::Duration};

use iced::Subscription;
use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::PlaybackState,
    tween::Tween,
    Volume,
};

use crate::caching::{SoundData, SoundDataHandleType, SoundDataType};

pub struct CurrentSong {
    pub handle: SoundDataHandleType,
    pub duration: Duration,
}

pub struct YTMRSAudioManager {
    manager: AudioManager,
    current_song: Option<CurrentSong>,
}
impl Debug for YTMRSAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YTMRSAudioManager")
    }
}
impl Default for YTMRSAudioManager {
    fn default() -> Self {
        Self {
            manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            current_song: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChangeSong {}

impl YTMRSAudioManager {
    pub fn subscription(&self) -> Subscription<ChangeSong> {
        match self.playback_state() {
            PlaybackState::Playing => {
                // get the time that will take when the song will be finished
                let remaining = self
                    .total()
                    .unwrap()
                    .checked_sub(Duration::from_secs_f64(self.elapsed().unwrap()));
                match remaining {
                    Some(remaining) => iced::time::every(remaining).map(|_| ChangeSong {}),
                    None => Subscription::none(),
                }
            }
            PlaybackState::Pausing
            | PlaybackState::Paused
            | PlaybackState::Stopping
            | PlaybackState::Stopped => Subscription::none(),
        }
    }

    pub fn playback_state(&self) -> PlaybackState {
        match &self.current_song {
            Some(s) => s.handle.playback_state(),
            None => PlaybackState::Stopped,
        }
    }

    pub fn play(&mut self) {
        if let Some(s) = &mut self.current_song {
            match &mut s.handle {
                SoundDataHandleType::Static(d) => d.resume(Tween::default()),
                SoundDataHandleType::Stream(d) => d.resume(Tween::default()),
            }
        }
    }

    pub fn pause(&mut self) {
        if let Some(s) = &mut self.current_song {
            match &mut s.handle {
                SoundDataHandleType::Static(d) => d.pause(Tween::default()),
                SoundDataHandleType::Stream(d) => d.pause(Tween::default()),
            }
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        if let Some(s) = &mut self.current_song {
            match &mut s.handle {
                SoundDataHandleType::Static(d) => {
                    d.set_volume(Volume::Amplitude(volume), Tween::default())
                }
                SoundDataHandleType::Stream(d) => {
                    d.set_volume(Volume::Amplitude(volume), Tween::default())
                }
            }
        }
    }

    pub fn seek(&mut self, secs: f64) {
        if let Some(s) = &mut self.current_song {
            match &mut s.handle {
                SoundDataHandleType::Static(d) => d.seek_to(secs),
                SoundDataHandleType::Stream(d) => d.seek_to(secs),
            }
        }
    }

    pub fn seek_to_start(&mut self) {
        if let Some(s) = &mut self.current_song {
            match &mut s.handle {
                SoundDataHandleType::Static(d) => d.seek_to(0.),
                SoundDataHandleType::Stream(d) => d.seek_to(0.),
            }
        }
    }

    pub fn seek_to_end(&mut self) {
        let total = self.total();
        if let Some(s) = &mut self.current_song {
            let total = total.unwrap().as_secs_f64();
            match &mut s.handle {
                SoundDataHandleType::Static(d) => d.seek_to(total),
                SoundDataHandleType::Stream(d) => d.seek_to(total),
            }
        }
    }

    pub fn elapsed(&self) -> Option<f64> {
        self.current_song.as_ref().map(|s| match &s.handle {
            SoundDataHandleType::Static(d) => d.position(),
            SoundDataHandleType::Stream(d) => d.position(),
        })
    }

    pub fn total(&self) -> Option<Duration> {
        self.current_song.as_ref().map(|s| s.duration)
    }

    pub fn play_once(&mut self, sound: SoundData) {
        self.seek_to_end();

        let data = sound.into_data();
        let duration = data.duration();
        let handle = match data {
            SoundDataType::Static(d) => SoundDataHandleType::Static(self.manager.play(d).unwrap()),
            SoundDataType::Stream(d) => SoundDataHandleType::Stream(self.manager.play(d).unwrap()),
        };
        let current_song = CurrentSong { handle, duration };

        self.current_song = Some(current_song);
    }
}
