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

pub fn color_to_argb(color: Color) -> Argb {
    Argb {
        red: (color.r * 255.0) as u8,
        green: (color.g * 255.0) as u8,
        blue: (color.b * 255.0) as u8,
        alpha: (color.a * 255.0) as u8,
    }
}

pub fn pixel_to_argb(pixel: image::Rgba<u8>) -> Argb {
    Argb {
        red: pixel.0[0],
        green: pixel.0[1],
        blue: pixel.0[2],
        alpha: pixel.0[3],
    }
}

pub fn update_button(appearance: button::Appearance, status: button::Status) -> button::Appearance {
    let mut appearance = appearance;
    match status {
        button::Status::Active => {}
        button::Status::Hovered => {
            appearance.border = Border::rounded(2).with_color(Color::WHITE).with_width(2)
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

pub fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powf(3.0)
}

// function easeOutCubic(x: number): number {
//     return 1 - Math.pow(1 - x, 3);

//     }
// same as above, but baked a to (0.0, 1.0)
fn map(b: (f32, f32), x: f32) -> f32 {
    let (min_b, max_b) = b;
    min_b + (x * (max_b - min_b))
}

pub fn interpolate_color(c1: &Color, c2: &Color, t: f32) -> Color {
    Color::new(
        map((c1.r, c2.r), t),
        map((c1.g, c2.g), t),
        map((c1.b, c2.b), t),
        map((c1.a, c2.a), t),
    )
}
