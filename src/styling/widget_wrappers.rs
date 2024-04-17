use iced::{
    widget::{
        button, container,
        scrollable::{self, Scrollbar, Scroller},
    },
    Border, Color,
};

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

pub fn update_button(appearance: button::Appearance, status: button::Status) -> button::Appearance {
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
pub struct ScrollableAppearance(pub scrollable::Appearance);
impl Default for ScrollableAppearance {
    fn default() -> Self {
        Self(scrollable::Appearance {
            container: container::Appearance {
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
    appearance: scrollable::Appearance,
    status: scrollable::Status,
) -> scrollable::Appearance {
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
