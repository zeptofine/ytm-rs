use iced::{
    advanced,
    widget::{button, column, hover, progress_bar, row, slider, text, Svg},
    Alignment, Command, Element, Length,
};
use once_cell::sync::Lazy;

use crate::{song::format_duration, styling::FullYtmrsScheme};

use super::YTMRSAudioManager;

static PLAY_SVG: Lazy<advanced::svg::Handle> = Lazy::new(|| {
    advanced::svg::Handle::from_memory(include_bytes!(
        "../../assets/play_arrow_40dp_FILL0_wght400_GRAD0_opsz40.svg"
    ))
});
static PAUSE_SVG: Lazy<advanced::svg::Handle> = Lazy::new(|| {
    advanced::svg::Handle::from_memory(include_bytes!(
        "../../assets/pause_40dp_FILL0_wght400_GRAD0_opsz40.svg"
    ))
});
static SKIP_NEXT_SVG: Lazy<advanced::svg::Handle> = Lazy::new(|| {
    advanced::svg::Handle::from_memory(include_bytes!(
        "../../assets/skip_next_40dp_FILL0_wght400_GRAD0_opsz40.svg"
    ))
});
static SKIP_PREVIOUS_SVG: Lazy<advanced::svg::Handle> = Lazy::new(|| {
    advanced::svg::Handle::from_memory(include_bytes!(
        "../../assets/skip_previous_40dp_FILL0_wght400_GRAD0_opsz40.svg"
    ))
});

#[derive(Debug, Clone)]
pub enum TrackerMsg {
    Pause,
    Play,
    Next,
    Previous,
    UpdateVolume(f32),
    ProgressSliderChanged(f32),
    ProgressSliderReleased(f32),
}

/// A struct that shows the progress of the manager's audio playback.
#[derive(Debug)]
pub struct AudioProgressTracker {
    pub elapsed: Option<f32>,
    pub total: Option<f32>,
    pub paused: bool,
    pub volume: f32,
    pub next_available: bool,
    pub previous_available: bool,
}
impl Default for AudioProgressTracker {
    fn default() -> Self {
        AudioProgressTracker {
            elapsed: None,
            total: None,
            paused: false,
            volume: 1.,
            next_available: true,
            previous_available: false,
        }
    }
}

impl AudioProgressTracker {
    pub fn update_from_manager(&mut self, manager: &YTMRSAudioManager) {
        self.elapsed = manager.elapsed();
        self.total = manager.total();
        self.paused = manager.is_paused();
        self.volume = manager.volume();
    }

    pub fn view(&self, scheme: &FullYtmrsScheme) -> Element<TrackerMsg> {
        let elapsed = self.elapsed.unwrap_or(0.0);
        let range = 0.0..=self.total.unwrap_or(1.0);

        let duration_display = text(format!(
            "{} / {}",
            format_duration(&elapsed),
            format_duration(range.end())
        ));
        let progress_bar = hover(
            progress_bar(range.clone(), elapsed).height(10),
            slider(range, elapsed, TrackerMsg::ProgressSliderChanged)
                .on_release(TrackerMsg::ProgressSliderReleased(elapsed))
                .height(10),
        );

        let next_button = {
            let button_style = scheme.playback_button_style.clone();
            button(Svg::new(SKIP_NEXT_SVG.clone()).width(32).height(32))
                .on_press(TrackerMsg::Next)
                .style(move |_, s| button_style.clone().update(s))
        };
        let previous_button = {
            let button_style = scheme.playback_button_style.clone();
            button(Svg::new(SKIP_PREVIOUS_SVG.clone()).width(32).height(32))
                .on_press(TrackerMsg::Previous)
                .style(move |_, s| button_style.clone().update(s))
        };

        let pause_play_button = {
            let (button_image, button_message) = if self.paused {
                (PLAY_SVG.clone(), TrackerMsg::Play)
            } else {
                (PAUSE_SVG.clone(), TrackerMsg::Pause)
            };
            let button_style = scheme.playback_button_style.clone();

            button(Svg::new(button_image).width(32).height(32))
                .on_press(button_message)
                .style(move |_, s| button_style.clone().update(s))
        };

        let volume_slider = slider(0.0..=1.0, self.volume, TrackerMsg::UpdateVolume);
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
