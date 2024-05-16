use iced::{
    widget::{button, column, hover, progress_bar, row, slider, text},
    Alignment, Command, Element, Length,
};

use crate::{song::format_duration, styling::FullYtmrsScheme};

use super::YTMRSAudioManager;

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

    pub fn view(&self, _scheme: &FullYtmrsScheme) -> Element<TrackerMsg> {
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

        let next_button = button(">|").on_press(TrackerMsg::Next);
        let previous_button = button("|<").on_press(TrackerMsg::Previous);

        let pause_play_button = {
            let (button_str, button_message) = if self.paused {
                ("||", TrackerMsg::Play)
            } else {
                ("▶", TrackerMsg::Pause)
            };

            button(button_str).on_press(button_message)
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
            TrackerMsg::ProgressSliderReleased(_) => todo!(),
        }
    }
}
