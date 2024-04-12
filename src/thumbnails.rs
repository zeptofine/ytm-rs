use std::path::PathBuf;

use iced::widget::image::Handle;
use image::{self, GenericImageView};

// use serde::{Deserialize, Serialize};
// use crate::{response_types::Thumbnail, song::Song};

#[derive(Debug, Clone, Default)]
pub enum ThumbnailState {
    #[default]
    Unknown,

    Downloaded {
        path: PathBuf,
        handle: Handle,
    },
}

pub async fn get_thumbnail(
    thumbnail_url: String,
    output: PathBuf,
) -> Result<ThumbnailState, image::ImageError> {
    if !output.exists() {
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
        thumbnail.save(&output)?;
    }
    let handle = Handle::from_path(&output);
    Ok(ThumbnailState::Downloaded {
        handle,
        path: output,
    })
}
