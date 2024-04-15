use iced::{
    gradient::{ColorStop, Linear},
    Background, Color,
};

#[derive(Debug, Clone)]
pub struct BackgroundGradient {
    pub start_color: Color,
    pub mid_color: Color,
    pub end_color: Color,
}

impl Default for BackgroundGradient {
    fn default() -> Self {
        Self {
            start_color: Color::new(0.0, 0.0, 0.0, 1.0),
            mid_color: Color::new(1.0, 0.0, 0.0, 1.0),
            end_color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl BackgroundGradient {
    pub fn new(start: Color, mid: Color, end: Color) -> Self {
        Self {
            start_color: start,
            mid_color: mid,
            end_color: end,
        }
    }
    pub fn to_background(&self) -> Background {
        Background::Gradient(iced::Gradient::Linear(Linear::new(0).add_stops([
            ColorStop {
                offset: 0.0,
                color: self.start_color,
            },
            ColorStop {
                offset: 0.5,
                color: self.mid_color,
            },
            ColorStop {
                offset: 1.0,
                color: self.end_color,
            },
        ])))
    }
}
