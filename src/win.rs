//! Direct Win32 control of the app's own window (Windows only).
//!
//! Why this module exists — the alpha.3 dead end (fixed in alpha.4):
//!
//! eframe applies `egui::ViewportCommand`s (show / focus / close) inside
//! its frame loop, and a frame only happens when Windows paints the
//! window. A window hidden with `set_visible(false)` never receives
//! `WM_PAINT`, so it produces no frames — and every command queued for
//! it stays unapplied forever: "显示窗口" queued `Visible(true)` that
//! nobody executed, "退出" queued `Close` that nobody executed. The tray
//! events were arriving all along; there was simply no path from the
//! event to the window.
//!
//! So the tray thread talks to the window directly (ShowWindow /
//! SetForegroundWindow / PostMessage(WM_CLOSE)), the way a native tray
//! app does. WM_CLOSE in particular lets winit deliver a normal
//! `WindowEvent::CloseRequested`, which eframe turns into a clean exit
//! (`on_exit` + state save) instead of a hard kill.

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, PostMessageW, SetForegroundWindow, ShowWindow,
    SW_RESTORE, SW_SHOW, WM_CLOSE,
};

/// Title prefix of the main window (see `main.rs`).
const TITLE_PREFIX: &str = "AgyLite";

/// Class name of the tray icon's own helper window (tray-icon crate).
/// It is a top-level window too, so it has to be skipped explicitly.
const TRAY_CLASS: &str = "tray_icon_app";

/// Find the app's own main window.
///
/// The process owns two top-level windows: the eframe one and the tray
/// icon's helper window (class "tray_icon_app"), which is skipped
/// explicitly. The title tells the main window apart from transient
/// windows (a file dialog, for instance); if no title matches, the first
/// remaining window of this process is used.
pub fn main_window() -> Option<HWND> {
    let mut search = Search {
        pid: unsafe { GetCurrentProcessId() },
        titled: None,
        fallback: None,
    };
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut search as *mut Search as LPARAM,
        );
    }
    search.titled.or(search.fallback)
}

struct Search {
    pid: u32,
    titled: Option<HWND>,
    fallback: Option<HWND>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = &mut *(lparam as *mut Search);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid != search.pid {
        return 1; // not ours — keep enumerating
    }
    if class_name(hwnd) == TRAY_CLASS {
        return 1; // the tray icon's helper window — not the app window
    }
    if title_starts_with(hwnd, TITLE_PREFIX) {
        search.titled = Some(hwnd);
        return 0; // found it — stop
    }
    if search.fallback.is_none() {
        search.fallback = Some(hwnd);
    }
    1
}

/// Window class name of a window (same process, so GetClassNameW works).
fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

/// Window caption of our own window (same process, so GetWindowTextW works).
fn title_starts_with(hwnd: HWND, prefix: &str) -> bool {
    let mut buf = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return false;
    }
    String::from_utf16_lossy(&buf[..len as usize]).starts_with(prefix)
}

/// Bring the window back on screen and give it the keyboard focus.
///
/// `ShowWindow` alone is not enough: a window that was hidden (not
/// minimised) comes back without activation, so the user would have to
/// click it again.
pub fn show_and_focus(hwnd: HWND) {
    unsafe {
        let cmd = if IsIconic(hwnd) != 0 { SW_RESTORE } else { SW_SHOW };
        ShowWindow(hwnd, cmd);

        // SetForegroundWindow is refused unless the calling thread may
        // take the foreground; borrowing the foreground thread's input
        // queue (AttachThreadInput) is the standard way around that.
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let cur_thread = GetCurrentThreadId();
        let attached = fg_thread != 0
            && fg_thread != cur_thread
            && AttachThreadInput(cur_thread, fg_thread, 1) != 0;
        SetForegroundWindow(hwnd);
        BringWindowToTop(hwnd);
        if attached {
            AttachThreadInput(cur_thread, fg_thread, 0);
        }
    }
}

/// Ask the window to close exactly like the title-bar X button does.
///
/// WM_CLOSE makes winit deliver `WindowEvent::CloseRequested`, which
/// eframe turns into a clean shutdown (on_exit, state save, exit).
pub fn request_close(hwnd: HWND) {
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

/// Is the window still alive? False once it has been destroyed.
pub fn is_alive(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) != 0 }
}
