use std::path::PathBuf;

use iced::widget::image::Handle;
use image::{self, imageops::FilterType, GenericImageView};
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

pub async fn get_thumbnail(
    thumbnail_url: String,
    output: PathBuf,
) -> Result<(PathBuf, PathBuf), image::ImageError> {
    let mut material_path = output.clone();
    material_path.push("_mat");

    if output.exists() && material_path.exists() {
        return Ok((output, material_path));
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
    thumbnail.save(&output)?;

    // let theme = mc::Image::new(if thumbnail.dimensions() > (128, 128) {
    //     thumbnail.resize(128, 128, FilterType::Gaussian).into()
    // } else {
    //     thumbnail.into()
    // });

    Ok((output, material_path))
}
