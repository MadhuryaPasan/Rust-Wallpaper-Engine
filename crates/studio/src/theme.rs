use iced::{
    Border, Color, Element, Font, Theme,
    widget::{button, container, text},
};

pub const BACKGROUND: Color = Color::from_rgb8(19, 21, 25);
pub const SURFACE: Color = Color::from_rgb8(27, 30, 35);
pub const INSET: Color = Color::from_rgb8(15, 18, 22);
pub const HOVER: Color = Color::from_rgb8(42, 46, 53);
pub const BORDER: Color = Color::from_rgb8(66, 73, 83);
pub const TEXT: Color = Color::from_rgb8(239, 239, 235);
pub const MUTED: Color = Color::from_rgb8(170, 178, 190);
pub const ACCENT: Color = Color::from_rgb8(255, 163, 139);
pub const SIGNAL: Color = Color::from_rgb8(119, 218, 218);
pub const WARNING: Color = Color::from_rgb8(244, 205, 130);
pub const BANDS: [Color; 7] = [
    Color::from_rgb8(190, 165, 255),
    Color::from_rgb8(242, 155, 193),
    Color::from_rgb8(255, 163, 139),
    Color::from_rgb8(238, 218, 140),
    Color::from_rgb8(166, 216, 153),
    SIGNAL,
    Color::from_rgb8(149, 187, 255),
];

pub fn app_theme() -> Theme {
    Theme::custom(
        "Serein",
        iced::theme::Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: SIGNAL,
            warning: WARNING,
            danger: Color::from_rgb8(255, 125, 140),
        },
    )
}

pub fn panel_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        text_color: Some(TEXT),
        border: Border {
            color: BORDER,
            width: 1.,
            radius: 12.into(),
        },
        ..Default::default()
    }
}
pub fn inset_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(INSET.into()),
        text_color: Some(TEXT),
        border: Border {
            radius: 8.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn panel<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> container::Container<'a, Message> {
    container(content).padding(20).style(panel_style)
}
pub fn caption<'a>(value: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(value).size(12).color(MUTED)
}
pub fn mono<'a>(value: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(value).font(Font::MONOSPACE).size(13)
}

pub fn action<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    message: Message,
    primary: bool,
) -> button::Button<'a, Message> {
    button(text(label).size(14))
        .padding([10, 16])
        .on_press(message)
        .style(move |_: &Theme, status| {
            let focused = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(
                    (if primary {
                        ACCENT
                    } else if focused {
                        HOVER
                    } else {
                        SURFACE
                    })
                    .into(),
                ),
                text_color: if status == button::Status::Disabled {
                    MUTED
                } else if primary {
                    INSET
                } else {
                    TEXT
                },
                border: Border {
                    color: if focused { ACCENT } else { BORDER },
                    width: if primary { 0. } else { 1. },
                    radius: 8.into(),
                },
                ..Default::default()
            }
        })
}

pub fn navigation<'a, Message: Clone + 'a>(
    label: &'a str,
    message: Message,
    active: bool,
) -> button::Button<'a, Message> {
    button(
        text(if active {
            format!("│  {label}")
        } else {
            format!("    {label}")
        })
        .size(14),
    )
    .padding([12, 10])
    .width(iced::Fill)
    .on_press(message)
    .style(move |_: &Theme, status| button::Style {
        background: Some(
            (if active || status == button::Status::Hovered {
                HOVER
            } else {
                SURFACE
            })
            .into(),
        ),
        text_color: if active { ACCENT } else { MUTED },
        border: Border {
            color: if status == button::Status::Hovered {
                ACCENT
            } else {
                Color::TRANSPARENT
            },
            width: 1.,
            radius: 8.into(),
        },
        ..Default::default()
    })
}
