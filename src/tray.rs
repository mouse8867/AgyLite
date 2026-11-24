//! System-tray icon (Windows only).
//!
//! Created once at startup and kept alive for the life of the app, so the
//! window always has somewhere to go: minimising (title-bar — button or
//! "收起窗口" after launch) hides the window and leaves only this icon;
//! left-click / double-click restores the window, right-click shows a menu
//! with 显示窗口 / 退出 AgyLite. Previously (alpha.1) there was no tray icon
//! at all — minimise only went to the taskbar, which is the bug Xi reported
//! on 2026-11-24.
//!
//! BUG (alpha.3, user report 2026-11-24): with the window in the tray,
//! "显示窗口" did nothing and "退出" did not quit — the process could only
//! be ended from the task manager. Two separate faults, both fixed in
//! alpha.4 (see \"scripts/verify_tray.ps1\" for the regression test):
//!
//! 1. *The events never reached the window.* eframe applies
//!    `egui::ViewportCommand`s inside its frame loop, and a frame only
//!    happens when Windows paints the window. A window hidden with
//!    `set_visible(false)` never receives `WM_PAINT`, so it produces no
//!    frames — every command queued for it (show, focus, close) sat in
//!    the queue forever. The tray events were arriving all along; there
//!    was simply no path from the event to the window. The listener now
//!    drives the window through Win32 directly (see `win`), the way a
//!    native tray app does.
//! 2. *Two consumers raced for the same events.* Both this listener
//!    thread and `AgyApp::update` drained the global tray receivers, so
//!    whoever polled first swallowed the event. This thread is now the
//!    only reader.

#[cfg(windows)]
use crate::win;

#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub const MENU_SHOW_ID: &str = "show";
pub const MENU_QUIT_ID: &str = "quit";

/// The app icon from assets/icon.png (self-drawn: gradient slate rounded
/// square, white orbit ring, copper planet) — the same artwork embedded
/// into the exe and used as the window icon. Falls back to the code-drawn
/// mark below if decoding ever fails.
fn make_icon() -> Result<Icon, tray_icon::BadIcon> {
    match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            Icon::from_rgba(rgba.into_raw(), w, h)
        }
        Err(_) => make_icon_drawn(),
    }
}

fn make_icon_drawn() -> Result<Icon, tray_icon::BadIcon> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            let r = (dx * dx + dy * dy).sqrt();
            let px: [u8; 4] = if r <= 5.0 {
                [0xB6, 0x6D, 0x44, 255] // copper centre
            } else if (12.5..14.5).contains(&r) {
                [255, 255, 255, 210] // orbit ring
            } else if r <= 15.0 {
                [0x1B, 0x1B, 0x19, 255] // dark disc
            } else {
                [0, 0, 0, 0] // transparent corners
            };
            rgba.extend_from_slice(&px);
        }
    }
    Icon::from_rgba(rgba, size, size)
}

#[cfg(windows)]
pub fn build_tray() -> Result<TrayIcon, String> {
    let menu = Menu::new();
    menu.append(&MenuItem::with_id(
        MenuId::new(MENU_SHOW_ID),
        "显示窗口",
        true,
        None,
    ))
    .map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&MenuItem::with_id(
        MenuId::new(MENU_QUIT_ID),
        "退出 AgyLite",
        true,
        None,
    ))
    .map_err(|e| e.to_string())?;

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("AgyLite · Antigravity 免 TUN 启动器")
        .with_icon(make_icon().map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())
}

/// Dedicated tray-event listener thread.
///
/// BUG (alpha.2, user report 2026-11-24): tray clicks did nothing while
/// the window was hidden. The events were only polled from
/// `eframe::App::update`, but a hidden viewport stops producing frames,
/// so the poll (and its 250 ms repaint chain) never ran again — restore
/// and quit were both dead. This thread polls the receivers independently
/// of the frame loop, so restore/quit work no matter how deeply the UI
/// sleeps.
///
/// This thread is also the *only* reader of the tray receivers (alpha.3):
/// `AgyApp::update` used to poll them as well, and the two consumers
/// raced for the same events.
pub static WINDOW_HIDDEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Timestamp (ms since epoch) of the last "显示窗口" restore; 0 = never.
///
/// The app turns an OS "window was minimised" signal into "hide into the
/// tray". Right after a restore that signal can still be stale, and
/// honouring it would hide the window again on the very next frame — so
/// it is ignored for a short grace period.
static RESTORED_AT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How long after a restore a stale "minimised" signal is ignored (ms).
const RESTORE_GRACE_MS: u64 = 800;

/// True while the stale-minimised grace period after a restore is running.
pub fn restore_grace_active() -> bool {
    let last = RESTORED_AT_MS.load(std::sync::atomic::Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    now_millis().saturating_sub(last) < RESTORE_GRACE_MS
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn spawn_listener(ctx: eframe::egui::Context) {
    std::thread::spawn(move || loop {
        let (restore, quit) = poll_tray();
        if quit {
            quit_app();
            break;
        }
        if restore {
            restore_window(&ctx);
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    });
}

/// Bring the window back from the tray.
///
/// The window itself is shown through Win32 (see `win`): eframe can only
/// apply viewport commands while it paints frames, and a hidden window is
/// never painted — so `Visible(true)` would sit in the queue forever.
fn restore_window(ctx: &eframe::egui::Context) {
    WINDOW_HIDDEN.store(false, std::sync::atomic::Ordering::Relaxed);
    RESTORED_AT_MS.store(now_millis(), std::sync::atomic::Ordering::Relaxed);

    if let Some(hwnd) = win::main_window() {
        win::show_and_focus(hwnd);
    }

    // Tell egui/eframe as well, so its own viewport state matches the
    // window on the next frame — frames resume now that it is visible.
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

/// Exit the app from the tray menu.
///
/// The window is asked to close the way the title-bar X button does
/// (WM_CLOSE), so winit sees a normal close request and eframe shuts
/// down cleanly. If the event loop does not wind down within a couple of
/// seconds we exit hard — a process that can only be killed from the
/// task manager is worse than a slightly rude exit.
fn quit_app() {
    let Some(hwnd) = win::main_window() else {
        std::process::exit(0);
    };
    win::request_close(hwnd);
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !win::is_alive(hwnd) {
            return;
        }
    }
    std::process::exit(0);
}

/// Drain pending tray-icon and menu events. Returns true when the window
/// should be restored, false otherwise; sets `quit` when 退出 was chosen.
#[cfg(windows)]
pub fn poll_tray() -> (bool, bool) {
    let mut restore = false;
    let mut quit = false;
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        match event {
            TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: tray_icon::MouseButton::Left,
                ..
            } => restore = true,
            _ => {}
        }
    }
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id.as_ref() == MENU_SHOW_ID {
            restore = true;
        } else if event.id.as_ref() == MENU_QUIT_ID {
            quit = true;
        }
    }
    (restore, quit)
}
