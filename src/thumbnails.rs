use std::path::PathBuf;

use iced::widget::image::Handle;
use image::{self, GenericImageView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub enum ThumbnailState {
    #[default]
    Unknown,

    Downloaded {
        path: PathBuf,
        handle: Handle,
        colors: Option<Handle>,
    },
}

pub async fn get_thumbnail(thumbnail_url: String, output: PathBuf) -> PathBuf {
    if output.exists() {
        return output;
    }
    let imgbytes = reqwest::get(thumbnail_url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let mut thumbnail = image::load_from_memory(&imgbytes).unwrap();
    let (w, h) = thumbnail.dimensions();
    // crop it to a square
    let smaller = h.min(w);
    let left = (w - smaller) / 2;
    let top = (h - smaller) / 2;

    thumbnail = thumbnail.crop(left, top, smaller, smaller);
    match thumbnail.save(&output) {
        Ok(_) => {}
        Err(e) => println!["Failed to save thumbnail: {}", e],
    };
    output
}
