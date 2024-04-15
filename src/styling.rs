use iced::{widget::button, Border, Color};
use material_colors::color::Argb;

pub fn argb_to_color(argb: Argb) -> Color {
    Color {
        r: argb.red as f32 / 255.0,
        g: argb.green as f32 / 255.0,
        b: argb.blue as f32 / 255.0,
        a: argb.alpha as f32 / 255.0,
    }
}

pub fn update_button(appearance: button::Appearance, status: button::Status) -> button::Appearance {
    let mut appearance = appearance;
    match status {
        button::Status::Active => {}
        button::Status::Hovered => {
            appearance.border = Border::rounded(2).with_color(Color::WHITE).with_width(10)
        }
        button::Status::Pressed => {}
        button::Status::Disabled => {}
    }
    appearance
}

#[derive(Debug, Clone)]
pub struct SongAppearance(pub button::Appearance);
impl Default for SongAppearance {
    fn default() -> Self {
        Self(button::Appearance {
            background: None,
            text_color: Color::WHITE,
            ..Default::default()
        })
    }
}
