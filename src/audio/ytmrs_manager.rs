use std::{fmt::Debug, path::PathBuf, time::Duration};

use kittyaudio::{Mixer, Sound, SoundHandle};

pub struct YTMRSAudioManager {
    mixer: Mixer,
    current_song: Option<SoundHandle>,
}
impl Debug for YTMRSAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YTMRSAudioManager")
    }
}
impl Default for YTMRSAudioManager {
    fn default() -> Self {
        let mixer = Mixer::new();
        mixer.init();
        Self {
            mixer,
            current_song: None,
        }
    }
}
impl YTMRSAudioManager {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            mixer,
            current_song: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.current_song.clone().is_some_and(|s| !s.paused())
    }

    pub fn play(&mut self) {
        if let Some(song) = &self.current_song {
            song.resume();
        }
    }

    pub fn pause(&mut self) {
        if let Some(song) = &self.current_song {
            song.pause();
        }
    }

    pub fn volume(&self) -> f32 {
        if let Some(song) = &self.current_song {
            song.volume()
        } else {
            0.0
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(song) = &self.current_song {
            song.set_volume(volume);
        }
    }

    pub fn seek(&mut self, pos: Duration) {
        if let Some(song) = &mut self.current_song {
            let secs = pos.as_secs_f32();
            let index = (secs * song.sample_rate() as f32) as usize;
            song.seek_to_index(index);
        }
    }

    pub fn position(&self) -> Option<Duration> {
        self.current_song.as_ref().map(|song| {
            let index = song.index();
            let secs = index as f32 / song.sample_rate() as f32;
            Duration::from_secs_f32(secs)
        })
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.current_song.as_ref().map(|s| s.duration())
    }

    pub fn play_once(&mut self, song: &PathBuf) {
        let sound = Sound::from_path(song).unwrap();
        if let Some(song) = &self.current_song {
            song.seek_to_end();
        }
        self.current_song = Some(self.mixer.play(sound));
    }
}
