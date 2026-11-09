#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod library;
mod local_library;
mod platform;
mod plots;
mod preferences;
mod preview;
mod runtime;
mod theme;
mod views;

fn main() -> iced::Result {
    let compact = std::env::args().any(|argument| argument == "--compact");
    iced::application(app::Studio::boot, app::Studio::update, app::Studio::view)
        .title(app::Studio::title)
        .theme(|_: &app::Studio| theme::app_theme())
        .subscription(app::Studio::subscription)
        .window(iced::window::Settings {
            size: if compact {
                iced::Size::new(1000., 740.)
            } else {
                iced::Size::new(1280., 780.)
            },
            position: iced::window::Position::Centered,
            min_size: Some(iced::Size::new(960., 700.)),
            exit_on_close_request: false,
            ..Default::default()
        })
        .default_font(iced::Font::with_name("Segoe UI"))
        .run()
}
