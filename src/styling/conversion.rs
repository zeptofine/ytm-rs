use iced::Color;
use material_colors::color::Argb;

#[allow(unused)]
pub fn argb_to_color(argb: Argb) -> Color {
    Color {
        r: argb.red as f32 / 255.0,
        g: argb.green as f32 / 255.0,
        b: argb.blue as f32 / 255.0,
        a: argb.alpha as f32 / 255.0,
    }
}

#[allow(unused)]
pub fn color_to_argb(color: Color) -> Argb {
    Argb {
        red: (color.r * 255.0) as u8,
        green: (color.g * 255.0) as u8,
        blue: (color.b * 255.0) as u8,
        alpha: (color.a * 255.0) as u8,
    }
}

#[allow(unused)]
pub fn pixel_to_argb(pixel: image::Rgba<u8>) -> Argb {
    Argb {
        red: pixel.0[0],
        green: pixel.0[1],
        blue: pixel.0[2],
        alpha: pixel.0[3],
    }
}

pub fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powf(3.0)
}

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
