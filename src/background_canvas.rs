use iced::{
    gradient::ColorStop,
    mouse,
    widget::{
        canvas::{self, gradient::Linear, Frame, Geometry, Gradient, Path as gPath},
        Canvas,
    },
    Color, Element, Length, Point, Rectangle, Renderer, Theme,
};

#[derive(Debug, Clone)]
pub enum BMessage {}

#[derive(Debug)]
pub struct BackgroundGradient {
    pub start_color: Color,
    pub mid_color: Color,
    pub end_color: Color,
    canvas_cache: canvas::Cache,
}

#[derive(Default, Debug)]
pub struct BackgroundState {}

impl Default for BackgroundGradient {
    fn default() -> Self {
        Self {
            start_color: Color::new(0.0, 0.0, 0.0, 1.0),
            mid_color: Color::new(1.0, 0.0, 0.0, 1.0),
            end_color: Color::new(1.0, 1.0, 1.0, 1.0),
            canvas_cache: canvas::Cache::default(),
        }
    }
}

impl BackgroundGradient {
    pub fn new(start: Color, mid: Color, end: Color) -> Self {
        Self {
            start_color: start,
            mid_color: mid,
            end_color: end,
            canvas_cache: canvas::Cache::default(),
        }
    }

    pub fn view(&self) -> Element<BMessage> {
        Canvas::new(self).width(Length::Fill).height(50).into()
    }

    fn draw(&self, frame: &mut Frame) {
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            frame.size(),
            Linear::new(Point::new(0.0, 0.0), Point::new(frame.width(), 0.0)).add_stops([
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
            ]),
        );
    }
}

impl<BMessage> canvas::Program<BMessage> for BackgroundGradient {
    type State = BackgroundState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<<Renderer as canvas::Renderer>::Geometry> {
        vec![self.canvas_cache.draw(renderer, bounds.size(), |frame| {
            self.draw(frame);
        })]
    }
}
