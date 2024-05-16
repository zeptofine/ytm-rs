use iced::{
    widget::{button, column, row, scrollable, text, text_input},
    Element,
};
use serde::{Deserialize, Serialize};

use crate::{
    song_operations::{SongOpConstructor, SongOpMessage},
    styling::FullYtmrsScheme,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistHeader {
    pub id: uuid::Uuid,
    pub name: String,
    pub num_of_songs: usize,
}
impl From<&Playlist> for PlaylistHeader {
    fn from(value: &Playlist) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            num_of_songs: value.constructor.all_song_keys_rec().count(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PlaylistMessage {
    ConstructorMessage(SongOpMessage),
    NameEdited(String),
    Save,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: uuid::Uuid,
    pub name: String,
    pub constructor: SongOpConstructor,
}
impl Playlist {
    pub fn view(&self, scheme: &FullYtmrsScheme) -> Element<PlaylistMessage> {
        let name_edit =
            text_input(&self.id.to_string(), &self.name).on_input(PlaylistMessage::NameEdited);
        let save_button = button(text("save")).on_press(PlaylistMessage::Save);

        let constructor = scrollable(
            self.constructor
                .view(scheme)
                .map(PlaylistMessage::ConstructorMessage),
        );

        column![row![name_edit, save_button], constructor].into()
    }
}
