use crate::{
    app::{Message, Page, Studio},
    theme::{self, action, caption, mono, panel},
};
use iced::{
    Alignment, Element, Fill,
    widget::{self, column, container, pick_list, row, slider, space, text, text_input},
};
use serein_render::catalog::PRESETS;

pub fn thumbnails() -> Vec<widget::image::Handle> {
    [
        include_bytes!("../assets/catalog/current.png").as_slice(),
        include_bytes!("../assets/catalog/contour.png").as_slice(),
        include_bytes!("../assets/catalog/dunes.png").as_slice(),
        include_bytes!("../assets/catalog/lattice.png").as_slice(),
        include_bytes!("../assets/catalog/aurora.png").as_slice(),
        include_bytes!("../assets/catalog/prism.png").as_slice(),
    ]
    .into_iter()
    .map(widget::image::Handle::from_bytes)
    .collect()
}

pub fn view(app: &Studio) -> Element<'_, Message> {
    let mut grid = column![
        row![
            text("Included wallpapers").size(18),
            space().width(Fill),
            caption(format!("{} native presets", PRESETS.len()))
        ]
        .align_y(Alignment::Center),
        caption("Select a wallpaper to preview it. Your customizations are saved automatically.")
    ]
    .spacing(16);
    let columns = if app.window_size.width >= 1750. { 3 } else { 2 };
    if !app.local_wallpapers.is_empty() {
        let mut own = column![text("Your wallpapers").size(18)].spacing(16);
        for chunk in (0..app.local_wallpapers.len())
            .collect::<Vec<_>>()
            .chunks(columns)
        {
            let mut cards = row![].spacing(16);
            for &index in chunk {
                let wallpaper = &app.local_wallpapers[index];
                let selected = app.selected_local.as_ref() == Some(&wallpaper.id);
                cards = cards.push(
                    widget::button(
                        column![
                            widget::image(wallpaper.thumbnail.clone())
                                .width(Fill)
                                .height(140)
                                .content_fit(iced::ContentFit::Cover),
                            text(&wallpaper.project.name).size(17),
                            caption(if selected {
                                "Image parallax · selected"
                            } else {
                                "Image parallax"
                            })
                        ]
                        .spacing(10),
                    )
                    .on_press(Message::LocalWallpaper(index))
                    .padding(12)
                    .width(Fill)
                    .style(move |_, status| widget::button::Style {
                        background: Some(theme::SURFACE.into()),
                        text_color: theme::TEXT,
                        border: iced::Border {
                            color: if selected || status == widget::button::Status::Hovered {
                                theme::ACCENT
                            } else {
                                theme::BORDER
                            },
                            width: if selected { 2. } else { 1. },
                            radius: 12.into(),
                        },
                        ..Default::default()
                    }),
                );
            }
            for _ in chunk.len()..columns {
                cards = cards.push(space().width(Fill));
            }
            own = own.push(cards);
        }
        grid = column![own, grid].spacing(26);
    }
    for chunk in (0..PRESETS.len()).collect::<Vec<_>>().chunks(columns) {
        let mut cards = row![].spacing(16);
        for &index in chunk {
            let preset = &PRESETS[index];
            let selected = app.selected_preset == Some(index);
            cards = cards.push(
                widget::button(
                    column![
                        widget::image(app.thumbnails[index].clone())
                            .width(Fill)
                            .height(140)
                            .content_fit(iced::ContentFit::Cover),
                        row![
                            text(preset.name).size(17),
                            space().width(Fill),
                            caption(if selected { "Selected" } else { "" })
                        ]
                        .spacing(8),
                        caption(preset.category),
                        text(preset.description).size(13).color(theme::MUTED),
                    ]
                    .spacing(10),
                )
                .on_press(Message::Starter(index))
                .padding(12)
                .width(Fill)
                .style(move |_, status| widget::button::Style {
                    background: Some(theme::SURFACE.into()),
                    text_color: theme::TEXT,
                    border: iced::Border {
                        color: if selected || status == widget::button::Status::Hovered {
                            theme::ACCENT
                        } else {
                            theme::BORDER
                        },
                        width: if selected { 2. } else { 1. },
                        radius: 12.into(),
                    },
                    ..Default::default()
                }),
            );
        }
        for _ in chunk.len()..columns {
            cards = cards.push(space().width(Fill));
        }
        grid = grid.push(cards);
    }
    let gallery = column![
        row![
            action("Import an image", Message::ImportImage(false), false),
            action("Open a project", Message::Open, false)
        ]
        .spacing(12),
        grid
    ]
    .spacing(22)
    .width(Fill);
    let selected = column![
        playback(app),
        crate::views::preview_panel(app, 180),
        panel(
            column![
                text("Make it yours").size(18),
                caption("Adjust color, speed, brightness and audio response without editing code."),
                action(
                    "Customize wallpaper",
                    Message::Navigate(Page::Customize),
                    false
                ),
                action(
                    "Edit code · Advanced",
                    Message::Navigate(Page::Studio),
                    false
                )
            ]
            .spacing(14)
        )
    ]
    .spacing(18)
    .width(Fill);
    if app.window_size.width >= 1200. {
        row![
            gallery.width(iced::Length::FillPortion(7)),
            selected.width(iced::Length::FillPortion(4))
        ]
        .spacing(20)
        .into()
    } else {
        column![playback(app), gallery, selected].spacing(20).into()
    }
}

pub fn playback(app: &Studio) -> Element<'_, Message> {
    let mut content = column![
        text(if app.runtime.active() {
            "Desktop wallpaper"
        } else {
            "Ready for your desktop"
        })
        .size(18),
        text(&app.runtime.status)
            .size(13)
            .color(if app.runtime.running {
                theme::SIGNAL
            } else {
                theme::MUTED
            }),
        text("Apply to").size(13),
        pick_list(
            serein_domain::DisplayMode::ALL,
            Some(app.preferences.display_mode),
            Message::DisplayMode
        ),
    ]
    .spacing(14);
    if app.preferences.display_mode == serein_domain::DisplayMode::Selected {
        content = content.push(pick_list(
            app.displays
                .iter()
                .map(|display| display.name.clone())
                .collect::<Vec<_>>(),
            app.preferences.monitor_name.clone(),
            Message::Monitor,
        ));
    }
    content = content.push(action("Apply wallpaper", Message::ApplyWallpaper, true).width(Fill));
    if app.runtime.active() {
        content = content.push(
            row![
                action(
                    if app.runtime.session.as_ref().is_some_and(|s| s.paused) {
                        "Resume wallpaper"
                    } else {
                        "Pause wallpaper"
                    },
                    Message::PauseWallpaper,
                    false
                ),
                action("Stop wallpaper", Message::StopWallpaper, false)
            ]
            .spacing(8),
        );
    }
    panel(content).width(Fill).into()
}

fn parameter<'a>(
    label: &'a str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    index: usize,
    suffix: &'a str,
) -> Element<'a, Message> {
    column![
        row![
            text(label).size(14),
            space().width(Fill),
            mono(if index == 4 {
                format!("{value:.3}{suffix}")
            } else {
                format!("{value:.2}{suffix}")
            })
        ],
        slider(range, value, move |v| Message::Appearance(index, v)).step(if index == 4 {
            0.001_f32
        } else {
            0.01_f32
        })
    ]
    .spacing(12)
    .into()
}

pub fn customize(app: &Studio) -> Element<'_, Message> {
    let look = &app.project.appearance;
    let speed: Element<'_, Message> = if app.project.image.is_some() {
        caption("Motion follows your mouse and audio.").into()
    } else {
        parameter("Animation speed", look.speed, 0.0..=4.0, 0, "×")
    };
    let controls = panel(
        column![
            text("Appearance").size(20),
            caption("Changes update an applied wallpaper automatically."),
            text("Wallpaper name").size(14),
            text_input("Wallpaper name", &app.project.name)
                .on_input(Message::Name)
                .padding(10),
            speed,
            parameter("Brightness", look.brightness, 0.1..=2.0, 1, "×"),
            parameter(
                "Scene zoom",
                look.detail,
                if app.project.image.is_some() {
                    1.0..=3.0
                } else {
                    0.5..=3.0
                },
                2,
                "×"
            ),
            parameter("Audio reaction", look.audio_amount, 0.0..=5.0, 3, "×"),
            parameter("Mouse depth", look.parallax, 0.0..=0.12, 4, ""),
            text("Color treatment").size(14),
            pick_list(
                ["Original", "Ember", "Arctic", "Violet"],
                Some(["Original", "Ember", "Arctic", "Violet"][look.palette.min(3) as usize]),
                |name| Message::Palette(match name {
                    "Ember" => 1,
                    "Arctic" => 2,
                    "Violet" => 3,
                    _ => 0,
                })
            ),
            text("Frame ceiling").size(14),
            pick_list(
                [15, 24, 30, 60],
                Some(app.project.preview_fps),
                Message::Fps
            ),
            action(
                "Fine-tune the equalizer",
                Message::Navigate(Page::Audio),
                false
            ),
        ]
        .spacing(20),
    )
    .width(Fill);
    let mut content = column![playback(app), crate::views::preview_panel(app, 280)]
        .spacing(20)
        .width(Fill);
    if let Some(image) = &app.project.image {
        content = content.push(panel(column![text("Image depth").size(18), text(if image.depth_path.is_some() { "Using your depth map · white is near, black is far" } else { "Automatic depth approximation · 2.5D parallax, not AI depth estimation" }).size(14), caption("Keep the depth amount subtle to avoid stretching. You can supply a depth map for more control."), action("Choose a depth map", Message::ImportImage(true), false), action("Choose another image", Message::ImportImage(false), false)].spacing(14)));
    }
    content = content.push(
        container(action(
            "Edit shader · Advanced",
            Message::Navigate(Page::Studio),
            false,
        ))
        .padding(4),
    );
    if app.window_size.width >= 1200. {
        row![content, controls].spacing(20).into()
    } else {
        column![content, controls].spacing(20).into()
    }
}
