use iced::{
    border,
    widget::{button, container, pick_list, scrollable},
    Background, Border, Color, Theme,
};

type StylyFunc<Status, Style> = Box<dyn Fn(&Theme, Status) -> Style>;

#[derive(Debug, Clone)]
pub struct SongStyle(pub container::Style);
impl Default for SongStyle {
    fn default() -> Self {
        Self(container::Style {
            background: None,
            text_color: Some(Color::WHITE),
            ..Default::default()
        })
    }
}
impl SongStyle {
    pub fn update(&self, selected: bool) -> container::Style {
        let mut style = self.0;
        if selected {
            style.background = Some(Background::Color(Color::from_rgb8(0x32, 0x52, 0x7b)));
        }
        style
    }
}

#[derive(Debug, Clone)]
pub struct ScrollableStyle(pub scrollable::Style);
impl Default for ScrollableStyle {
    fn default() -> Self {
        Self(scrollable::Style {
            container: container::Style {
                text_color: None,
                background: None,
                ..Default::default()
            },
            vertical_rail: scrollable::Rail {
                background: None,
                border: border::rounded(12).width(1).color(Color::TRANSPARENT),
                scroller: scrollable::Scroller {
                    color: Color::TRANSPARENT,
                    border: border::rounded(8),
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(Background::Color(Color::BLACK)),
                border: border::rounded(2).width(1),
                scroller: scrollable::Scroller {
                    color: Color::WHITE,
                    border: border::rounded(8),
                },
            },
            gap: None,
        })
    }
}
impl ScrollableStyle {
    pub fn update(self) -> StylyFunc<scrollable::Status, scrollable::Style> {
        Box::new(move |_t, status| {
            let mut style = self.0;
            match status {
                scrollable::Status::Active => {}
                scrollable::Status::Hovered {
                    is_horizontal_scrollbar_hovered: _,
                    is_vertical_scrollbar_hovered: vert,
                } => {
                    if vert {
                        style.vertical_rail.scroller.color = Color::WHITE;
                        style.vertical_rail.border = style
                            .vertical_rail
                            .border
                            .width(1)
                            .color(Color::new(1., 1., 1., 0.01));
                    } else {
                        style.vertical_rail.scroller.color = Color::new(1., 1., 1., 0.05);
                    }
                }
                scrollable::Status::Dragged {
                    is_horizontal_scrollbar_dragged: _,
                    is_vertical_scrollbar_dragged: vert,
                } => {
                    if vert {
                        style.vertical_rail.scroller.color = Color::WHITE;

                        style.vertical_rail.scroller.border = border::rounded(6);
                        style.vertical_rail.border = border::rounded(8)
                            .width(2)
                            .color(Color::new(1., 1., 1., 0.02));
                    }
                }
            }
            style
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackButtonStyle(pub button::Style);
impl Default for PlaybackButtonStyle {
    fn default() -> Self {
        Self(button::Style {
            text_color: Color::WHITE,
            border: border::rounded(2),
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
    }
}
impl PlaybackButtonStyle {
    pub fn update(self, status: button::Status) -> button::Style {
        let mut style = self.0;
        match status {
            button::Status::Active => {}
            button::Status::Hovered => {
                style.background = Some(Background::Color(Color::new(1., 1., 1., 0.2)));
            }
            button::Status::Pressed => style.background = Some(Background::Color(Color::WHITE)),
            button::Status::Disabled => {}
        }
        style
    }
}

#[derive(Debug, Clone)]
pub struct PickListStyle(pub pick_list::Style);
impl Default for PickListStyle {
    fn default() -> Self {
        Self(pick_list::Style {
            text_color: Color::WHITE,
            placeholder_color: Color::WHITE,
            handle_color: Color::WHITE,
            background: iced::Background::Color(Color::TRANSPARENT),

            border: Border {
                color: Color::new(1., 1., 1., 0.5),
                width: 1.,
                radius: 4.into(),
            },
        })
    }
}
impl From<Color> for PickListStyle {
    fn from(value: Color) -> Self {
        Self(pick_list::Style {
            text_color: Color::WHITE,
            placeholder_color: Color::WHITE,
            handle_color: value,
            background: iced::Background::Color(Color::TRANSPARENT),
            border: border::rounded(4).width(2).color(value),
        })
    }
}
impl PickListStyle {
    pub fn update(self) -> StylyFunc<pick_list::Status, pick_list::Style> {
        Box::new(move |_theme: &Theme, status: pick_list::Status| {
            let mut style = self.0;

            match status {
                pick_list::Status::Active => {}
                pick_list::Status::Hovered => {
                    // style.border.color = Color::WHITE;
                }
                pick_list::Status::Opened => {
                    style.background = Background::Color(Color::WHITE);
                    style.text_color = Color::BLACK;
                }
            }

            style
        })
    }
}
