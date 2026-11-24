//! System-tray icon (Windows only).
//!
//! Created once at startup and kept alive for the life of the app, so the
//! window always has somewhere to go: minimising (title-bar — button or
//! "收起窗口" after launch) hides the window and leaves only this icon;
//! left-click / double-click restores the window, right-click shows a menu
//! with 显示窗口 / 退出 AgyLite. Previously (alpha.1) there was no tray icon
//! at all — minimise only went to the taskbar, which is the bug Xi reported
//! on 2026-11-24.

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
/// of the frame loop and issues the viewport commands directly
/// (`egui::Context` is thread-safe and sending a command also wakes the
/// event loop), so restore/quit work no matter how deeply the UI sleeps.
/// Shared "window is hidden in the tray" flag. The listener thread cannot
/// touch the app struct, so both sides sync through this atomic.
pub static WINDOW_HIDDEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn spawn_listener(ctx: eframe::egui::Context) {
    std::thread::spawn(move || loop {
        let (restore, quit) = poll_tray();
        if quit {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            ctx.request_repaint();
            break;
        }
        if restore {
            WINDOW_HIDDEN.store(false, std::sync::atomic::Ordering::Relaxed);
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    });
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
