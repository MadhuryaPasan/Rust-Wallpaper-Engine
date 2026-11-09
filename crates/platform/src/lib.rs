use serein_domain::DisplayInfo;

#[cfg(not(windows))]
pub fn displays() -> Vec<DisplayInfo> {
    Vec::new()
}

#[cfg(windows)]
pub fn attach_wallpaper(raw: isize, display: &DisplayInfo) -> Result<(), String> {
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM, POINT, WPARAM},
            Graphics::Gdi::MapWindowPoints,
            UI::WindowsAndMessaging::*,
        },
        core::{BOOL, w},
    };
    // Explorer's hosting window is a shell convention, isolated here so OS changes
    // cannot leak into the renderer. No existing desktop window is hidden or replaced.
    let progman = unsafe { FindWindowW(w!("Progman"), None) }
        .map_err(|_| "Windows Explorer desktop was not found".to_string())?;
    let mut result = 0;
    // SAFETY: A bounded message to Explorer asks it to expose its wallpaper layer.
    unsafe {
        SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(1),
            SMTO_ABORTIFHUNG,
            1000,
            Some(&mut result),
        );
    }
    // On the raised Windows 11 desktop WorkerW paints the static background.
    // Parenting into it leaves a GPU child behind that composition surface.
    // Our surface must instead be its layered sibling, directly below the icons.
    let raised = unsafe { GetWindowLongPtrW(progman, GWL_EXSTYLE) }
        & WS_EX_NOREDIRECTIONBITMAP.0 as isize
        != 0;
    let icons = unsafe { FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None) }.ok();
    let mut parent =
        unsafe { FindWindowExW(Some(progman), None, w!("WorkerW"), None) }.unwrap_or_default();
    if raised {
        if icons.is_none() || parent.0.is_null() {
            return Err("Explorer's raised desktop layers are unavailable. Try applying again after Explorer finishes starting.".into());
        }
        parent = progman;
    }
    if parent.0.is_null() {
        unsafe extern "system" fn find(top: HWND, context: LPARAM) -> BOOL {
            // SAFETY: EnumWindows invokes this synchronously; context is a live HWND.
            if unsafe { FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), None) }.is_ok()
                && let Ok(worker) = unsafe { FindWindowExW(None, Some(top), w!("WorkerW"), None) }
            {
                unsafe {
                    *(context.0 as *mut HWND) = worker;
                }
                return BOOL(0);
            }
            BOOL(1)
        }
        unsafe {
            let _ = EnumWindows(Some(find), LPARAM(&mut parent as *mut HWND as isize));
        }
    }
    if parent.0.is_null() {
        return Err("Explorer did not expose a wallpaper layer. Restart Windows Explorer, then apply the wallpaper again.".into());
    }
    let hwnd = HWND(raw as *mut _);
    // SAFETY: hwnd belongs to our winit window. Only our window's styles/parent change.
    unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN).0 as isize,
        );
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | if raised {
                    WS_EX_LAYERED
                } else {
                    WINDOW_EX_STYLE(0)
                })
            .0 as isize,
        );
        let _ = SetParent(hwnd, Some(parent));
        if GetParent(hwnd).ok() != Some(parent) {
            return Err("Unable to attach the wallpaper beneath desktop icons. Try applying it again after Explorer restarts.".into());
        }
        if raised {
            SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                LWA_ALPHA,
            )
            .map_err(|e| format!("Unable to enable desktop composition: {e}"))?;
        }
        let mut origin = POINT {
            x: display.x,
            y: display.y,
        };
        MapWindowPoints(None, Some(parent), std::slice::from_mut(&mut origin));
        SetWindowPos(
            hwnd,
            Some(if raised {
                icons.unwrap_or(HWND_TOP)
            } else {
                HWND_TOP
            }),
            origin.x,
            origin.y,
            display.width as i32,
            display.height as i32,
            SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn attach_wallpaper(_: isize, _: &DisplayInfo) -> Result<(), String> {
    Err("Desktop playback currently requires Windows".into())
}

/// Verify the shell still owns the surface, including its stacking order.
/// Frame submission alone does not establish that a wallpaper is attached.
#[cfg(windows)]
pub fn wallpaper_attached(raw: isize, display: &DisplayInfo) -> bool {
    use windows::{
        Win32::{
            Foundation::{HWND, RECT},
            UI::WindowsAndMessaging::*,
        },
        core::w,
    };
    let hwnd = HWND(raw as *mut _);
    unsafe {
        let Ok(parent) = GetParent(hwnd) else {
            return false;
        };
        if !IsWindowVisible(hwnd).as_bool() || !IsWindowVisible(parent).as_bool() {
            return false;
        }
        let Ok(progman) = FindWindowW(w!("Progman"), None) else {
            return false;
        };
        if GetWindowLongPtrW(progman, GWL_EXSTYLE) & WS_EX_NOREDIRECTIONBITMAP.0 as isize != 0 {
            if parent != progman
                || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED.0 as isize == 0
            {
                return false;
            }
            let Ok(icons) = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None) else {
                return false;
            };
            let Ok(worker) = FindWindowExW(Some(progman), None, w!("WorkerW"), None) else {
                return false;
            };
            // Walk siblings in front-to-back order: icons, our surface, background.
            let mut sibling = GetWindow(icons, GW_HWNDNEXT).unwrap_or_default();
            let mut found = false;
            while !sibling.0.is_null() {
                if sibling == hwnd {
                    found = true;
                }
                if sibling == worker {
                    if !found {
                        return false;
                    }
                    break;
                }
                sibling = GetWindow(sibling, GW_HWNDNEXT).unwrap_or_default();
            }
            if !found {
                return false;
            }
        } else {
            let mut class = [0u16; 32];
            let length = GetClassNameW(parent, &mut class);
            if String::from_utf16_lossy(&class[..length.max(0) as usize]) != "WorkerW" {
                return false;
            }
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).is_ok()
            && rect.left == display.x
            && rect.top == display.y
            && rect.right - rect.left == display.width as i32
            && rect.bottom - rect.top == display.height as i32
    }
}
#[cfg(not(windows))]
pub fn wallpaper_attached(_: isize, _: &DisplayInfo) -> bool {
    false
}

#[cfg(windows)]
pub fn pointer_position() -> Option<(i32, i32)> {
    let mut point = windows::Win32::Foundation::POINT::default();
    // SAFETY: point is writable and remains alive for this synchronous query.
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) }.ok()?;
    Some((point.x, point.y))
}
#[cfg(not(windows))]
pub fn pointer_position() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
pub fn is_minimized(raw: isize) -> bool {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::IsIconic(windows::Win32::Foundation::HWND(
            raw as *mut _,
        ))
        .as_bool()
    }
}
#[cfg(not(windows))]
pub fn is_minimized(_: isize) -> bool {
    false
}

#[cfg(windows)]
pub fn should_pause(displays: &[DisplayInfo], fullscreen: bool, battery: bool) -> bool {
    use windows::Win32::{
        Foundation::RECT,
        System::{
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
            StationsAndDesktops::{CloseDesktop, DESKTOP_READOBJECTS, OpenInputDesktop},
        },
        UI::WindowsAndMessaging::*,
    };
    // An inaccessible input desktop normally means the session is locked.
    match unsafe {
        OpenInputDesktop(
            windows::Win32::System::StationsAndDesktops::DESKTOP_CONTROL_FLAGS(0),
            false,
            DESKTOP_READOBJECTS,
        )
    } {
        Ok(desktop) => unsafe {
            let _ = CloseDesktop(desktop);
        },
        Err(_) => return true,
    }
    if battery {
        let mut power = SYSTEM_POWER_STATUS::default();
        if unsafe { GetSystemPowerStatus(&mut power) }.is_ok() && power.ACLineStatus == 0 {
            return true;
        }
    }
    if !fullscreen {
        return false;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return false;
    }
    let mut class = [0_u16; 128];
    let length = unsafe { GetClassNameW(foreground, &mut class) };
    let class = String::from_utf16_lossy(&class[..length.max(0) as usize]);
    if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
        return false;
    }
    if unsafe { IsZoomed(foreground).as_bool() } {
        return true;
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(foreground, &mut rect) }.is_err() {
        return false;
    }
    displays.iter().any(|d| {
        rect.left <= d.x
            && rect.top <= d.y
            && rect.right >= d.x + d.width as i32
            && rect.bottom >= d.y + d.height as i32
    })
}
#[cfg(not(windows))]
pub fn should_pause(_: &[DisplayInfo], _: bool, _: bool) -> bool {
    false
}

#[cfg(windows)]
pub fn displays() -> Vec<DisplayInfo> {
    use windows::Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
        },
    };
    use windows::core::BOOL;
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        user: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: Win32 supplies the monitor handle; info has the required size.
        if unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
            let r = info.monitorInfo.rcMonitor;
            let len = info
                .szDevice
                .iter()
                .position(|v| *v == 0)
                .unwrap_or(info.szDevice.len());
            // SAFETY: EnumDisplayMonitors is synchronous. user points to our live Vec.
            let output = unsafe { &mut *(user.0 as *mut Vec<DisplayInfo>) };
            output.push(DisplayInfo {
                name: String::from_utf16_lossy(&info.szDevice[..len]),
                x: r.left,
                y: r.top,
                width: (r.right - r.left).max(0) as u32,
                height: (r.bottom - r.top).max(0) as u32,
                primary: info.monitorInfo.dwFlags & 1 != 0,
            });
        }
        BOOL(1)
    }
    let mut output = Vec::new();
    // SAFETY: The callback is synchronous and its context remains alive for the call.
    let result = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut output as *mut Vec<DisplayInfo> as isize),
        )
    };
    if !result.as_bool() {
        output.clear();
    }
    output
}
