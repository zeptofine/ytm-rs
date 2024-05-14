use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::fmt::Debug;

pub struct YTMRSAudioManager {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
}
impl Debug for YTMRSAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("YTMRSAudioManager")
    }
}
impl Default for YTMRSAudioManager {
    fn default() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        Self {
            stream,
            handle,
            sink,
        }
    }
}
impl YTMRSAudioManager {
    pub fn new(stream: OutputStream, handle: OutputStreamHandle) -> Self {
        let sink = Sink::try_new(&handle).unwrap();
        Self {
            stream,
            handle,
            sink,
        }
    }
}
