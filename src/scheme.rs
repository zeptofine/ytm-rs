use std::time::SystemTime;
use std::{path::PathBuf, thread};

use iced::{
    gradient::{ColorStop, Linear},
    widget::button,
    Background, Color, Degrees, Gradient,
};
use image::{imageops::FilterType, io::Reader, GenericImageView};
use material_colors::{color::Argb, quantize::QuantizerWsmeans, score::Score, theme::ThemeBuilder};

use crate::styling::{
    argb_to_color, ease_out_cubic, interpolate_color, pixel_to_argb, SongAppearance,
};
use crate::{BACKGROUND_TRANSITION_DURATION, BACKGROUND_TRANSITION_RATE};

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
            back_start_color: Color::new(0., 0., 0., 1.0),
            back_end_color: Color::new(0., 0., 0., 1.0),
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
            let mut image = Reader::open(path)
                .expect("Failed to open")
                .decode()
                .expect("Failed to decode image");

            // Resizing the image speeds up the process. Little benefit keeping it large
            if image.dimensions() > (128, 128) {
                image = image.resize(128, 128, FilterType::Nearest);
            }

            let result = QuantizerWsmeans::quantize(
                &image
                    .pixels()
                    .map(|p| pixel_to_argb(p.2))
                    .collect::<Vec<_>>(),
                128,
                None,
                None,
                Some(100_000),
                None,
            );
            let scores = Score::score(
                &result.color_to_count,
                Some(2),
                Some(Argb::new(0, 0, 0, 0)),
                None,
            );

            scores[0]
        });

        Self::from_argb(thread.join().unwrap()).await
    }

    pub async fn from_argb(argb: Argb) -> Self {
        let theme = ThemeBuilder::with_source(argb).build();
        let scheme = theme.schemes.dark;

        Self {
            primary_color: argb_to_color(scheme.primary),
            error_color: argb_to_color(scheme.error_container),
            back_start_color: argb_to_color(scheme.surface_container_high),
            back_end_color: argb_to_color(scheme.surface_container_lowest),
            song_appearance: SongAppearance(button::Appearance {
                background: None,
                text_color: Color::WHITE,
                ..Default::default()
            }),
        }
    }

    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        let t = ease_out_cubic(t).clamp(0.0, 1.0);

        Self {
            primary_color: interpolate_color(&self.primary_color, &other.primary_color, t),
            error_color: interpolate_color(&self.error_color, &other.error_color, t),
            back_start_color: interpolate_color(&self.back_start_color, &other.back_start_color, t),
            back_end_color: interpolate_color(&self.back_end_color, &other.back_end_color, t),
            song_appearance: self.song_appearance.clone(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Started {
    pub from: YtmrsScheme,
    pub to: YtmrsScheme,
    pub started: SystemTime,
}

#[derive(Debug, Clone)]
pub struct Transitioning {
    pub from: YtmrsScheme,
    pub to: YtmrsScheme,
    pub value: YtmrsScheme,
    pub started: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct Finished(pub YtmrsScheme);

#[derive(Debug, Clone)]
pub enum SchemeState {
    Started(Box<Started>),
    Transitioning(Box<Transitioning>),
    Finished(Box<Finished>),
}

impl Default for SchemeState {
    fn default() -> Self {
        Self::Finished(Box::default())
    }
}
impl SchemeState {
    pub fn first_choice(&self) -> &YtmrsScheme {
        match self {
            SchemeState::Started(s) => &s.from,
            SchemeState::Transitioning(t) => &t.value,
            SchemeState::Finished(f) => &f.0,
        }
    }
}

pub async fn transition_scheme(state: SchemeState) -> SchemeState {
    match state {
        SchemeState::Started(s) => {
            let now = SystemTime::now();

            let progress = now.duration_since(s.started).unwrap();
            thread::sleep(BACKGROUND_TRANSITION_RATE);

            SchemeState::Transitioning(Box::new(Transitioning {
                value: s.from.interpolate(
                    &s.to,
                    progress.as_millis() as f32 / BACKGROUND_TRANSITION_RATE.as_millis() as f32,
                ),
                from: s.from,
                to: s.to,
                started: s.started,
            }))
        }
        SchemeState::Transitioning(t) => {
            let now = SystemTime::now();
            let progress = now.duration_since(t.started).unwrap();
            if progress < BACKGROUND_TRANSITION_DURATION {
                thread::sleep(BACKGROUND_TRANSITION_RATE);
                let actual_progress = (progress.as_millis() as f32
                    / BACKGROUND_TRANSITION_DURATION.as_millis() as f32)
                    .clamp(0.0, 1.0);
                let transitioned = t.from.interpolate(&t.to, actual_progress);
                SchemeState::Transitioning(Box::new(Transitioning {
                    value: transitioned,
                    ..*t
                }))
            } else {
                SchemeState::Finished(Box::new(Finished(t.to)))
            }
        }
        SchemeState::Finished(_) => todo!(),
    }
    // match state {
    //     SchemeState::Transitioning {``
    //         from,
    //         to,
    //         value: _,
    //         started,
    //     } => {
    //         println!["Transitioning!!!"];
    //         let until_finished = BACKGROUND_TRANSITION_DURATION
    //             - (SystemTime::now().duration_since(started).unwrap());

    //         if until_finished < Duration::ZERO {
    //             SchemeState::Finished
    //         } else {
    //             thread::sleep(BACKGROUND_TRANSITION_RATE);
    //             let progress = until_finished.as_millis();

    //             println!["{:?}", progress];
    //             SchemeState::Transitioning {
    //                 value: Some(
    //                     from.interpolate(
    //                         &to,
    //                         (progress as f32 / BACKGROUND_TRANSITION_DURATION.as_millis() as f32)
    //                             .clamp(0.0, 1.1),
    //                     ),
    //                 ),
    //                 from,
    //                 to,
    //                 started,
    //             }
    //         }
    //     }
    //     SchemeState::Finished => SchemeState::Finished,
    // }
}
