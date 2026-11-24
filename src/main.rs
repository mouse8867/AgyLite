#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod detect;
mod launch;
mod preflight;
mod theme;
#[cfg(windows)]
mod tray;

use config::{AfterLaunch, Config};
use eframe::egui;
use egui::{Align2, Color32, FontId, Rect, Sense, Vec2};
use preflight::{PreflightResult, Status};
use std::sync::mpsc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Status,
    Checks,
    Settings,
}

struct AgyApp {
    cfg: Config,
    tab: Tab,
    result: Option<PreflightResult>,
    checking: bool,
    rx: Option<mpsc::Receiver<PreflightResult>>,
    banner: Option<String>,
    error: Option<String>,
    expanded: Option<usize>,
    port_text: String,
    /// Support QR codes, decoded once at startup (see `load_qr_texture`).
    alipay_tex: Option<egui::TextureHandle>,
    wechat_tex: Option<egui::TextureHandle>,
    qr_error: Option<String>,
    /// Tray icon, created at startup on Windows and kept alive for the
    /// life of the app. `None` on other platforms, or if creation failed.
    #[cfg(windows)]
    tray: Option<tray_icon::TrayIcon>,
    /// True while the window is hidden in the tray.
    hidden_to_tray: bool,
}

/// Decode a JPEG at startup and upload it as a texture.
///
/// BUG (alpha.1, user report 2026-11-24): the About page showed only ⚠
/// placeholders where the two support QR codes should be. The codes were
/// rendered with `egui::Image::from_bytes`, whose async image loader did
/// not resolve on Windows in this build. Decoding with the `image` crate
/// and uploading a texture directly removes that loader from the path;
/// the QR bitmap is identical, only the delivery changes.
fn load_qr_texture(
    ctx: &egui::Context,
    name: &str,
    bytes: &[u8],
) -> Result<egui::TextureHandle, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    // Downscale to ~260px wide (displayed at 130px, so still 2× for HiDPI).
    let img = img.thumbnail(260, 360);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
    Ok(ctx.load_texture(name, color, egui::TextureOptions::default()))
}

impl AgyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        let mut cfg = Config::load();
        if cfg.agy_path.is_none() {
            cfg.agy_path = detect::find_antigravity().map(|p| p.display().to_string());
        }
        let port_text = cfg.proxy_port.map(|p| p.to_string()).unwrap_or_default();
        // QR codes: decode now; if either fails, the About page shows the
        // reason in text instead of a broken-image placeholder.
        let mut qr_error = None;
        let alipay_tex = match load_qr_texture(
            &cc.egui_ctx,
            "alipay-qr",
            include_bytes!("../docs/assets/support/alipay-public.jpg"),
        ) {
            Ok(t) => Some(t),
            Err(e) => {
                qr_error = Some(format!("支付宝收款码加载失败：{e}"));
                None
            }
        };
        let wechat_tex = match load_qr_texture(
            &cc.egui_ctx,
            "wechat-qr",
            include_bytes!("../docs/assets/support/wechat-pay-public.jpg"),
        ) {
            Ok(t) => Some(t),
            Err(e) => {
                qr_error = Some(format!("微信收款码加载失败：{e}"));
                None
            }
        };
        #[cfg(windows)]
        let tray = match tray::build_tray() {
            Ok(t) => Some(t),
            Err(e) => {
                let msg = format!("托盘图标创建失败，已回退到任务栏最小化：{e}");
                qr_error = Some(match qr_error {
                    Some(prev) => format!("{prev}；{msg}"),
                    None => msg,
                });
                None
            }
        };
        let mut app = Self {
            cfg,
            tab: Tab::Status,
            result: None,
            checking: false,
            rx: None,
            banner: None,
            error: None,
            expanded: None,
            port_text,
            alipay_tex,
            wechat_tex,
            qr_error,
            #[cfg(windows)]
            tray,
            hidden_to_tray: false,
        };
        // Tray events are handled by a dedicated listener thread: a hidden
        // window produces no frames, so polling from update() alone left
        // tray restore/quit dead in alpha.2.
        #[cfg(windows)]
        if app.tray.is_some() {
            tray::spawn_listener(cc.egui_ctx.clone());
        }
        app.start_check(&cc.egui_ctx);
        app
    }

    fn start_check(&mut self, ctx: &egui::Context) {
        if self.checking {
            return;
        }
        self.checking = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let cfg = self.cfg.clone();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let res = preflight::run(&cfg);
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }

    fn poll_check(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(res) = rx.try_recv() {
                // Adopt the best detected port if the user has not set one.
                if self.cfg.proxy_port.is_none() {
                    if let Some(best) = detect::best_port(&res.ports) {
                        self.cfg.proxy_port = Some(best.port);
                        self.port_text = best.port.to_string();
                        self.cfg.save();
                    }
                }
                self.result = Some(res);
                self.checking = false;
                self.rx = None;
            }
        }
    }

    /// Hide the window into the tray (Windows). Returns false when there
    /// is no tray icon to hide into — the caller then falls back to the
    /// old minimise-to-taskbar behaviour instead of losing the window.
    #[cfg(windows)]
    fn hide_to_tray(&mut self, ctx: &egui::Context) -> bool {
        if self.tray.is_none() {
            return false;
        }
        self.hidden_to_tray = true;
        tray::WINDOW_HIDDEN.store(true, std::sync::atomic::Ordering::Relaxed);
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        true
    }

    #[cfg(windows)]
    fn restore_from_tray(&mut self, ctx: &egui::Context) {
        self.hidden_to_tray = false;
        tray::WINDOW_HIDDEN.store(false, std::sync::atomic::Ordering::Relaxed);
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    #[cfg(windows)]
    fn poll_tray(&mut self, ctx: &egui::Context) {
        let (restore, quit) = tray::poll_tray();
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if restore {
            self.restore_from_tray(ctx);
        }
    }

    fn do_launch(&mut self, ctx: &egui::Context) {
        self.error = None;
        if let Some(res) = &self.result {
            if res.has_fail() {
                self.error =
                    Some("环境检测有未通过项，请先到「检测」页按提示处理".to_string());
                self.tab = Tab::Checks;
                return;
            }
        }
        match launch::launch(&self.cfg) {
            Ok(msg) => {
                self.banner = Some(format!(
                    "{msg}。AgyLite 现在可以关闭；下次使用 Antigravity 请仍从 AgyLite 启动。"
                ));
                match self.cfg.after_launch {
                    AfterLaunch::Minimise => {
                        // "收进托盘": hide into the tray; only if there is no
                        // tray icon do we fall back to taskbar minimise.
                        #[cfg(windows)]
                        if !self.hide_to_tray(ctx) {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        #[cfg(not(windows))]
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    AfterLaunch::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    AfterLaunch::Keep => {}
                }
            }
            Err(e) => self.error = Some(e),
        }
    }
}

impl eframe::App for AgyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_check();
        if self.checking {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        #[cfg(windows)]
        {
            self.poll_tray(ctx);
            // The title-bar — button minimises to the taskbar in winit and
            // there is no minimise hook, so convert it here: once the OS
            // reports the window minimised, hide it into the tray instead.
            // The tray listener thread restores the window without going
            // through this struct; re-sync the flag from the shared atomic.
            self.hidden_to_tray =
                tray::WINDOW_HIDDEN.load(std::sync::atomic::Ordering::Relaxed);
            let os_minimised = ctx.input(|i| i.viewport().minimized) == Some(true);
            if os_minimised && !self.hidden_to_tray && !self.hide_to_tray(ctx) {
                // No tray icon (creation failed): leave the taskbar
                // minimise as it is rather than hiding the window.
            }
            if self.hidden_to_tray {
                // A hidden viewport gets no repaint requests on its own;
                // poll the tray a few times a second so a tray click
                // restores the window promptly.
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        theme::paint_background(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                self.header(ui);
                ui.add_space(10.0);
                self.segmented(ui, ctx);
                ui.add_space(12.0);
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    Tab::Status => self.status_page(ui, ctx),
                    Tab::Checks => self.checks_page(ui),
                    Tab::Settings => self.settings_page(ui),
                });
            });
    }
}

impl AgyApp {
    fn header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Brand mark: dark disc, orbit ring, copper centre (drawn, no assets).
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            let p = ui.painter();
            p.circle_filled(rect.center(), 16.0, theme::INK);
            p.circle_stroke(
                rect.center(),
                13.5,
                egui::Stroke::new(1.6_f32, Color32::from_white_alpha(150)),
            );
            p.circle_filled(rect.center(), 5.0, theme::COPPER);
            ui.label(
                egui::RichText::new("AgyLite")
                    .size(20.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(11.0)
                    .color(Color32::from_white_alpha(160)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.checking {
                    ui.spinner();
                    ui.label("检测中…");
                }
            });
        });
    }

    fn segmented(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let labels = ["状态", "检测", "设置"];
        let tabs = [Tab::Status, Tab::Checks, Tab::Settings];
        let current = tabs.iter().position(|t| *t == self.tab).unwrap_or(0) as f32;
        let width = ui.available_width();
        let height = 36.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 18.0, Color32::from_white_alpha(28));
        p.rect_stroke(
            rect,
            18.0,
            egui::Stroke::new(1.0_f32, Color32::from_white_alpha(60)),
            egui::StrokeKind::Inside,
        );
        let seg_w = rect.width() / 3.0;
        // Liquid pill: slide the highlight towards the selected segment.
        let t = ctx.animate_value_with_time(egui::Id::new("tab-pill"), current, 0.22);
        let pill = Rect::from_min_size(
            egui::pos2(rect.left() + t * seg_w + 3.0, rect.top() + 3.0),
            Vec2::new(seg_w - 6.0, height - 6.0),
        );
        p.rect_filled(pill, 15.0, Color32::from_white_alpha(225));
        for (i, (label, tab)) in labels.iter().zip(tabs.iter()).enumerate() {
            let seg = Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * seg_w, rect.top()),
                Vec2::new(seg_w, height),
            );
            let resp = ui.interact(seg, egui::Id::new(("tab", i)), Sense::click());
            if resp.clicked() {
                self.tab = *tab;
            }
            let selected = (t - i as f32).abs() < 0.5;
            let color = if selected {
                theme::BG_DEEP
            } else {
                Color32::from_white_alpha(210)
            };
            p.text(
                seg.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(14.0),
                color,
            );
        }
    }

    fn status_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Dark anchor card (UI design spec: one near-black card grounds the glass).
        let frame = egui::Frame::new()
            .fill(theme::INK)
            .corner_radius(18.0)
            .inner_margin(egui::Margin::same(16));
        frame.show(ui, |ui| {
            ui.label(
                egui::RichText::new("当前状态")
                    .size(13.0)
                    .color(Color32::from_white_alpha(170)),
            );
            ui.add_space(6.0);
            match &self.result {
                Some(res) => {
                    for item in &res.items {
                        self.status_row(ui, item.status, &item.title, &item.detail);
                    }
                }
                None => {
                    ui.label("正在检测环境…");
                }
            }
        });
        ui.add_space(12.0);

        // Glass summary card.
        let frame = theme::glass_frame();
        frame.show(ui, |ui| match &self.result {
            Some(res) => {
                let warns = res
                    .items
                    .iter()
                    .filter(|i| i.status != Status::Pass)
                    .count();
                if warns == 0 {
                    ui.label("✓ 环境检测全部通过，可以启动");
                } else {
                    ui.label(format!(
                        "环境检测：{} 项通过，{} 项需要留意（见「检测」页）",
                        res.pass_count(),
                        warns
                    ));
                }
            }
            None => {
                ui.label("环境检测进行中…");
            }
        });
        ui.add_space(14.0);

        let can_launch = self.result.as_ref().map(|r| !r.has_fail()).unwrap_or(false);
        let btn = egui::Button::new(
            egui::RichText::new("启动 Antigravity（免 TUN）")
                .size(16.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(if can_launch {
            theme::COPPER
        } else {
            Color32::from_rgb(120, 100, 92)
        })
        .corner_radius(14.0);
        if ui.add_sized([ui.available_width(), 46.0], btn).clicked() && can_launch {
            self.do_launch(ctx);
        }
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("启动成功后 AgyLite 会收进托盘，右键托盘图标可显示窗口或退出")
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
        });

        if let Some(banner) = &self.banner {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(banner.clone())
                    .color(Color32::from_rgb(150, 220, 150)),
            );
        }
        if let Some(err) = &self.error {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(err.clone()).color(Color32::from_rgb(240, 140, 130)));
        }
    }

    fn status_row(&self, ui: &mut egui::Ui, status: Status, title: &str, detail: &str) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
            let color = match status {
                Status::Pass => theme::OK,
                Status::Warn => theme::WARN,
                Status::Fail => theme::ERR,
            };
            ui.painter().circle_filled(rect.center(), 4.5, color);
            ui.label(
                egui::RichText::new(format!("{title}  "))
                    .size(13.5)
                    .color(theme::TEXT_MAIN),
            );
            ui.label(
                egui::RichText::new(detail)
                    .size(13.0)
                    .color(theme::TEXT_DIM),
            );
        });
    }

    fn qr_image(&self, ui: &mut egui::Ui, tex: &Option<egui::TextureHandle>, label: &str) {
        match tex {
            Some(t) => {
                let size = t.size_vec2();
                let disp = Vec2::new(130.0, 130.0 * size.y / size.x);
                ui.image((t.id(), disp));
                ui.label(
                    egui::RichText::new(label)
                        .size(12.5)
                        .color(theme::TEXT_MAIN),
                );
            }
            None => {
                ui.label(
                    egui::RichText::new(format!("{label}（图片加载失败）"))
                        .size(12.5)
                        .color(theme::TEXT_DIM),
                );
            }
        }
    }

    fn checks_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("启动前环境自检")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT_MAIN),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // A real button, not a hint: copper fill, full height, so it
                // reads as an action (alpha.1 used the stock light button,
                // which looked like a grey tooltip label).
                let btn = egui::Button::new(
                    egui::RichText::new("重新检测")
                        .size(14.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(theme::COPPER)
                .corner_radius(10.0);
                if ui.add_sized([120.0, 36.0], btn).clicked() {
                    let ctx = ui.ctx().clone();
                    self.result = None;
                    self.start_check(&ctx);
                }
            });
        });
        ui.add_space(8.0);
        let items: Vec<(Status, String, String, Option<String>)> = match &self.result {
            Some(res) => res
                .items
                .iter()
                .map(|i| (i.status, i.title.clone(), i.detail.clone(), i.hint.clone()))
                .collect(),
            None => Vec::new(),
        };
        if items.is_empty() {
            ui.label("检测中…");
            return;
        }
        for (idx, (status, title, detail, hint)) in items.iter().enumerate() {
            let frame = theme::glass_frame();
            let mut toggle = false;
            frame.show(ui, |ui| {
                let resp = ui
                    .horizontal(|ui| {
                        let icon = match status {
                            Status::Pass => "✅",
                            Status::Warn => "⚠️",
                            Status::Fail => "❌",
                        };
                        ui.label(
                            egui::RichText::new(format!("{icon} {title}"))
                                .color(theme::TEXT_MAIN),
                        );
                        ui.label(
                            egui::RichText::new(detail).color(theme::TEXT_DIM),
                        );
                        if hint.is_some() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(if self.expanded == Some(idx) { "▾" } else { "›" });
                                },
                            );
                        }
                    })
                    .response;
                if hint.is_some() && resp.interact(Sense::click()).clicked() {
                    toggle = true;
                }
                if self.expanded == Some(idx) {
                    if let Some(h) = hint {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("怎么办：{h}"))
                                .size(12.5)
                                .color(Color32::from_rgb(240, 200, 160)),
                        );
                    }
                }
            });
            if toggle {
                self.expanded = if self.expanded == Some(idx) {
                    None
                } else {
                    Some(idx)
                };
            }
            ui.add_space(8.0);
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let frame = theme::glass_frame();
        let mut changed = false;
        let mut autostart_toggled = false;
        frame.show(ui, |ui| {
            ui.label(egui::RichText::new("设置").strong().size(15.0));
            ui.add_space(8.0);
            if ui
                .checkbox(&mut self.cfg.autostart, "开机自启（托盘常驻，不自动打开 Antigravity）")
                .changed()
            {
                changed = true;
                autostart_toggled = true;
            }
            if ui
                .checkbox(&mut self.cfg.kill_residual, "启动前清理 Antigravity 残留进程（推荐）")
                .changed()
            {
                changed = true;
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("启动成功后：").color(theme::TEXT_MAIN),
            );
            ui.horizontal(|ui| {
                changed |= ui
                    .radio_value(&mut self.cfg.after_launch, AfterLaunch::Minimise, "收进托盘")
                    .changed();
                changed |= ui
                    .radio_value(&mut self.cfg.after_launch, AfterLaunch::Close, "自动关闭")
                    .changed();
                changed |= ui
                    .radio_value(&mut self.cfg.after_launch, AfterLaunch::Keep, "保持窗口")
                    .changed();
            });
        });
        if autostart_toggled {
            if let Err(e) = launch::set_autostart(self.cfg.autostart) {
                self.error = Some(format!("设置开机自启失败：{e}"));
                self.cfg.autostart = !self.cfg.autostart;
            }
        }
        ui.add_space(12.0);

        // Advanced (collapsed): manual path / port overrides.
        egui::CollapsingHeader::new(
            egui::RichText::new("高级（一般不用动）").color(theme::TEXT_MAIN),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Antigravity 路径：").color(theme::TEXT_MAIN),
                );
                let mut path = self.cfg.agy_path.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut path).changed() {
                    self.cfg.agy_path = if path.trim().is_empty() {
                        None
                    } else {
                        Some(path)
                    };
                    changed = true;
                }
                if ui.button("选择…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Antigravity", &["exe"])
                        .pick_file()
                    {
                        self.cfg.agy_path = Some(p.display().to_string());
                        changed = true;
                    }
                }
                if ui.button("重新自动查找").clicked() {
                    self.cfg.agy_path =
                        detect::find_antigravity().map(|p| p.display().to_string());
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("代理端口：").color(theme::TEXT_MAIN));
                if ui.text_edit_singleline(&mut self.port_text).changed() {
                    self.cfg.proxy_port = self.port_text.trim().parse().ok();
                    changed = true;
                }
                ui.label("（留空则自动检测）");
            });
        });
        ui.add_space(12.0);

        // About + support.
        let frame = theme::glass_frame();
        frame.show(ui, |ui| {
            ui.label(egui::RichText::new("关于 AgyLite").strong().size(15.0));
            ui.add_space(4.0);
            ui.label("不开 TUN、不改系统代理，只给 Antigravity 一个进程级代理。");
            ui.label(format!("版本 v{}", env!("CARGO_PKG_VERSION")));
            ui.label("联系作者：mouse8867@gmail.com");
            ui.label("独立社区工具，与 Google 无隶属关系；不提供任何代理节点、账号或订阅。");
            ui.add_space(8.0);
            ui.label("如果它帮你省去了开 TUN 的麻烦，愿意的话请作者垫吧半个面包（自愿赞赏 ¥1，不影响任何功能）：");
            ui.add_space(6.0);
            // QR codes are pre-decoded textures (see `load_qr_texture`):
            // alpha.1 used `Image::from_bytes` and showed only ⚠
            // placeholders here. Displayed at 130px wide with the QR's
            // own aspect ratio — never stretched, a stretched QR won't scan.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    self.qr_image(ui, &self.alipay_tex, "支付宝 · ❤ 感谢支持");
                });
                ui.vertical(|ui| {
                    self.qr_image(ui, &self.wechat_tex, "微信支付 · ❤ 感谢支持");
                });
            });
            if let Some(err) = &self.qr_error {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(err.clone())
                        .size(12.0)
                        .color(theme::TEXT_DIM),
                );
            }
        });

        if changed {
            self.cfg.save();
        }
    }
}

/// Window/taskbar icon from the self-drawn assets/icon.png.
fn window_icon() -> Option<std::sync::Arc<egui::viewport::IconData>> {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(std::sync::Arc::new(egui::viewport::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }))
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("AgyLite · Antigravity 免 TUN 启动器")
        .with_inner_size([620.0, 600.0])
        .with_min_inner_size([540.0, 500.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "AgyLite",
        options,
        Box::new(|cc| Ok(Box::new(AgyApp::new(cc)))),
    )
}
