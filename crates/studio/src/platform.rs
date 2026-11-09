pub use serein_platform::displays;

#[cfg(windows)]
pub struct Tray {
    _icon: tray_icon::TrayIcon,
    show: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
    pause: tray_icon::menu::MenuId,
    stop: tray_icon::menu::MenuId,
}
#[cfg(not(windows))]
pub struct Tray;
pub enum TrayAction {
    Show,
    Quit,
    Pause,
    Stop,
}

impl Tray {
    #[cfg(windows)]
    pub fn new() -> Result<Self, String> {
        use tray_icon::{
            Icon, TrayIconBuilder,
            menu::{Menu, MenuItem},
        };
        let menu = Menu::new();
        let show = MenuItem::new("Open Serein Studio", true, None);
        let quit = MenuItem::new("Exit Serein", true, None);
        let pause = MenuItem::new("Pause / resume wallpaper", true, None);
        let stop = MenuItem::new("Stop wallpaper", true, None);
        menu.append(&pause).map_err(|e| e.to_string())?;
        menu.append(&stop).map_err(|e| e.to_string())?;
        menu.append(&show).map_err(|e| e.to_string())?;
        menu.append(&quit).map_err(|e| e.to_string())?;
        let mut pixels = vec![0_u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let radius = (x as f32 - 15.5).hypot(y as f32 - 15.5);
                if radius < 14. {
                    let i = (y * 32 + x) * 4;
                    let ink = (y as f32 - 16. - (x as f32 * 0.23).sin() * 4.).abs() < 2.;
                    pixels[i..i + 4].copy_from_slice(if ink {
                        &[19, 21, 25, 255]
                    } else {
                        &[255, 163, 139, 255]
                    });
                }
            }
        }
        let icon = Icon::from_rgba(pixels, 32, 32).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Serein Studio Â· development preview")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _icon: tray,
            show: show.id().clone(),
            quit: quit.id().clone(),
            pause: pause.id().clone(),
            stop: stop.id().clone(),
        })
    }
    #[cfg(not(windows))]
    pub fn new() -> Result<Self, String> {
        Err("The tray prototype is Windows-only.".into())
    }
    #[cfg(windows)]
    pub fn poll(&self) -> Option<TrayAction> {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
        while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if event.id == self.pause {
                return Some(TrayAction::Pause);
            }
            if event.id == self.stop {
                return Some(TrayAction::Stop);
            }
            if event.id == self.quit {
                return Some(TrayAction::Quit);
            }
            if event.id == self.show {
                return Some(TrayAction::Show);
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                return Some(TrayAction::Show);
            }
        }
        None
    }
    #[cfg(not(windows))]
    pub fn poll(&self) -> Option<TrayAction> {
        None
    }
}
