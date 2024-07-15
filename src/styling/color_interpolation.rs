use iced::Color;
use lilt::Interpolable;

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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InterpColor(Color);

impl Interpolable for InterpColor {
    fn interpolated(&self, other: Self, ratio: f32) -> Self {
        Self({
            let c2 = &other.0;
            Color::new(
                map(((&self.0).r, c2.r), ratio),
                map(((&self.0).g, c2.g), ratio),
                map(((&self.0).b, c2.b), ratio),
                map(((&self.0).a, c2.a), ratio),
            )
        })
    }
}
