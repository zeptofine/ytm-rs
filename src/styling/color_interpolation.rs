use iced::Color;

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
