//! Liquid-glass design tokens for AgyLite (see docs/UI_DESIGN.md).
//! The Web/CSS recipes from the design map are translated into static
//! egui painting: vertical gradient background, translucent glass cards,
//! 1px light strokes, one copper accent. No shaders, no screen capture,
//! so idle cost stays at zero.

use eframe::egui;
use egui::{Color32, Mesh, Pos2, Rect, Shape};

pub const BG_TOP: Color32 = Color32::from_rgb(0x22, 0x2C, 0x38);
pub const BG_MID: Color32 = Color32::from_rgb(0x30, 0x3D, 0x4F);
pub const BG_BOT: Color32 = Color32::from_rgb(0x3E, 0x4D, 0x62);
pub const BG_DEEP: Color32 = Color32::from_rgb(0x22, 0x2C, 0x38);
pub const INK: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x19);
/// Body text on dark surfaces. Used explicitly (not via the theme) so a
/// text colour can never again depend on which egui style is active.
pub const TEXT_MAIN: Color32 = Color32::from_rgb(0xF2, 0xF4, 0xF7);
/// Secondary text on dark surfaces (details, hints).
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xCB, 0xD5, 0xE1);
pub const COPPER: Color32 = Color32::from_rgb(0xB6, 0x6D, 0x44);
#[allow(dead_code)]
pub const COPPER_LIGHT: Color32 = Color32::from_rgb(0xE7, 0xA4, 0x71);
pub const OK: Color32 = Color32::from_rgb(0x7B, 0xC4, 0x7F);
pub const WARN: Color32 = Color32::from_rgb(0xE7, 0xA4, 0x71);
pub const ERR: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x5F);

pub fn apply(ctx: &egui::Context) {
    // BUG (alpha.1, user report 2026-11-24): the window rendered with the
    // stock LIGHT visuals — dark text on the dark gradient (headings such as
    // "启动前环境自检" were invisible) and light buttons. Root cause: egui
    // 0.31 keeps separate dark/light styles and follows the OS theme by
    // default, so `set_visuals()` only customised the dark style; on Xi's
    // light-mode Windows the active style was the untouched light one.
    // Fix: pin the theme to Dark AND write the same dark visuals into both
    // styles, so no OS theme change can flip the app back to light.
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.override_text_color = Some(Color32::from_white_alpha(235));
    // Text edits were white boxes with dark text under the light style;
    // keep them dark with light text like the rest of the window.
    visuals.extreme_bg_color = Color32::from_rgb(0x1B, 0x25, 0x30);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);
}

/// Load a CJK system font at runtime (Microsoft YaHei on Windows,
/// Noto Sans CJK elsewhere) so Chinese renders without embedding a
/// multi-MB font into the executable.
pub fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "cjk".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// Paint the three-stop vertical gradient over the whole window.
pub fn paint_background(ctx: &egui::Context) {
    let rect = ctx.input(|i| i.screen_rect());
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rows = 24;
    let mut mesh = Mesh::default();
    for i in 0..=rows {
        let t = i as f32 / rows as f32;
        let y = rect.top() + t * rect.height();
        let color = gradient_at(t);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(rect.left(), y),
            uv: Pos2::ZERO,
            color,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(rect.right(), y),
            uv: Pos2::ZERO,
            color,
        });
        if i < rows {
            let a = (i * 2) as u32;
            mesh.indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    painter.add(Shape::mesh(mesh));
    // A soft light arc in the top-right, like light through frosted glass.
    let center = Pos2::new(rect.right() - 40.0, rect.top() - 60.0);
    painter.circle_filled(center, 170.0, Color32::from_white_alpha(14));
    painter.circle_filled(center, 110.0, Color32::from_white_alpha(12));
}

fn gradient_at(t: f32) -> Color32 {
    if t < 0.55 {
        lerp_color(BG_TOP, BG_MID, t / 0.55)
    } else {
        lerp_color(BG_MID, BG_BOT, (t - 0.55) / 0.45)
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// A translucent glass card frame.
///
/// BUG (alpha.1): the old fill was `white_alpha(26)` over a gradient that
/// faded to light grey-blue (#7A8CA3) at the bottom, so cards came out pale
/// grey and any light text on them washed out. The fill is now a mostly
/// opaque dark slate, so white/light-grey text always has contrast — the
/// glass feel comes from the stroke and the gradient behind the card, not
/// from a near-transparent fill.
pub fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(0x2E, 0x3C, 0x4E, 235))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_white_alpha(70)))
        .corner_radius(18.0)
        .inner_margin(egui::Margin::same(14))
}

#[allow(dead_code)]
pub fn glass_rect() -> Rect {
    Rect::ZERO
}
