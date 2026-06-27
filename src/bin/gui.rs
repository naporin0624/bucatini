//! Minimal launcher GUI for ndi-share.

use std::sync::mpsc::{self, Receiver as MpscReceiver, Sender};
use std::thread;

use eframe::egui;
use ndi_share::ndi::{Finder, Ndi, Source};

/// Result of one discovery pass, sent from the worker to the UI.
enum DiscoverMsg {
    Ok(Vec<Source>),
    Err(String),
}

const DISCOVER_TIMEOUT_MS: u32 = 2000;

struct GuiApp {
    sources: Vec<Source>,
    selected: usize,
    name: String,
    name_edited: bool,
    status: String,
    discovering: bool,
    disco_rx: Option<MpscReceiver<DiscoverMsg>>,
}

impl GuiApp {
    fn new(ctx: &egui::Context) -> Self {
        let mut app = GuiApp {
            sources: Vec::new(),
            selected: 0,
            name: String::new(),
            name_edited: false,
            status: String::new(),
            discovering: false,
            disco_rx: None,
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
                    if !self.name_edited {
                        self.name = self.sources[0].name.clone();
                    }
                }
            }
            Ok(DiscoverMsg::Err(e)) => {
                self.discovering = false;
                self.disco_rx = None;
                self.status = format!("Discovery failed: {e}");
            }
            Err(_) => {} // nothing yet
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

// NOTE: eframe 0.35's `App` trait requires `fn ui(&mut self, ui: &mut egui::Ui,
// frame: &mut Frame)` — the older `fn update(&mut self, ctx, frame)` does NOT
// exist in 0.35. The `ui` param IS the central panel's Ui (the framework wraps
// it for you), so build directly into `ui` — no `CentralPanel` wrapper. Get the
// Context via `ui.ctx()` (clone it once up front for the worker spawns + repaint).
impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_discovery();
        let ctx = ui.ctx().clone();

        ui.heading(format!("NDI \u{2192} {}", ndi_share::output::output_kind()));
        ui.add_space(8.0);

        // Source dropdown. Split borrows so the closure can hold
        // `&sources` and `&mut selected` at once.
        ui.horizontal(|ui| {
            ui.label("Source:");
            let prev = self.selected;
            let sources = &self.sources;
            let selected = &mut self.selected;
            let label = sources
                .get(*selected)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "(none)".to_owned());
            ui.add_enabled_ui(!sources.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("ndi_source")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for (i, s) in sources.iter().enumerate() {
                            ui.selectable_value(selected, i, &s.name);
                        }
                    });
            });
            // If the user picked a different source and hasn't hand-edited
            // the name, follow the source name.
            if self.selected != prev && !self.name_edited {
                if let Some(s) = self.sources.get(self.selected) {
                    self.name = s.name.clone();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.discovering, |ui| {
                if ui.button("\u{1F504} Refresh").clicked() {
                    self.start_discovery(&ctx);
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Name:");
            if ui.text_edit_singleline(&mut self.name).changed() {
                self.name_edited = true;
            }
        });

        ui.add_space(8.0);
        if !self.status.is_empty() {
            ui.label(&self.status);
        }

        if self.discovering {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ndi-share",
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(&cc.egui_ctx)))),
    )
}
