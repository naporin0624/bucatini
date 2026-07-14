//! "使い方" (usage) window — a README-equivalent reachable from the app menu and
//! the tray, with an EN/JA toggle. Content is curated for the GUI rather than a
//! raw README embed, so it stays short enough to fit a small viewport.

use eframe::egui;

/// Content is loosely structured as `\n\n`-separated blocks: a block starting
/// with `# ` renders as a heading, a block starting with `http` renders as a
/// hyperlink, and everything else renders as a wrapped label. This keeps the
/// source a single readable string per language while still avoiding one giant
/// monospace blob on screen (see `render_content`).
const USAGE_EN: &str = "\
# What Bucatini does
Bucatini receives a single NDI source on your network and republishes it as a shared texture \u{2014} a Syphon server on macOS, or a Spout sender on Windows \u{2014} so other apps on the same machine can pick it up in real time.

# Getting started
1. Pick an NDI source from the Source dropdown. If yours is missing, press \u{1F504} to re-scan the network.
2. Press \u{25B6} Start to begin publishing. The published server name matches the NDI source name you selected.
3. Press \u{25A0} Stop to stop publishing at any time.

# While it's running
Closing the window with \u{2715} doesn't quit \u{2014} it hides Bucatini to the tray and keeps publishing in the background. Reopen it any time from the tray icon.
Cmd/Ctrl+Q, or Quit from the tray menu, exits the app for real.

# Learn more
https://github.com/naporin0624/bucatini";

const USAGE_JA: &str = "\
# Bucatini について
Bucatini は、ネットワーク上の NDI ソースを1つ受信し、共有テクスチャとして再配信する小さなツールです。macOS では Syphon サーバーとして、Windows では Spout センダーとして配信するので、同じマシン上の他のアプリからリアルタイムに取り込めます。

# 使いかた
1. Source のドロップダウンから NDI ソースを選びます。見つからない場合は \u{1F504} でネットワークを再スキャンしてください。
2. \u{25B6} Start を押すと配信を開始します。配信されるサーバー名は選択した NDI ソース名と同じです。
3. いつでも \u{25A0} Stop で配信を停止できます。

# 動作について
ウィンドウを \u{2715} で閉じても終了せず、トレイに隠れて配信を続けます。トレイアイコンからいつでも再表示できます。
Cmd/Ctrl+Q、またはトレイメニューの Quit で完全に終了します。

# もっと詳しく
https://github.com/naporin0624/bucatini";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    En,
    #[default]
    Ja,
}

impl Lang {
    fn content(self) -> &'static str {
        match self {
            Lang::En => USAGE_EN,
            Lang::Ja => USAGE_JA,
        }
    }
}

fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("bucatini-usage-window")
}

/// State for the "使い方" viewport. `open` gates whether `show` renders it this
/// frame; `show` self-clears `open` when the user closes the native window.
#[derive(Default)]
pub struct UsageWindow {
    open: bool,
    lang: Lang,
}

impl UsageWindow {
    /// Opens the window, or focuses it if it's already open.
    pub fn request_open(&mut self, ctx: &egui::Context) {
        if self.open {
            ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
        } else {
            self.open = true;
        }
    }

    /// Draws the viewport if open. Call unconditionally every frame — it
    /// self-gates on `open` and does nothing otherwise.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let mut still_open = true;
        let lang = &mut self.lang;
        ctx.show_viewport_immediate(
            viewport_id(),
            egui::ViewportBuilder::default()
                .with_title("\u{4F7F}\u{3044}\u{65B9} \u{2014} Bucatini")
                .with_inner_size([440.0, 420.0]),
            |ui, _class| {
                ui.horizontal(|ui| {
                    ui.selectable_value(lang, Lang::En, "EN");
                    ui.selectable_value(lang, Lang::Ja, "\u{65E5}\u{672C}\u{8A9E}");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_content(ui, lang.content());
                });

                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );
        self.open = still_open;
    }
}

/// Renders `\n\n`-separated content blocks as headings, hyperlinks, or wrapped
/// labels (see the doc comment on `USAGE_EN`/`USAGE_JA`) instead of dumping the
/// whole string into a single monospace widget.
fn render_content(ui: &mut egui::Ui, content: &str) {
    for block in content.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        if let Some(heading) = block.strip_prefix("# ") {
            ui.heading(heading);
        } else if block.starts_with("http") {
            ui.hyperlink(block);
        } else {
            ui.add(egui::Label::new(block).wrap());
        }
        ui.add_space(8.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{Lang, UsageWindow, USAGE_EN, USAGE_JA};

    #[test]
    fn default_lang_is_ja() {
        assert_eq!(UsageWindow::default().lang, Lang::Ja);
    }

    #[test]
    fn en_content_is_non_empty_and_covers_the_key_terms() {
        assert!(!USAGE_EN.is_empty());
        for term in ["NDI", "Syphon", "Spout"] {
            assert!(USAGE_EN.contains(term), "EN content missing {term:?}");
        }
    }

    #[test]
    fn ja_content_is_non_empty_and_covers_the_key_terms() {
        assert!(!USAGE_JA.is_empty());
        for term in ["NDI", "Syphon", "Spout"] {
            assert!(USAGE_JA.contains(term), "JA content missing {term:?}");
        }
    }
}
