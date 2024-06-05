use kittyaudio::Sound;

use super::IDed;

#[derive(Debug, Clone)]
pub struct SoundData(String, Sound);
impl SoundData {
    pub fn sound(&self) -> &Sound {
        &self.1
    }
}
impl From<(String, Vec<u8>)> for SoundData {
    fn from(value: (String, Vec<u8>)) -> Self {
        // println!["{:?}", value];
        SoundData(value.0, Sound::from_bytes(value.1).unwrap())
    }
}
impl IDed<String> for SoundData {
    fn id(&self) -> &String {
        &self.0
    }
}