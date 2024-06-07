use super::YTMRSAudioManager;
use crate::{settings::YTMRUserSettings, song::format_duration, styling::FullYtmrsScheme};
use iced::{
    alignment::Vertical,
    widget::{button, column, container, hover, progress_bar, row, slider, Text},
    Alignment, Border, Color, Command, Element, Length,
};

#[cfg(feature = "svg")]
use iced::widget::Svg;
use kira::sound::PlaybackState;

#[cfg(feature = "svg")]
#[inline]
fn pause_play_button(playing: bool) -> (Svg<'static>, TrackerMsg) {
    match playing {
        true => (Svg::new(crate::audio::PAUSE_SVG.clone()), TrackerMsg::Pause),
        false => (Svg::new(crate::audio::PLAY_SVG.clone()), TrackerMsg::Play),
    }
}
#[cfg(not(feature = "svg"))]
#[inline]
fn pause_play_button<'a>(playing: bool) -> (Text<'a>, TrackerMsg) {
    use iced::alignment::Horizontal;

    match playing {
        true => (
            Text::new("||").horizontal_alignment(Horizontal::Center),
            TrackerMsg::Pause,
        ),
        false => (
            Text::new(">").horizontal_alignment(Horizontal::Center),
            TrackerMsg::Play,
        ),
    }
}
#[cfg(feature = "svg")]
#[inline]
fn next_button() -> Svg<'static> {
    Svg::new(crate::audio::SKIP_NEXT_SVG.clone())
}
#[cfg(not(feature = "svg"))]
#[inline]
fn next_button() -> Text<'static> {
    Text::new(">|").horizontal_alignment(iced::alignment::Horizontal::Center)
}
#[cfg(feature = "svg")]
#[inline]
fn previous_button() -> Svg<'static> {
    use iced::{Radians, Rotation};
    next_button().rotation(Rotation::Floating(Radians::PI)) // Rotate 180 deg
}
#[cfg(not(feature = "svg"))]
#[inline]
fn previous_button() -> Text<'static> {
    Text::new("|<").horizontal_alignment(iced::alignment::Horizontal::Center)
}

#[derive(Debug, Clone)]
pub enum TrackerMsg {
    Pause,
    Play,
    Next,
    Previous,
    UpdateVolume(f64),
    ProgressSliderChanged(f64),
    ProgressSliderReleased(f64),
}

/// A struct that shows the progress of the manager's audio playback.
#[derive(Debug, Clone)]
pub struct AudioProgressTracker {
    pub elapsed: Option<f64>,
    pub total: Option<f64>,
    pub paused: bool,
    pub volume: f64,
    pub next_available: bool,
    pub previous_available: bool,
}
impl Default for AudioProgressTracker {
    fn default() -> Self {
        AudioProgressTracker {
            elapsed: None,
            total: None,
            paused: false,
            volume: 1000.,
            next_available: true,
            previous_available: false,
        }
    }
}

impl AudioProgressTracker {
    pub fn new(settings: &YTMRUserSettings) -> Self {
        Self {
            volume: settings.volume as f64 * 1000_f64,
            ..Default::default()
        }
    }

    pub fn update_from_manager(&mut self, manager: &YTMRSAudioManager) {
        self.elapsed = manager.elapsed();
        self.total = manager.total().map(|d| d.as_secs_f64());
        self.paused = manager.playback_state() == PlaybackState::Paused;
    }

    pub fn view(&self, scheme: &FullYtmrsScheme) -> Element<TrackerMsg> {
        let elapsed = self.elapsed.unwrap_or(0.0) as f32;
        let range = 0.0..=self.total.unwrap_or(1.0) as f32;

        let duration_display = Text::new(format!(
            "{} / {}",
            format_duration(&elapsed),
            format_duration(range.end())
        ));
        let progress_color = scheme.colors.primary_color;
        let progress_bar = hover(
            container(
                progress_bar(range.clone(), elapsed)
                    .height(8)
                    .style(move |_| progress_bar::Style {
                        background: iced::Background::Color(Color::BLACK),
                        bar: iced::Background::Color(progress_color),
                        border: Border::rounded(0),
                    }),
            )
            .align_y(Vertical::Center)
            .height(10),
            slider(range, elapsed, |x| {
                TrackerMsg::ProgressSliderChanged(x as f64)
            })
            .on_release(TrackerMsg::ProgressSliderReleased(elapsed as f64))
            .height(10),
        );

        let next_button = {
            let button_style = scheme.playback_button_style.clone();
            button(next_button().width(32).height(32))
                .on_press(TrackerMsg::Next)
                .style(move |_, s| button_style.clone().update(s))
        };
        let previous_button = {
            let button_style = scheme.playback_button_style.clone();

            button(previous_button().width(32).height(32))
                .on_press(TrackerMsg::Previous)
                .style(move |_, s| button_style.clone().update(s))
        };

        let pause_play_button = {
            let (button_image, button_message) = pause_play_button(!self.paused);
            let button_style = scheme.playback_button_style.clone();

            button(button_image.width(32).height(32))
                .on_press(button_message)
                .style(move |_, s| button_style.clone().update(s))
        };

        let volume_slider = slider(0.0..=1000.0, self.volume, TrackerMsg::UpdateVolume);
        Element::new(
            column![
                progress_bar,
                row![
                    row![duration_display].width(Length::Fill),
                    column![row![previous_button, pause_play_button, next_button]]
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                    column![volume_slider.width(100)]
                        .align_items(Alignment::End)
                        .width(Length::Fill),
                ]
                .padding(10)
                .align_items(Alignment::Center)
            ]
            .width(Length::Fill),
        )
    }

    pub fn update(&mut self, signal: TrackerMsg) -> Command<TrackerMsg> {
        match signal {
            TrackerMsg::ProgressSliderChanged(v) => {
                self.elapsed = Some(v);
                Command::none()
            }
            TrackerMsg::Pause => todo!(),
            TrackerMsg::Play => todo!(),
            TrackerMsg::Next => todo!(),
            TrackerMsg::Previous => todo!(),
            TrackerMsg::UpdateVolume(_) => todo!(),
            TrackerMsg::ProgressSliderReleased(_) => Command::none(),
        }
    }
}
