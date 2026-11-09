use crate::{
    app::{Message, Page, Studio},
    plots,
    preview::Preview,
    theme::{self, action, caption, mono, panel},
};
use iced::widget::rule::horizontal as horizontal_rule;
use iced::{
    Alignment, Element, Fill, Font, Length,
    widget::{
        self, checkbox, column, container, pick_list, progress_bar, row, scrollable, slider, space,
        text, text_editor, text_input,
    },
};
use serein_audio::AudioSource;
use serein_domain::FilterKind;

pub fn shell(app: &Studio) -> Element<'_, Message> {
    let compact = app.window_size.width < 1120.;
    let mut navigation = column![
        text("serein")
            .size(if compact { 23 } else { 30 })
            .color(theme::TEXT),
        caption("WALLPAPERS, MADE YOURS"),
        space().height(26),
    ]
    .spacing(8);
    for page in Page::ALL {
        navigation = navigation.push(theme::navigation(
            if compact && page == Page::Performance {
                "Policy"
            } else if compact && page == Page::Audio {
                "Audio"
            } else {
                page.label()
            },
            Message::Navigate(page),
            app.page == page,
        ));
    }
    navigation = navigation
        .push(space().height(Fill))
        .push(horizontal_rule(1))
        .push(caption(concat!("PREVIEW  /  ", env!("CARGO_PKG_VERSION"))));
    if !compact {
        navigation = navigation
            .push(caption("Choose. Customize. Apply."))
            .push(space().height(8));
    }
    navigation = navigation.push(action("Quit studio", Message::Quit, false));
    let rail = container(navigation)
        .padding(if compact { 12 } else { 22 })
        .width(if compact { 150 } else { 212 })
        .height(Fill)
        .style(|_| container::Style {
            background: Some(theme::SURFACE.into()),
            ..Default::default()
        });

    let top = row![
        action("Commands · Ctrl+K", Message::CommandPalette, false),
        space().width(Fill),
        caption(if app.dirty {
            "● Unsaved changes"
        } else {
            "Saved locally"
        }),
        action("Open project", Message::Open, false),
        action(
            if app.file_busy {
                "Working…"
            } else {
                "Export project"
            },
            Message::Save,
            false
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let title = match app.page {
        Page::Audio => "Shape what your wallpaper hears.",
        Page::Studio => "A canvas for your code.",
        Page::Library => "Make the desktop your own.",
        Page::Customize => "Your wallpaper. Your way.",
        Page::Displays => "One workspace. Every screen.",
        Page::Performance => "Beautiful, within a budget.",
        Page::Roadmap => "Built to grow. Open to contributors.",
    };
    let subtitle = match app.page {
        Page::Audio => "Sculpt a signal, name it, and use it anywhere in your scene.",
        Page::Studio => "Edit WGSL, compile deliberately, and keep the last good preview.",
        Page::Library => "Choose a wallpaper, customize it, and make it your Windows background.",
        Page::Customize => "Adjust the look and response. Code is optional.",
        Page::Displays => {
            "Inspect your actual display layout. Desktop playback is the next milestone."
        }
        Page::Performance => {
            "Explore playback rules. These are policy decisions, not live resource readings."
        }
        Page::Roadmap => "Native wallpapers, local customization and an optional shader workspace.",
    };
    let header = column![
        row![
            text(app.page.label()).size(13).color(theme::ACCENT),
            space().width(Fill),
            caption("PERSONAL WORKSPACE")
        ],
        text(title).size(if compact { 25 } else { 30 }),
        text(subtitle).size(14).color(theme::MUTED)
    ]
    .spacing(8);
    let content = match app.page {
        Page::Audio => audio(app),
        Page::Studio => studio(app),
        Page::Library => crate::library::view(app),
        Page::Customize => crate::library::customize(app),
        Page::Displays => displays(app),
        Page::Performance => performance(app),
        Page::Roadmap => roadmap(),
    };
    let mut body = column![top, space().height(8), header].spacing(16);
    if app.command_open {
        body = body.push(panel(column![text("Run a command").size(18), text_input("frequency 1200", &app.command).id("command-input").on_input(Message::CommandText).on_submit(Message::ExecuteCommand).padding(12), caption("Enter to run · Esc to close · precise values use Hz, dB and ms"), text("Pages: library, customize, studio, audio, displays, performance, roadmap\nActions: apply, pause-wallpaper, stop-wallpaper, import-image, depth-map, help, save, open, compile, preview, quit, reset, undo, refresh\nBand: band 1…7, frequency 20…20000, gain −24…24, q 0.1…18\nFilter: filter bell/lowpass/highpass/lowshelf/highshelf/notch, enabled, bypass\nBus: low 30, high 220, sensitivity 3, attack 25, release 240, busname Bass pulse\nLooks: speed 1, brightness 1, zoom 1, reaction 1, depth 0.035, device default (or its output number)\nOther: source test/system, pointer, spectrum, fps 30, template 1…10, name My scene\nPlayback: fullscreen, battery, pause-wallpaper, stop-wallpaper\nUnsaved work: discard, cancel").size(13), row![action("Run command", Message::ExecuteCommand, true), action("Close commands", Message::Cancel, false)].spacing(12)].spacing(14)));
    }
    if app.pending.is_some() {
        body = body.push(panel(
            column![
                text("This project has unsaved changes.").size(17),
                text("Save the project first, or discard these changes to continue.").size(14),
                row![
                    action("Discard changes", Message::Discard, false),
                    action("Keep editing", Message::Cancel, true)
                ]
                .spacing(12)
            ]
            .spacing(10),
        ));
    }
    body = body.push(content);
    let scroll = scrollable(body.padding(if compact { 20 } else { 28 })).height(Fill);
    let status = container(
        row![
            caption(&app.status),
            space().width(Fill),
            mono(if app.preview_running {
                "Preview running"
            } else {
                "Preview paused"
            })
        ]
        .spacing(12),
    )
    .padding([10, 24])
    .width(Fill)
    .style(theme::inset_style);
    row![rail, column![scroll, status].width(Fill)]
        .height(Fill)
        .into()
}

pub(crate) fn preview_panel(app: &Studio, height: u32) -> Element<'_, Message> {
    let preview = Preview {
        source: app.compiled.clone(),
        appearance: app.project.appearance.clone(),
        image: app.project.image.clone(),
        time: app.preview_time,
        audio: if app.preview_running {
            app.audio_frame.bus
        } else {
            0.
        },
        pointer_enabled: app.pointer && app.preview_running,
    };
    panel(
        column![
            row![
                text("Live canvas").size(17),
                space().width(Fill),
                caption(if app.page == Page::Studio {
                    "WGSL shader"
                } else {
                    "Live preview"
                })
            ]
            .align_y(Alignment::Center),
            widget::shader(preview).width(Fill).height(height as f32),
            text(&app.project.name).size(16),
            caption(if app.preview_running {
                "Live preview of your current settings"
            } else {
                "Static preview · start motion when you are ready"
            }),
            row![
                action(
                    if app.preview_running {
                        "Pause preview"
                    } else {
                        "Run preview"
                    },
                    Message::TogglePreview,
                    false
                ),
                space().width(Fill),
                mono(format!("{} FPS cap", app.project.preview_fps))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            checkbox(app.pointer)
                .label("React to pointer in preview")
                .on_toggle(Message::Pointer),
        ]
        .spacing(14),
    )
    .width(Fill)
    .into()
}

fn audio(app: &Studio) -> Element<'_, Message> {
    let band = app.project.audio.bands[app.selected_band];
    let gain_control = if matches!(
        band.kind,
        FilterKind::Peak | FilterKind::LowShelf | FilterKind::HighShelf
    ) {
        control(
            "Gain",
            format!("{:+.1} dB", band.gain_db),
            slider(-24.0..=24.0, band.gain_db, Message::Gain)
                .step(0.1_f32)
                .into(),
        )
    } else {
        column![text("Gain").size(13), caption("Not used by this filter")]
            .spacing(10)
            .width(Fill)
            .into()
    };
    let mut bands = row![].spacing(8);
    for (i, settings) in app.project.audio.bands.iter().enumerate() {
        bands = bands.push(
            widget::button(
                column![
                    text(format!("{:02}", i + 1))
                        .size(14)
                        .color(theme::BANDS[i]),
                    mono(format_hz(settings.frequency_hz))
                ]
                .spacing(6),
            )
            .width(Fill)
            .padding([12, 6])
            .on_press(Message::SelectBand(i))
            .style(move |_, status| widget::button::Style {
                background: Some(
                    (if i == app.selected_band {
                        theme::HOVER
                    } else {
                        theme::INSET
                    })
                    .into(),
                ),
                text_color: theme::TEXT,
                border: iced::Border {
                    color: if i == app.selected_band || status == widget::button::Status::Hovered {
                        theme::BANDS[i]
                    } else {
                        theme::BORDER
                    },
                    width: 1.,
                    radius: 8.into(),
                },
                ..Default::default()
            }),
        );
    }
    let graph = panel(
        column![
            row![
                text("Analysis equalizer").size(18),
                space().width(Fill),
                checkbox(app.project.audio.bypass)
                    .label("Bypass EQ")
                    .on_toggle(Message::Bypass)
            ]
            .align_y(Alignment::Center)
            .spacing(12),
            row![
                caption(match app.source {
                    AudioSource::TestSignal => "Test signal · 48 kHz / 2048 FFT",
                    AudioSource::SystemOutput => "Live output · 48 kHz / 2048 FFT",
                }),
                space().width(Fill),
                caption("— EQ gain"),
                caption("▱ Spectrum · −90 to 0 dBFS")
            ]
            .spacing(16),
            container(
                widget::canvas(plots::Equalizer {
                    settings: &app.project.audio,
                    audio: &app.audio_frame,
                    selected: app.selected_band,
                    post: app.post
                })
                .width(Fill)
                .height(225)
            )
            .style(theme::inset_style),
            row![
                action(
                    if app.post {
                        "Show pre-EQ spectrum"
                    } else {
                        "Show post-EQ spectrum"
                    },
                    Message::Post(!app.post),
                    false
                ),
                space().width(Fill),
                caption("Drag a node or use the controls below")
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            bands,
            horizontal_rule(1),
            row![
                text(format!("Band {}", app.selected_band + 1))
                    .size(17)
                    .color(theme::BANDS[app.selected_band]),
                pick_list(FilterKind::ALL, Some(band.kind), Message::Filter),
                space().width(Fill),
                checkbox(band.enabled)
                    .label("Enabled")
                    .on_toggle(Message::Enabled)
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            row![
                control(
                    "Frequency",
                    format!("{} Hz", band.frequency_hz.round()),
                    slider(
                        20_f32.log10()..=20000_f32.log10(),
                        band.frequency_hz.log10(),
                        Message::Frequency
                    )
                    .step(0.001_f32)
                    .into()
                ),
                gain_control,
                control(
                    "Q / resonance",
                    format!("{:.2}", band.q),
                    slider(0.1..=18., band.q, Message::Q).step(0.01_f32).into()
                ),
            ]
            .spacing(24),
            row![
                action("Bass focus", Message::EqPreset(1), false),
                action("Voice range", Message::EqPreset(2), false),
                space().width(Fill),
                action("Undo EQ", Message::UndoEq, false)
                    .on_press_maybe((!app.history.is_empty()).then_some(Message::UndoEq)),
                action("Reset EQ", Message::ResetEq, false)
            ]
            .spacing(8),
        ]
        .spacing(16),
    )
    .width(Fill);

    let source = panel(column![
        row![text("Audio output").size(18), space().width(Fill), caption("AUTOMATIC")].align_y(Alignment::Center),
        row![pick_list(app.devices.clone(), Some(app.selected_output.clone()), Message::OutputSelected).width(Fill), action("Refresh devices", Message::RefreshOutputs, false)].spacing(12),
        caption("The default follows Windows output changes. A selected device stays pinned and reconnects automatically."),
        text(&app.audio_status).size(12).color(theme::MUTED),
        row![caption("INPUT WAVEFORM · POST-EQ METERS"), space().width(Fill), mono(format!("RMS {:5.1} dBFS   PEAK {:5.1} dBFS", db(app.audio_frame.rms), db(app.audio_frame.peak)))].spacing(12),
        widget::canvas(plots::Waveform(&app.audio_frame)).height(52).width(Fill),
    ].spacing(14));
    let left = column![source, graph].spacing(18).width(FillPortion(7));
    let right = column![preview_panel(app, 190), bus_panel(app)]
        .spacing(18)
        .width(FillPortion(4));
    if app.window_size.width >= 1240. {
        row![left, right].spacing(20).into()
    } else {
        column![left.width(Fill), right.width(Fill)]
            .spacing(18)
            .into()
    }
}

use iced::Length::FillPortion;

fn bus_panel(app: &Studio) -> Element<'_, Message> {
    let bus = &app.project.audio.bus;
    panel(column![
        row![text("Response bus").size(18), space().width(Fill), mono("01")],
        caption("One signal. Any effect."),
        text("Bus name").size(13),
        text_input("Bass pulse", &bus.name).on_input(Message::BusName).padding(10),
        row![action("Bass", Message::BusPreset(0), false), action("Mids", Message::BusPreset(1), false), action("Air", Message::BusPreset(2), false)].spacing(8),
        control("Low edge", format_hz(bus.low_hz), slider(20_f32.log10()..=19999_f32.log10(), bus.low_hz.log10(), Message::BusLow).step(0.001_f32).into()),
        control("High edge", format_hz(bus.high_hz), slider(21_f32.log10()..=20000_f32.log10(), bus.high_hz.log10(), Message::BusHigh).step(0.001_f32).into()),
        control("Sensitivity", format!("{:.1}×", bus.gain), slider(0.0..=20.0, bus.gain, Message::BusGain).step(0.1_f32).into()),
        row![control("Attack", format!("{:.0} ms", bus.attack_ms), slider(1.0..=1000.0, bus.attack_ms, Message::Attack).step(1_f32).into()), control("Release", format!("{:.0} ms", bus.release_ms), slider(1.0..=2000.0, bus.release_ms, Message::Release).step(1_f32).into())].spacing(20),
        row![caption("NORMALIZED OUTPUT"), space().width(Fill), mono(format!("{:.3}", app.audio_frame.bus)).color(theme::SIGNAL)],
        progress_bar(0.0..=1.0, app.audio_frame.bus).girth(5),
        caption("Already connected to the shader’s audio input. EQ shapes the analysis, never your speakers."),
    ].spacing(14)).width(Fill).into()
}

fn control<'a>(label: &'a str, value: String, input: Element<'a, Message>) -> Element<'a, Message> {
    column![
        row![text(label).size(13), space().width(Fill), mono(value)].spacing(8),
        input
    ]
    .spacing(10)
    .width(Fill)
    .into()
}

fn studio(app: &Studio) -> Element<'_, Message> {
    let code = panel(
        column![
            row![
                text("main.wgsl").font(Font::MONOSPACE).size(15),
                space().width(Fill),
                caption("FRAGMENT SDK v0"),
                action("Compile shader", Message::Compile, true)
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            row![
                caption("Ctrl+Enter compile · Ctrl+S save · Ctrl+Z undo"),
                space().width(Fill),
                action("Undo", Message::UndoCode, false)
                    .on_press_maybe((!app.code_undo.is_empty()).then_some(Message::UndoCode)),
                action("Redo", Message::RedoCode, false)
                    .on_press_maybe((!app.code_redo.is_empty()).then_some(Message::RedoCode))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text_editor(&app.editor)
                .on_action(Message::ShaderEdit)
                .key_binding(|event| {
                    use iced::keyboard::{Key, key::Named};
                    use text_editor::Binding;
                    if matches!(event.status, text_editor::Status::Focused { .. })
                        && event.modifiers.control()
                    {
                        let message = match event.key.as_ref() {
                            Key::Named(Named::Enter) => Some(Message::Compile),
                            Key::Character("s") => Some(Message::Save),
                            Key::Character("k") => Some(Message::CommandPalette),
                            Key::Character("z") => Some(if event.modifiers.shift() {
                                Message::RedoCode
                            } else {
                                Message::UndoCode
                            }),
                            Key::Character("y") => Some(Message::RedoCode),
                            _ => None,
                        };
                        if let Some(message) = message {
                            return Some(Binding::Custom(message));
                        }
                    }
                    Binding::from_key_press(event)
                })
                .font(Font::MONOSPACE)
                .size(14)
                .height(465)
                .padding(16)
                .highlight("wgsl", iced::highlighter::Theme::Base16Ocean),
            text(if app.compile_ok {
                "Shader ready"
            } else {
                "Shader changed or compile failed"
            })
            .size(14)
            .color(if app.compile_ok {
                theme::SIGNAL
            } else {
                theme::WARNING
            }),
            scrollable(mono(&app.diagnostics)).height(Length::Fixed(110.)),
        ]
        .spacing(14),
    )
    .width(FillPortion(7));
    let inspector = panel(column![
        text("Project properties").size(18),
        text("Project name").size(13),
        text_input("Chromatic current", &app.project.name).on_input(Message::Name).padding(10),
        text("Frame ceiling").size(13),
        pick_list([15, 24, 30, 60, 120], Some(app.project.preview_fps), Message::Fps),
        horizontal_rule(1),
        text("Shader inputs").size(16),
        mono("uv       vec2<f32>\ntime     seconds\naudio    0…1 bus value\npointer  0…1 preview UV"),
        caption("Reusable WGSL functions belong above shade(). Only the tested preview ABI is available in this foundation."),
        action("Shape the audio signal", Message::Navigate(Page::Audio), false),
        action("Read the creator guide", Message::OpenGuide, false),
    ].spacing(14));
    let right = column![preview_panel(app, 220), inspector]
        .spacing(18)
        .width(FillPortion(4));
    if app.window_size.width >= 1240. {
        row![code, right].spacing(20).into()
    } else {
        column![code.width(Fill), right.width(Fill)]
            .spacing(18)
            .into()
    }
}

fn displays(app: &Studio) -> Element<'_, Message> {
    let mut monitors = column![
        row![
            text("Connected displays").size(18),
            space().width(Fill),
            action("Refresh displays", Message::RefreshDisplays, false)
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(16);
    for (index, display) in app.displays.iter().enumerate() {
        monitors = monitors.push(panel(
            row![
                mono(format!("{:02}", index + 1))
                    .size(25)
                    .color(theme::ACCENT),
                column![
                    text(&display.name).size(17),
                    mono(format!(
                        "{} × {} pixels · origin ({}, {})",
                        display.width, display.height, display.x, display.y
                    )),
                    caption(if display.primary {
                        "Primary display"
                    } else {
                        "Extended display"
                    })
                ]
                .spacing(8)
            ]
            .spacing(24),
        ));
    }
    column![panel(monitors), crate::library::playback(app), panel(column![text("How display modes work").size(18), text("Primary: use your Windows primary display.
Every display: repeat the wallpaper on each screen.
Span: continue one composition across the entire desktop.
Selected: apply only to the chosen screen.").size(15).line_height(1.7), caption("Changing this choice affects the next Apply wallpaper action. Your normal Windows background remains underneath and returns when playback stops.")].spacing(14))].spacing(20).into()
}

fn performance(app: &Studio) -> Element<'_, Message> {
    column![panel(column![text("Automatic playback").size(20), checkbox(app.preferences.pause_fullscreen).label("Pause for fullscreen or maximized applications").on_toggle(|value| Message::PlaybackRule(0, value)), checkbox(app.preferences.pause_battery).label("Pause on battery power").on_toggle(|value| Message::PlaybackRule(1, value)), text("Frame ceiling").size(14), pick_list([15, 24, 30, 60], Some(app.project.preview_fps), Message::Fps), caption("30 FPS is a balanced default. Pausing retains the current frame while stopping rendering and audio capture in the desktop player.")].spacing(20)), crate::library::playback(app), panel(column![text("Background behavior").size(18), text("Closing or minimizing Serein sends it to the system tray. An applied wallpaper keeps playing in its separate desktop process. Use the tray menu to pause, stop, reopen, or exit.").size(15), caption("The studio suspends its own preview and audio analysis while hidden. The player handles the active wallpaper. No CPU, GPU or energy saving claim is made without a comparable benchmark.")].spacing(14))].spacing(20).into()
}
fn roadmap<'a>() -> Element<'a, Message> {
    column![panel(column![text("Serein").size(28), text("Ten native wallpaper presets, desktop playback, automatic system-audio response, simple visual controls, image parallax and an advanced WGSL workspace.").size(16), text("Getting started").size(20), text("1. Select a wallpaper in Library.
2. Open Customize to adjust its appearance.
3. Select Apply wallpaper.
4. Close or minimize the app; playback continues from the tray.").size(15).line_height(1.7), text("Creator guide").size(20), text("The bundled docs/07-creator-guide.md explains every shader input, reusable functions, audio response, 3D examples and image depth maps. The project README explains how to run and package both binaries.").size(15), caption("Image conversion uses a local depth approximation or your own depth map. It does not infer semantic 3D geometry. Full mesh/timeline editing and video/web players are still future work.")].spacing(20))].into()
}

fn format_hz(hz: f32) -> String {
    if hz >= 1000. {
        format!("{:.1} kHz", hz / 1000.)
    } else {
        format!("{hz:.0} Hz")
    }
}
fn db(value: f32) -> f32 {
    (20. * value.max(1e-5).log10()).max(-90.)
}
