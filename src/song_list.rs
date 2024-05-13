use serde::{Deserialize, Serialize};

// Since Iced's Scrollable component gets *real* laggy with big lists,
// we use one with pages to maintain performance.

// pub enum PSLMessage {
//     NextPage,
//     PrevPage,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedSongList<T> {
    pub songs: Vec<T>,
    song_per_page: usize,
    current_index: usize,
}

// impl<T> Default for PagedSongList<T> {
//     fn default() -> Self {
//         Self {
//             songs: Default::default(),
//             song_per_page: 25,
//             current_index: 0,
//         }
//     }
// }

// impl<T> PagedSongList<T> {
//     pub fn get_current_page(&self) -> std::iter::Take<std::slice::Iter<'_, T>> {
//         self.songs[(self.current_index * self.song_per_page)..]
//             .into_iter()
//             .take(self.song_per_page)
//     }

//     pub fn clear(&mut self) {
//         self.songs.clear();
//     }

//     pub fn push(&mut self, obj: T) {
//         self.songs.push(obj);
//     }

//     pub fn view(&self, scheme: &FullYtmrsScheme) -> Element {}
// }
