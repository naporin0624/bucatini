//! Minimal launcher GUI for bucatini.

// Suppress the extra console window on Windows GUI launch (release builds only,
// so debug builds keep stdout/stderr for troubleshooting).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod menu;
mod theme;
mod tray;
mod usage;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use bucatini::ndi::{Finder, Ndi, Receiver, Source};
use bucatini::output::make_output;
use bucatini::run::run_capture_loop;
use eframe::egui;
use menu::{build_app_menu, AppMenu, MenuAction};
use theme::install_style;
use tray::{build_tray, AppTray, TrayAction};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use usage::UsageWindow;

/// Result of one discovery pass, sent from the worker to the UI.
enum DiscoverMsg {
    Ok(Vec<Source>),
    Err(String),
}

const DISCOVER_TIMEOUT_MS: u32 = 2000;

/// Shared between the UI thread and the receive worker.
struct RunState {
    /// `true` = keep running; UI sets `false` to request stop.
    stop: AtomicBool,
    /// Frames published so far (UI displays this live).
    frames: AtomicU64,
    /// Set by the worker if setup or the loop errored.
    error: Mutex<Option<String>>,
}

impl RunState {
    fn new() -> Self {
        RunState {
            stop: AtomicBool::new(true),
            frames: AtomicU64::new(0),
            error: Mutex::new(None),
        }
    }
}

/// Worker body: build the pipeline and pump frames until stopped.
/// All FFI objects are created and dropped here — none cross threads.
fn run_worker(source: &Source, name: &str, shared: &RunState) -> anyhow::Result<()> {
    let ndi = Ndi::new()?;
    let receiver = Receiver::new(&ndi, source, "Bucatini")?;
    let mut out = make_output(name)?;
    run_capture_loop(&receiver, &mut *out, &shared.stop, &shared.frames, false)?;
    Ok(())
}

struct RunHandle {
    shared: Arc<RunState>,
    join: thread::JoinHandle<()>,
}

struct GuiApp {
    sources: Vec<Source>,
    selected: usize,
    status: String,
    discovering: bool,
    disco_rx: Option<MpscReceiver<DiscoverMsg>>,
    running: Option<RunHandle>,
    tray: Option<AppTray>,
    /// `None` when `build_app_menu` failed — the app still runs without a
    /// native menu bar (same failure-tolerance as `tray`).
    app_menu: Option<AppMenu>,
    usage: UsageWindow,
    /// Set to `true` when the user chooses Quit from the tray menu so the
    /// close-interceptor lets the `Close` command through instead of hiding.
    quitting: bool,
    /// Last content height the window was fitted to (avoids re-sending resizes).
    fit_h: f32,
}

impl GuiApp {
    fn new(ctx: &egui::Context) -> Self {
        install_style(ctx);
        // eframe has already created NSApplication by the time `new` runs, so
        // installing the app menu here (rather than in `main`) is safe on macOS.
        let mut app = GuiApp {
            sources: Vec::new(),
            selected: 0,
            status: String::new(),
            discovering: false,
            disco_rx: None,
            running: None,
            tray: build_tray().ok(),
            app_menu: build_app_menu().ok(),
            usage: UsageWindow::default(),
            quitting: false,
            fit_h: 0.0,
        };
        app.start_discovery(ctx);
        app
    }

    fn start_discovery(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.disco_rx = Some(rx);
        self.discovering = true;
        self.status = "Searching for NDI sources\u{2026}".to_owned();
        spawn_discovery(tx, ctx.clone());
    }

    /// Drain the discovery channel if a result has arrived.
    fn poll_discovery(&mut self) {
        let Some(rx) = &self.disco_rx else { return };
        match rx.try_recv() {
            Ok(DiscoverMsg::Ok(sources)) => {
                self.sources = sources;
                self.selected = 0;
                self.discovering = false;
                self.disco_rx = None;
                if self.sources.is_empty() {
                    self.status = "No NDI sources found.".to_owned();
                } else {
                    self.status.clear();
                }
            }
            Ok(DiscoverMsg::Err(e)) => {
                self.discovering = false;
                self.disco_rx = None;
                self.status = format!("Discovery failed: {e}");
            }
            Err(mpsc::TryRecvError::Empty) => {} // nothing yet
            Err(mpsc::TryRecvError::Disconnected) => {
                self.discovering = false;
                self.disco_rx = None;
                self.status = "Discovery thread exited unexpectedly.".to_owned();
            }
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        let Some(source) = self.sources.get(self.selected).cloned() else {
            return;
        };
        let name = source.name.clone();
        let shared = Arc::new(RunState::new());
        let worker_shared = shared.clone();
        let ctx2 = ctx.clone();
        let join = thread::spawn(move || {
            if let Err(e) = run_worker(&source, &name, &worker_shared) {
                *worker_shared.error.lock().unwrap() = Some(e.to_string());
            }
            ctx2.request_repaint();
        });
        self.status.clear();
        self.running = Some(RunHandle { shared, join });
    }

    fn stop(&mut self) {
        if let Some(handle) = self.running.take() {
            handle.shared.stop.store(false, Ordering::SeqCst);
            let _ = handle.join.join();
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            self.status = format!("Stopped \u{00B7} {} frames", group_thousands(frames));
        }
    }

    /// Signal the worker to stop, join it, then send a Close to the viewport.
    /// Call this for any real-quit path (tray Quit menu or Cmd/Ctrl+Q).
    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        self.stop();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// One-line status for every non-live state (no leading glyph).
    fn idle_label(&self) -> String {
        if self.discovering {
            "Searching\u{2026}".to_owned()
        } else if self.sources.is_empty() {
            "No NDI sources found".to_owned()
        } else if self.status.is_empty() {
            "Ready".to_owned()
        } else {
            self.status.clone()
        }
    }

    /// Single-line summary for the tray menu (live folds onto one line).
    fn info_line(&self) -> String {
        if let Some(handle) = &self.running {
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            let name = self
                .sources
                .get(self.selected)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            format!(
                "Live \u{00B7} {} frames \u{00B7} {} \"{}\"",
                group_thousands(frames),
                bucatini::output::output_kind(),
                name
            )
        } else {
            self.idle_label()
        }
    }

    /// If the worker exited on its own (error or natural stop), surface it and reset.
    fn poll_running(&mut self) {
        let crashed = self
            .running
            .as_ref()
            .map(|h| h.join.is_finished())
            .unwrap_or(false);
        if crashed {
            if let Some(handle) = self.running.take() {
                let _ = handle.join.join();
                let err = handle.shared.error.lock().unwrap().take();
                self.status = match err {
                    Some(e) => format!("Error: {e}"),
                    None => "Stopped.".to_owned(),
                };
            }
        }
    }

    fn handle_quit_shortcut(&mut self, ctx: &egui::Context) {
        // Cmd/Ctrl+Q quits for real (the window ✕ only hides to tray).
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Q)) {
            self.quit(ctx);
        }
    }

    fn handle_close_to_tray(&self, ctx: &egui::Context) {
        // ✕ hides to tray instead of quitting; the receive worker keeps
        // running. When `quitting` is set, let the Close through so eframe
        // actually exits — don't cancel it.
        if !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    /// Drains the one global `MenuEvent` channel that both the tray menu and
    /// the native app menu bar post to (see `tray.rs`'s module doc), plus the
    /// separate tray click channel.
    fn poll_native_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let tray_action = self.tray.as_ref().and_then(|t| t.action_for(&ev.id));
            let menu_action = self.app_menu.as_ref().and_then(|m| m.action_for(&ev.id));
            match (tray_action, menu_action) {
                (Some(TrayAction::Quit), _) => self.quit(ctx),
                (Some(TrayAction::Show), _) => show_window(ctx),
                (Some(TrayAction::OpenUsage), _) | (_, Some(MenuAction::OpenUsage)) => {
                    self.usage.request_open(ctx);
                }
                (None, None) => {}
            }
        }
        // Left-click / double-click on the tray icon → show the window.
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                ev,
                TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }
            ) {
                show_window(ctx);
            }
        }
    }

    fn draw_header(&self, ui: &mut egui::Ui) {
        ui.heading(format!("NDI \u{2192} {}", bucatini::output::output_kind()));
        ui.add_space(8.0);
    }

    fn draw_source_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Refresh icon pinned to the right at its natural width; the dropdown
        // fills the remaining width to its left and truncates — so select +
        // icon always fit the window, no pixel math.
        ui.horizontal(|ui| {
            ui.label("Source:");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(self.running.is_none() && !self.discovering, |ui| {
                    if ui.button("\u{1F504}").on_hover_text("Refresh").clicked() {
                        self.start_discovery(ctx);
                    }
                });
                let combo_w = ui.available_width();
                let sources = &self.sources;
                let selected = &mut self.selected;
                let label = sources
                    .get(*selected)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "(none)".to_owned());
                ui.add_enabled_ui(self.running.is_none() && !sources.is_empty(), |ui| {
                    egui::ComboBox::from_id_salt("ndi_source")
                        .width(combo_w)
                        .truncate()
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (i, s) in sources.iter().enumerate() {
                                ui.selectable_value(selected, i, &s.name);
                            }
                        });
                });
            });
        });
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let is_running = self.running.is_some();
        let can_start = !self.sources.is_empty();
        let mut start_clicked = false;
        let mut stop_clicked = false;
        ui.horizontal(|ui| {
            if is_running {
                stop_clicked = ui.button("\u{25A0} Stop").clicked();
            } else {
                start_clicked = ui
                    .add_enabled(can_start, egui::Button::new("\u{25B6} Start"))
                    .clicked();
            }
        });
        if start_clicked {
            self.start(ctx);
        }
        if stop_clicked {
            self.stop();
        }
    }

    fn draw_status(&self, ui: &mut egui::Ui) {
        use egui::{Color32, RichText};
        const ACCENT: Color32 = Color32::from_rgb(0x39, 0x96, 0xFF);
        const TEXT: Color32 = Color32::from_rgb(0xEE, 0xEF, 0xF2);
        const TEXT_MUTED: Color32 = Color32::from_rgb(0xC9, 0xCC, 0xD2);
        const TEXT_DIM: Color32 = Color32::from_rgb(0x9E, 0x9E, 0x9E);

        if let Some(handle) = &self.running {
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            let name = self
                .sources
                .get(self.selected)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            // Line 1: accent dot as the live indicator, neutral text for the
            // frame throughput (proof the stream is flowing).
            ui.horizontal(|ui| {
                ui.colored_label(ACCENT, "\u{25CF}");
                ui.colored_label(
                    TEXT,
                    format!("Live \u{00B7} {} frames", group_thousands(frames)),
                );
            });
            // Line 2: the published server name a downstream app selects.
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "Publishing as {} \"{}\"",
                        bucatini::output::output_kind(),
                        name
                    ))
                    .color(TEXT_MUTED),
                )
                .wrap(),
            );
        } else {
            // Single dim line; wraps so a long error/status never widens the window.
            ui.add(
                egui::Label::new(
                    RichText::new(format!("\u{25CB} {}", self.idle_label())).color(TEXT_DIM),
                )
                .wrap(),
            );
        }
    }

    fn sync_tray_status(&self) {
        if let Some(tray) = &self.tray {
            tray.status
                .set_text(format!("Bucatini — {}", self.info_line()));
        }
    }

    fn fit_window(&mut self, ui: &egui::Ui, ctx: &egui::Context) {
        // Fit the window height to the content (width stays fixed) so it is
        // tight in both idle and running. Only resend when the content height
        // actually changes.
        let content_h = ui.min_rect().height() + 16.0;
        if (content_h - self.fit_h).abs() > 1.0 {
            self.fit_h = content_h;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                400.0, content_h,
            )));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

/// Run `Finder::list` off the UI thread, then wake the UI.
fn spawn_discovery(tx: Sender<DiscoverMsg>, ctx: egui::Context) {
    thread::spawn(move || {
        let result = (|| -> anyhow::Result<Vec<Source>> {
            let ndi = Ndi::new()?;
            let finder = Finder::new(&ndi)?;
            Ok(finder.list(DISCOVER_TIMEOUT_MS))
        })();
        let msg = match result {
            Ok(sources) => DiscoverMsg::Ok(sources),
            Err(e) => DiscoverMsg::Err(e.to_string()),
        };
        let _ = tx.send(msg);
        ctx.request_repaint();
    });
}

fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

// `App::ui` hands us the central panel's `Ui` directly (no `CentralPanel`
// wrapper needed); obtain the `Context` via `ui.ctx()`.
impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.poll_discovery();
        self.poll_running();

        self.handle_quit_shortcut(&ctx);
        self.handle_close_to_tray(&ctx);
        self.poll_native_events(&ctx);

        self.draw_header(ui);
        self.draw_source_row(ui, &ctx);
        self.draw_controls(ui, &ctx);
        self.draw_status(ui);
        self.sync_tray_status();
        self.fit_window(ui, &ctx);

        // Self-gated: does nothing when the usage window isn't open.
        self.usage.show(&ctx);
    }
}

/// Format an integer with thousands separators (e.g. `1240` -> `"1,240"`).
fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + (len.saturating_sub(1)) / 3);
    for (i, ch) in digits.char_indices() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn main() -> eframe::Result {
    // Fixed-width launcher: the app fits the window height to its content each
    // frame (see the InnerSize command in `ui`); width stays 400 and the window
    // is non-resizable so the auto-fit never fights a manual drag.
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([400.0, 165.0])
        .with_resizable(false);
    // Window / Dock / taskbar icon, decoded from the bundled app icon PNG.
    // Skip silently if decoding ever fails — the app still runs without an icon.
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon/bucatini-1024.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Bucatini",
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(&cc.egui_ctx)))),
    )
}

#[cfg(test)]
mod tests {
    use super::group_thousands;

    #[test]
    fn groups_thousands_with_commas() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(42), "42");
        assert_eq!(group_thousands(100), "100");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(1_000), "1,000");
        assert_eq!(group_thousands(1_240), "1,240");
        assert_eq!(group_thousands(1_234_567), "1,234,567");
    }
}
