use std::path::PathBuf;

use iced::widget::image::Handle;
use image::{self, imageops::FilterType, GenericImageView};
use material_colors::{
    image::{Image as mcImage, ImageReader},
    theme::{Theme, ThemeBuilder},
};
use serde::{Deserialize, Serialize};

use crate::{response_types::Thumbnail, song::Song};

#[derive(Debug, Clone, Default)]
pub enum ThumbnailState {
    #[default]
    Unknown,

    Downloaded {
        path: PathBuf,
        handle: Handle,
        colors: Option<SongTheme>,
    },
}

pub async fn get_thumbnail(
    thumbnail_url: String,
    output: PathBuf,
) -> Result<ThumbnailState, image::ImageError> {
    // let mut material_path = output.clone();
    // material_path.push("_mat");

    // if output.exists() && material_path.exists() {
    //     return Ok((output, material_path));
    // }
    let thumbnail = if !output.exists() {
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
        thumbnail
    } else {
        image::open(&output)?
    };

    let col = ImageReader::extract_color(&mcImage::new(if thumbnail.dimensions() > (128, 128) {
        thumbnail.resize(128, 128, FilterType::Gaussian).into()
    } else {
        thumbnail.into()
    }));

    let theme = ThemeBuilder::with_source(col).build();
    println!["{:?}", theme];

    // Ok((output, theme.into()))
    Ok(ThumbnailState::Downloaded {
        handle: Handle::from_path(&output),
        path: output,
        colors: Some(SongTheme(theme)),
    })
}

#[derive(Debug)]
pub struct SongTheme(Theme);

impl From<Theme> for SongTheme {
    fn from(value: Theme) -> Self {
        Self(value)
    }
}

impl Clone for SongTheme {
    fn clone(&self) -> Self {
        // Ew
        Self(ThemeBuilder::with_source(self.0.source).build())
    }
}
