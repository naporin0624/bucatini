# src/ Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CLAUDE.md 規約からの局所的逸脱を、外部挙動を一切変えずに是正する（gui の `ui()` 分解 / 過去文脈コメント除去 / cli 構造体統合）。

**Architecture:** 既存の層構造（FFI 薄ラッパ → trait 抽象 → 非依存ループ）は維持。`gui.rs::ui()` の巨大メソッドを責務ごとの private メソッドへ抽出してオーケストレータ化し、`cli.rs` の重複構造体を 1 つに統合する。新しい挙動・新機能は追加しない。

**Tech Stack:** Rust 2021, eframe/egui 0.35, clap (derive), tray-icon, anyhow。

## Global Constraints

- 挙動完全保持: GUI 描画順（header → source row → controls → status → tray sync → window fit）、ウィンドウ自動フィット（`|content_h - fit_h| > 1.0`、幅 400 固定）、`request_repaint_after(200ms)`、トレイ挙動、CLI 引数解釈をすべて不変に保つ。
- 各タスクの完了ゲート: `cargo fmt --all -- --check` / `cargo clippy --all-targets --features gui -- -D warnings` / `cargo test` がすべてパス。
- 本リポジトリの pre-commit フックは `cargo fmt --all -- --check && cargo clippy --all-targets --features gui -- -D warnings` を走らせる。コミットはフック通過が前提。
- リファクタにつき新規テストは原則追加しない。既存テスト群が回帰の安全網（`cli.rs`/`run.rs`/`ndi`/`dll.rs`/`syphon.rs` のテスト）。
- 規約準拠の現在形コメント（run.rs の zero-copy 説明等）は残す。削除対象は「過去の文脈」コメントのみ。

---

## File Structure

- `src/cli.rs` — Modify: 重複構造体 `RawArgs` を削除し `Args` を clap derive 構造体に統合（Task 1）。
- `src/bin/gui.rs` — Modify: `ui()` を分解し private メソッド群を追加、過去文脈 NOTE コメントを整理（Task 2）。

Task 1 と Task 2 は別ファイルで相互依存なし。順序は問わない（subagent-driven ではレビューを挟むため逐次でよい）。

---

## Task 1: cli.rs の RawArgs/Args 統合

**Files:**
- Modify: `src/cli.rs:3-46`（`RawArgs` 定義・`Args` 定義・`parse()`）
- Test: `src/cli.rs` 既存 `mod tests`（変更しない。回帰確認のみ）

**Interfaces:**
- Consumes: なし。
- Produces: `pub struct Args { pub list: bool, pub source: Option<String>, pub name: Option<String>, pub timeout_ms: u32, pub verbose: bool }` と `pub fn parse() -> Args`。`main.rs` がこのシグネチャに依存（フィールド名・型は現状と同一なので呼び出し側変更不要）。

- [ ] **Step 1: 現状の回帰ベースラインを取る**

Run: `cargo test 2>&1 | tail -20`
Expected: 既存テスト（`no_match_returns_none` 他、`cli`/`ndi`/`dll`/`run`/`syphon`）が PASS。この出力を基準とする。

- [ ] **Step 2: RawArgs と Args を 1 つの clap derive 構造体へ統合**

`src/cli.rs` の先頭（`use clap::Parser;` の下）から `parse()` までを、以下で置き換える。`RawArgs`・手動コピーを削除し、フィールドを `pub` 化した `Args` に集約する。

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "bucatini",
    about = "Republish an NDI source as a Syphon Metal texture"
)]
#[allow(dead_code)]
pub struct Args {
    /// List discovered NDI sources and exit
    #[arg(long)]
    pub list: bool,
    /// NDI source name (case-insensitive substring match)
    #[arg(long)]
    pub source: Option<String>,
    /// Syphon server name to publish under (default: the NDI source name)
    #[arg(long)]
    pub name: Option<String>,
    /// Discovery / capture timeout in milliseconds
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u32,
    /// Verbose logging (resolution, fps)
    #[arg(long)]
    pub verbose: bool,
}

#[allow(dead_code)]
pub fn parse() -> Args {
    Args::parse()
}
```

`SourceMatch` / `match_source` / `parse_selection` 以降は変更しない。

- [ ] **Step 3: フォーマットと lint**

Run: `cargo fmt --all && cargo clippy --all-targets --features gui -- -D warnings`
Expected: 警告・エラーなしで終了。

- [ ] **Step 4: テストで回帰がないことを確認**

Run: `cargo test 2>&1 | tail -20`
Expected: Step 1 と同じテストがすべて PASS。

- [ ] **Step 5: CLI 解釈が不変であることをスモーク確認**

Run: `cargo run --bin bucatini -- --help`
Expected: usage に `--list` `--source` `--name` `--timeout-ms`（default 5000）`--verbose` が現状どおり表示される。

- [ ] **Step 6: コミット**

```bash
git add src/cli.rs
git commit -m "refactor(cli): unify RawArgs/Args into one clap-derived struct"
```

---

## Task 2: gui.rs の ui() 分解 + 過去文脈コメント除去

**Files:**
- Modify: `src/bin/gui.rs:351-355`（NOTE コメント）と `src/bin/gui.rs:356-489`（`impl eframe::App` の `ui()`）
- Test: GUI に単体テストはない。回帰確認はビルド・clippy・既存 `cargo test` で行う。

**Interfaces:**
- Consumes: 既存 `GuiApp` のフィールド（`sources`/`selected`/`status`/`discovering`/`running`/`tray`/`quitting`/`fit_h`）と既存メソッド（`poll_discovery`/`poll_running`/`quit`/`start`/`stop`/`start_discovery`/`info_line`）、フリー関数 `show_window`。
- Produces: `GuiApp` に private メソッド `handle_quit_shortcut` / `handle_close_to_tray` / `handle_tray_events` / `draw_header` / `draw_source_row` / `draw_controls` / `draw_status` / `sync_tray_status` / `fit_window` を追加。`ui()` はこれらを順に呼ぶオーケストレータになる。

- [ ] **Step 1: 過去文脈 NOTE コメントを現在形へ書き換え**

`src/bin/gui.rs:351-355` の 5 行 NOTE ブロック（"eframe 0.35's App trait ... does NOT exist in 0.35 ..."）を、以下の 2 行に置き換える。

```rust
// `App::ui` hands us the central panel's `Ui` directly (no `CentralPanel`
// wrapper needed); obtain the `Context` via `ui.ctx()`.
```

- [ ] **Step 2: 抽出先メソッドを `impl GuiApp` に追加**

`impl GuiApp { ... }` ブロック内（既存メソッド群の末尾、`poll_running` の後など）に以下を追加する。各メソッドのコメントは元の `ui()` 内コメントを移設したもので、現在の挙動を説明する規約準拠コメント。

```rust
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

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        // Quit / show from the tray menu.
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let is_quit = self.tray.as_ref().is_some_and(|t| ev.id == t.quit_id);
            let is_show = self.tray.as_ref().is_some_and(|t| ev.id == t.status.id());
            if is_quit {
                self.quit(ctx);
            } else if is_show {
                show_window(ctx);
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
        // Status row is always present (running=accent dot, idle=dim) so the
        // line never appears/disappears and shifts the layout vertically.
        if self.running.is_some() {
            ui.colored_label(
                egui::Color32::from_rgb(0x39, 0x96, 0xFF),
                "\u{25CF} Running",
            );
        } else {
            ui.colored_label(egui::Color32::from_rgb(0x69, 0x69, 0x69), "\u{25CB} Idle");
        }
        // Info line wraps within the window width, so a long status (frame
        // count + source name) never widens the window.
        ui.add(egui::Label::new(format!("info: {}", self.info_line())).wrap());
    }

    fn sync_tray_status(&self) {
        if let Some(tray) = &self.tray {
            // Tray menu is single-line: collapse the info newlines into separators.
            tray.status.set_text(format!(
                "Bucatini — {}",
                self.info_line().replace('\n', " \u{30FB} ")
            ));
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
```

- [ ] **Step 3: `ui()` 本体をオーケストレータへ縮小**

`impl eframe::App for GuiApp` の `ui()`（`fn ui(...) { ... }` 全体）を以下で置き換える。呼び出し順は元コードの実行順と一致させること（観測挙動）。

```rust
impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.poll_discovery();
        self.poll_running();

        self.handle_quit_shortcut(&ctx);
        self.handle_close_to_tray(&ctx);
        self.handle_tray_events(&ctx);

        self.draw_header(ui);
        self.draw_source_row(ui, &ctx);
        self.draw_controls(ui, &ctx);
        self.draw_status(ui);
        self.sync_tray_status();
        self.fit_window(ui, &ctx);
    }
}
```

- [ ] **Step 4: フォーマットと lint**

Run: `cargo fmt --all && cargo clippy --all-targets --features gui -- -D warnings`
Expected: 警告・エラーなしで終了（特に未使用変数・借用エラーがないこと）。

- [ ] **Step 5: ビルドと既存テストで回帰がないことを確認**

Run: `cargo build --features gui --bin gui && cargo test 2>&1 | tail -20`
Expected: gui バイナリがビルドでき、既存テストがすべて PASS。

- [ ] **Step 6: 差分が構造的変更のみであることを目視確認**

Run: `git diff src/bin/gui.rs`
Expected: 追加メソッド群・`ui()` 縮小・NOTE コメント 1 箇所の書き換えのみ。ロジック・定数（色 `0x3996FF`/`0x696969`、`400.0`、`16.0`、`1.0`、`200ms`、`from_id_salt("ndi_source")` 等）に変化がないこと。

- [ ] **Step 7: コミット**

```bash
git add src/bin/gui.rs
git commit -m "refactor(gui): split ui() into single-responsibility methods; drop migration-context comment"
```

---

## Self-Review

**1. Spec coverage:**
- A（`ui()` 分解）→ Task 2 Step 2-3。spec の 9 メソッド表とプラン Step 2 の 9 メソッドが一致。
- B（過去文脈コメント除去）→ Task 2 Step 1。grep で唯一該当の `gui.rs:351-355` を対象化。
- C（`cli.rs` 統合）→ Task 1 Step 2。`#[allow(dead_code)]` 残置も spec どおり。
- 検証ゲート（fmt/clippy/test/挙動差分ゼロ）→ 各タスクの Step に展開済み。

**2. Placeholder scan:** "TBD"/"TODO"/"適切に"/"similar to" なし。全コードステップに実コードを記載。

**3. Type consistency:**
- `Args` のフィールド名・型は Task 1 と `main.rs`（`args.list`/`args.source`/`args.name`/`args.timeout_ms`/`args.verbose`）で一致。
- Task 2 の追加メソッド名は `ui()` オーケストレータの呼び出し名（`handle_quit_shortcut`/`handle_close_to_tray`/`handle_tray_events`/`draw_header`/`draw_source_row`/`draw_controls`/`draw_status`/`sync_tray_status`/`fit_window`）と完全一致。
- 既存メソッド（`quit`/`start`/`stop`/`start_discovery`/`info_line`/`show_window`）のシグネチャは変更しないため呼び出し側と整合。

問題なし。
