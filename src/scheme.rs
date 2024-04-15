use std::{path::PathBuf, thread};

use iced::{
    gradient::{ColorStop, Linear},
    widget::button,
    Background, Color, Degrees, Gradient,
};
use material_colors::{image::ImageReader, theme::ThemeBuilder};

use crate::styling::{argb_to_color, SongAppearance};

#[derive(Debug, Clone)]
pub struct YtmrsScheme {
    pub primary_color: Color,
    pub song_appearance: SongAppearance,
    pub error_color: Color,
    pub back_start_color: Color,
    pub back_end_color: Color,
}

impl Default for YtmrsScheme {
    fn default() -> Self {
        Self {
            primary_color: Color::new(1.0, 1.0, 1.0, 1.0),
            error_color: Color::new(1.0, 1.0, 1.0, 1.0),
            back_start_color: Color::new(0.0, 0.0, 0.0, 1.0),
            back_end_color: Color::new(1.0, 1.0, 1.0, 1.0),
            song_appearance: SongAppearance::default(),
        }
    }
}

impl YtmrsScheme {
    pub fn to_background(&self) -> Background {
        Background::Gradient(Gradient::Linear(Linear::new(Degrees(180.0)).add_stops([
            ColorStop {
                offset: 0.0,
                color: self.back_start_color,
            },
            ColorStop {
                offset: 1.0,
                color: self.back_end_color,
            },
        ])))
    }

    pub async fn from_image(path: PathBuf) -> Self {
        // I dont even know if wrapping this in a thread does anything but it
        // doesnt seem to block the UI so I'm happy
        let thread = thread::spawn(move || {
            let image = ImageReader::open(path).expect("Image failed to load");

            let theme = ThemeBuilder::with_source(ImageReader::extract_color(&image)).build();
            let scheme = theme.schemes.dark;

            Self {
                primary_color: argb_to_color(scheme.primary),
                error_color: argb_to_color(scheme.error_container),
                back_start_color: argb_to_color(scheme.surface_container_high),
                back_end_color: argb_to_color(scheme.surface_container),
                song_appearance: SongAppearance(button::Appearance {
                    background: None,
                    text_color: Color::WHITE,
                    ..Default::default()
                }),
            }
        });

        thread.join().unwrap()
    }
}
