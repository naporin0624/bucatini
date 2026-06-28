//! Minimal launcher GUI for ndi-share.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;
use ndi_share::ndi::{Finder, Ndi, Receiver, Source};
use ndi_share::output::make_output;
use ndi_share::run::run_capture_loop;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Owns the tray icon + the handles needed to update/route its menu.
struct TrayState {
    _icon: TrayIcon,
    status: MenuItem,
    quit_id: MenuId,
}

/// A tiny solid accent-blue square icon (32x32 RGBA), generated in code.
fn tray_icon_image() -> tray_icon::Icon {
    const N: u32 = 32;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for _ in 0..(N * N) {
        rgba.extend_from_slice(&[0x39, 0x96, 0xFF, 0xFF]);
    }
    tray_icon::Icon::from_rgba(rgba, N, N).expect("valid 32x32 RGBA icon")
}

/// Build the tray icon with a status line and Quit. Returns None on failure
/// (the app still runs windowed without a tray).
fn build_tray() -> Option<TrayState> {
    let menu = Menu::new();
    let status = MenuItem::new("Idle", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&status).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;
    let icon = TrayIconBuilder::new()
        .with_tooltip("ndi-share")
        .with_icon(tray_icon_image())
        .with_menu(Box::new(menu))
        .build()
        .ok()?;
    Some(TrayState { _icon: icon, status, quit_id: quit.id().clone() })
}

/// Install the bundled LINE Seed JP font and the cannelloni dark theme.
fn install_style(ctx: &egui::Context) {
    use std::sync::Arc;

    // --- Fonts: LINE Seed JP as the primary proportional face, keeping
    //     egui's default fonts (incl. emoji) as fallbacks. ---
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "LINESeedJP".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../vendor/fonts/LINESeedJP-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "LINESeedJP".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("LINESeedJP".to_owned());
    ctx.set_fonts(fonts);

    // --- Theme: cannelloni (dark, neo-brutalist: 0 corners, visible borders, no shadow). ---
    use egui::{Color32, CornerRadius, Stroke};
    let canvas = Color32::from_rgb(0x12, 0x12, 0x12);
    let surface = Color32::from_rgb(0x1C, 0x1C, 0x1C);
    let muted = Color32::from_rgb(0x26, 0x26, 0x26);
    let emphasis = Color32::from_rgb(0x2E, 0x2E, 0x2E);
    let text = Color32::from_rgb(0xEE, 0xEF, 0xF2);
    let text_muted = Color32::from_rgb(0xC9, 0xCC, 0xD2);
    let border = Color32::from_rgb(0x69, 0x69, 0x69);
    let border_subtle = Color32::from_rgb(0x42, 0x42, 0x42);
    let border_strong = Color32::from_rgb(0x86, 0x86, 0x86);
    let accent = Color32::from_rgb(0x39, 0x96, 0xFF);

    let mut v = egui::Visuals::dark();
    v.panel_fill = canvas;
    v.window_fill = canvas;
    v.faint_bg_color = surface;
    v.extreme_bg_color = surface;
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);

    let w = &mut v.widgets;
    // noninteractive: labels / panel
    w.noninteractive.bg_fill = canvas;
    w.noninteractive.weak_bg_fill = canvas;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border_subtle);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text_muted);
    w.noninteractive.corner_radius = CornerRadius::ZERO;
    // inactive: idle buttons / combo
    w.inactive.bg_fill = surface;
    w.inactive.weak_bg_fill = surface;
    w.inactive.bg_stroke = Stroke::new(1.0, border);
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.corner_radius = CornerRadius::ZERO;
    // hovered
    w.hovered.bg_fill = muted;
    w.hovered.weak_bg_fill = muted;
    w.hovered.bg_stroke = Stroke::new(1.0, border_strong);
    w.hovered.fg_stroke = Stroke::new(1.0, text);
    w.hovered.corner_radius = CornerRadius::ZERO;
    // active (pressed) + open (combo open)
    for wv in [&mut w.active, &mut w.open] {
        wv.bg_fill = emphasis;
        wv.weak_bg_fill = emphasis;
        wv.bg_stroke = Stroke::new(1.0, accent);
        wv.fg_stroke = Stroke::new(1.0, text);
        wv.corner_radius = CornerRadius::ZERO;
    }

    ctx.set_visuals(v);
    ctx.global_style_mut(|s| s.spacing.item_spacing = egui::vec2(8.0, 8.0));
}

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
    let receiver = Receiver::new(&ndi, source, "ndi-share")?;
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
    tray: Option<TrayState>,
    /// Set to `true` when the user chooses Quit from the tray menu so the
    /// close-interceptor lets the `Close` command through instead of hiding.
    quitting: bool,
}

impl GuiApp {
    fn new(ctx: &egui::Context) -> Self {
        install_style(ctx);
        let mut app = GuiApp {
            sources: Vec::new(),
            selected: 0,
            status: String::new(),
            discovering: false,
            disco_rx: None,
            running: None,
            tray: build_tray(),
            quitting: false,
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
        let Some(source) = self.sources.get(self.selected).cloned() else { return };
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
            self.status = format!("Stopped after {frames} frames.");
        }
    }

    /// One-line status shown after `info:`.
    fn info_line(&self) -> String {
        if let Some(handle) = &self.running {
            let frames = handle.shared.frames.load(Ordering::SeqCst);
            let name = self
                .sources
                .get(self.selected)
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            format!("{frames} frames · {name} · {}", ndi_share::output::output_kind())
        } else if self.discovering {
            "searching\u{2026}".to_owned()
        } else if self.sources.is_empty() {
            "no NDI sources".to_owned()
        } else if self.status.is_empty() {
            "ready".to_owned()
        } else {
            self.status.clone()
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

// NOTE: eframe 0.35's `App` trait requires `fn ui(&mut self, ui: &mut egui::Ui,
// frame: &mut Frame)` — the older `fn update(&mut self, ctx, frame)` does NOT
// exist in 0.35. The `ui` param IS the central panel's Ui (the framework wraps
// it for you), so build directly into `ui` — no `CentralPanel` wrapper. Get the
// Context via `ui.ctx()` (clone it once up front for the worker spawns + repaint).
impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_discovery();
        self.poll_running();

        // ✕ hides to tray instead of quitting; the receive worker keeps running.
        // When `quitting` is true (tray Quit was chosen), let the Close through
        // so eframe actually exits — don't cancel it.
        if !self.quitting && ui.ctx().input(|i| i.viewport().close_requested()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // Quit / show from the tray menu.
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let is_quit = self.tray.as_ref().map_or(false, |t| ev.id == t.quit_id);
            let is_show = self.tray.as_ref().map_or(false, |t| ev.id == t.status.id());
            if is_quit {
                self.quitting = true;
                self.stop();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else if is_show {
                show_window(ui.ctx());
            }
        }
        // Left-click / double-click on the tray icon → show the window.
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if matches!(ev, TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }) {
                show_window(ui.ctx());
            }
        }

        let ctx = ui.ctx().clone();

        ui.heading(format!("NDI \u{2192} {}", ndi_share::output::output_kind()));
        ui.add_space(8.0);

        // Source row: dropdown + refresh icon on one line.
        ui.horizontal(|ui| {
            ui.label("Source:");
            let sources = &self.sources;
            let selected = &mut self.selected;
            let label = sources
                .get(*selected)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "(none)".to_owned());
            ui.add_enabled_ui(self.running.is_none() && !sources.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("ndi_source")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for (i, s) in sources.iter().enumerate() {
                            ui.selectable_value(selected, i, &s.name);
                        }
                    });
            });
            ui.add_enabled_ui(self.running.is_none() && !self.discovering, |ui| {
                if ui.button("\u{1F504}").on_hover_text("Refresh").clicked() {
                    self.start_discovery(&ctx);
                }
            });
        });

        // Start / Stop.
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
            self.start(&ctx);
        }
        if stop_clicked {
            self.stop();
        }

        // Running indicator (accent dot) + info line.
        if self.running.is_some() {
            ui.colored_label(egui::Color32::from_rgb(0x39, 0x96, 0xFF), "\u{25CF} Running");
        }
        ui.label(format!("info: {}", self.info_line()));

        if let Some(tray) = &self.tray {
            tray.status.set_text(format!("ndi-share — {}", self.info_line()));
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([360.0, 170.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ndi-share",
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(&cc.egui_ctx)))),
    )
}
