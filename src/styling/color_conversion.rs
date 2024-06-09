use iced::Color;

use material_colors::color::Argb;

pub fn argb_to_color(argb: Argb) -> Color {
    Color {
        r: argb.red as f32 / 255.0,
        g: argb.green as f32 / 255.0,
        b: argb.blue as f32 / 255.0,
        a: argb.alpha as f32 / 255.0,
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
