//! Read-only shell diagnostics; prints classes and geometry, never window titles.
#[cfg(windows)]
fn main() {
    use windows::{
        Win32::{
            Foundation::{HWND, RECT},
            UI::WindowsAndMessaging::*,
        },
        core::w,
    };
    unsafe fn describe(window: HWND, depth: usize) {
        let mut class = [0u16; 128];
        let count = unsafe { GetClassNameW(window, &mut class) };
        let mut rect = RECT::default();
        let _ = unsafe { GetWindowRect(window, &mut rect) };
        println!(
            "{}{:x} {} visible={} ex={:x} rect={:?}",
            " ".repeat(depth),
            window.0 as usize,
            String::from_utf16_lossy(&class[..count.max(0) as usize]),
            unsafe { IsWindowVisible(window).as_bool() },
            unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) },
            rect
        );
        let mut child = unsafe { GetWindow(window, GW_CHILD) }.unwrap_or_default();
        while !child.0.is_null() {
            unsafe { describe(child, depth + 1) };
            child = unsafe { GetWindow(child, GW_HWNDNEXT) }.unwrap_or_default();
        }
    }
    unsafe {
        if let Ok(progman) = FindWindowW(w!("Progman"), None) {
            describe(progman, 0);
        }
    }
}
#[cfg(not(windows))]
fn main() {}
