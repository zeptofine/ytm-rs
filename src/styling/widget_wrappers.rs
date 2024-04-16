use iced::{
    widget::{
        button, container,
        scrollable::{self, Scrollbar, Scroller},
    },
    Border, Color,
};

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
                border: Border::rounded(2),
                scroller: Scroller {
                    color: Color::WHITE,
                    border: Border::rounded(2),
                },
            },
            horizontal_scrollbar: Scrollbar {
                background: None,
                border: Border::rounded(2),
                scroller: Scroller {
                    color: Color::WHITE,
                    border: Border::rounded(2),
                },
            },
            gap: None,
        })
    }
}
