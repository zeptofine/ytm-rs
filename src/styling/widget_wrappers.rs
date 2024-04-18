use iced::{
    overlay::menu,
    widget::{
        button, container, pick_list,
        scrollable::{self, Scrollbar, Scroller},
    },
    Background, Border, Color, Theme,
};

#[derive(Debug, Clone)]
pub struct SongAppearance(pub button::Style);
impl Default for SongAppearance {
    fn default() -> Self {
        Self(button::Style {
            background: None,
            text_color: Color::WHITE,
            ..Default::default()
        })
    }
}
pub fn update_song_button(appearance: button::Style, status: button::Status) -> button::Style {
    let mut appearance = appearance;
    match status {
        button::Status::Active => {}
        button::Status::Hovered => {
            appearance.border = Border::rounded(5)
                .with_color(Color::new(1., 1., 1., 0.025))
                .with_width(2)
        }
        button::Status::Pressed => {}
        button::Status::Disabled => {}
    }
    appearance
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
            vertical_scrollbar: Scrollbar {
                background: None,
                border: Border::rounded(12)
                    .with_width(1)
                    .with_color(Color::TRANSPARENT),
                scroller: Scroller {
                    color: Color::TRANSPARENT,
                    border: Border::rounded(8),
                },
            },
            horizontal_scrollbar: Scrollbar {
                background: None,
                border: Border::rounded(2).with_width(1),
                scroller: Scroller {
                    color: Color::WHITE,
                    border: Border::rounded(8),
                },
            },
            gap: None,
        })
    }
}
pub fn update_scrollable(
    appearance: scrollable::Style,
    status: scrollable::Status,
) -> scrollable::Style {
    let mut appearance = appearance;
    match status {
        scrollable::Status::Active => {}
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered: _,
            is_vertical_scrollbar_hovered: vert,
        } => {
            if vert {
                appearance.vertical_scrollbar.scroller.color = Color::WHITE;
                appearance.vertical_scrollbar.border = Border::rounded(8)
                    .with_width(1)
                    .with_color(Color::new(1., 1., 1., 0.01));
            } else {
                appearance.vertical_scrollbar.scroller.color = Color::new(1., 1., 1., 0.05);
            }
        }
        scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged: _,
            is_vertical_scrollbar_dragged: vert,
        } => {
            if vert {
                appearance.vertical_scrollbar.scroller.color = Color::WHITE;
                appearance.vertical_scrollbar.scroller.border = Border::rounded(6);
                appearance.vertical_scrollbar.border = Border::rounded(8)
                    .with_width(2)
                    .with_color(Color::new(1., 1., 1., 0.02));
            }
        }
    }
    appearance
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
            border: Border::rounded(4)
                .with_width(1)
                .with_color(Color::new(1., 1., 1., 0.5)),
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
            border: Border::rounded(4).with_width(2).with_color(value),
        })
    }
}
impl PickListStyle {
    pub fn update(self) -> Box<dyn Fn(&Theme, pick_list::Status) -> pick_list::Style> {
        Box::new(move |theme: &Theme, status: pick_list::Status| {
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

#[derive(Debug, Clone)]
pub struct PickMenuStyle(pub menu::Style);
impl Default for PickMenuStyle {
    fn default() -> Self {
        Self(menu::Style {
            background: Background::Color(Color::BLACK),
            border: Border::rounded(4).with_width(1).with_color(Color::WHITE),
            text_color: Color::WHITE,
            selected_text_color: Color::BLACK,
            selected_background: Background::Color(Color::WHITE),
        })
    }
}
