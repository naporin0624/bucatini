# GUI Contextual Status + WCAG AA Tokens — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `src/bin/gui.rs` のステータス表示を文脈的・非機械的（パターンB / 英語）に再設計し、color token を WCAG 2.1 AA 準拠にする。

**Architecture:** PR #12 で分割済みの `GuiApp` メソッド構成を維持。純ヘルパ `group_thousands` を TDD で追加し、`install_style`（トークン）/ `draw_status` / `info_line` / `idle_label`（新）/ `sync_tray_status` / `stop` の本文を差し替える。ロジック・FFI・CLI は不変。

**Tech Stack:** Rust 2021, eframe/egui 0.35。

## Global Constraints

- 変更は `src/bin/gui.rs` のみ。
- コピーは全面英語、構造のみ改善（`info:` 接頭辞撤廃、idle→Ready / running→Live）。
- WCAG 値は確定済み・厳守: `text_dim = #9E9E9E`（draw_status ローカル const、本文 6.99:1）、`border_subtle = #696969`（install_style、surface 3.10:1）。accent `#3996FF` / text `#EEEFF2` / text_muted `#C9CCD2` は据え置き（既に AA）。
- frame 数は 3 桁区切り（`group_thousands`）。
- 完了ゲート: `cargo fmt --all -- --check` / `cargo clippy --all-targets --features gui -- -D warnings` / `cargo test --lib`（18 維持）/ `cargo test --features gui --bin bucatini-gui`（group_thousands テスト）/ `cargo build --features gui --bin bucatini-gui`。
- bin 名は `bucatini-gui`。`git` コミットは実装者は行わない（コントローラがブランチ上で実施）。

---

## File Structure

- `src/bin/gui.rs` — Modify のみ。`group_thousands`（自由関数 + `#[cfg(test)] mod tests`）追加、`install_style` の `border_subtle` 1 行、`draw_status` / `info_line` / `sync_tray_status` / `stop` 本文、`idle_label`（新メソッド）追加。

Task 1（純ヘルパ + テスト）→ Task 2（トークン + 表示）の順。Task 2 は `group_thousands` を使うため Task 1 に依存。

---

## Task 1: group_thousands 純ヘルパ（TDD）

**Files:**
- Modify: `src/bin/gui.rs`（自由関数 + 末尾に `#[cfg(test)] mod tests`）
- Test: 同ファイル `mod tests`

**Interfaces:**
- Consumes: なし。
- Produces: `fn group_thousands(n: u64) -> String`（ファイルスコープの自由関数）。Task 2 が使用。

- [ ] **Step 1: 失敗するテストを追加**

`src/bin/gui.rs` の末尾（`fn main` の後）に追加:

```rust
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
```

- [ ] **Step 2: テストが（未定義で）失敗することを確認**

Run: `cargo test --features gui --bin bucatini-gui 2>&1 | tail -15`
Expected: コンパイルエラー `cannot find function 'group_thousands'`（=RED）。

- [ ] **Step 3: 最小実装を追加**

`src/bin/gui.rs` に自由関数を追加（`fn main` の直前など、`impl` ブロック外）:

```rust
/// Format an integer with thousands separators (e.g. `1240` -> `"1,240"`).
fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + (len.saturating_sub(1)) / 3);
    for (i, ch) in digits.char_indices() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cargo test --features gui --bin bucatini-gui 2>&1 | tail -8`
Expected: `groups_thousands_with_commas ... ok`、`test result: ok. 1 passed`。

- [ ] **Step 5: fmt / clippy**

Run: `cargo fmt --all && cargo clippy --all-targets --features gui -- -D warnings`
Expected: 警告なし。

（コミットはコントローラが行う。実装者はここで停止し報告。）

---

## Task 2: WCAG token + 文脈的ステータス表示

**Files:**
- Modify: `src/bin/gui.rs` — `install_style`（1 行）、`draw_status` / `info_line` / `sync_tray_status` / `stop` 本文、`idle_label` 追加。

**Interfaces:**
- Consumes: `group_thousands`（Task 1）、既存フィールド `running` / `sources` / `selected` / `discovering` / `status`、`bucatini::output::output_kind()`。
- Produces: `idle_label(&self) -> String`（非 live 状態の 1 行ラベル）。`info_line` は単一行トレイサマリを返す責務に変更。

- [ ] **Step 1: install_style の border_subtle を AA 値へ**

`src/bin/gui.rs` の該当行を変更:

```rust
    let border_subtle = Color32::from_rgb(0x42, 0x42, 0x42);
```

を

```rust
    let border_subtle = Color32::from_rgb(0x69, 0x69, 0x69);
```

に。他のトークン行は変更しない。

- [ ] **Step 2: idle_label メソッドを追加**

`impl GuiApp` 内（`info_line` の直前）に追加:

```rust
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
```

- [ ] **Step 3: info_line を単一行トレイサマリへ書き換え**

既存 `info_line` 全体を置き換え:

```rust
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
```

- [ ] **Step 4: draw_status を文脈ブロックへ書き換え**

既存 `draw_status` 全体を置き換え:

```rust
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
```

- [ ] **Step 5: sync_tray_status の改行置換を撤去**

既存 `sync_tray_status` を置き換え（`info_line` が単一行になったため `replace` 不要）:

```rust
    fn sync_tray_status(&self) {
        if let Some(tray) = &self.tray {
            tray.status.set_text(format!("Bucatini — {}", self.info_line()));
        }
    }
```

- [ ] **Step 6: stop() の停止メッセージを新コピーへ**

`stop` 内の status 文字列を変更:

```rust
            self.status = format!("Stopped after {frames} frames.");
```

を

```rust
            self.status = format!("Stopped \u{00B7} {} frames", group_thousands(frames));
```

に（idle_label の else 経由で `○ Stopped · 1,240 frames` と表示される）。`stop` の他の行（`handle.shared.stop.store(...)` / `join` / `frames` 取得）は変更しない。

- [ ] **Step 7: fmt / clippy / build / test**

Run:
```
cargo fmt --all && \
cargo clippy --all-targets --features gui -- -D warnings && \
cargo build --features gui --bin bucatini-gui && \
cargo test --lib && \
cargo test --features gui --bin bucatini-gui
```
Expected: clippy 0 警告、build 成功、lib 18/18、bin 1/1（group_thousands）。

- [ ] **Step 8: 自己レビュー（挙動・WCAG）**

Run: `git diff src/bin/gui.rs`
確認: 変更は border_subtle 1 行 + idle_label 追加 + info_line/draw_status/sync_tray_status/stop 本文のみ。live は2行（accent ドット + text "Live · N frames" / text_muted "Publishing as …"）、非 live は text_dim 1行。`info:` 接頭辞が消えていること。`group_thousands` 経由で frame に区切りが入ること。WCAG 値（`#9E9E9E` / `#696969`）が指定どおりであること。

（コミットはコントローラが行う。実装者は報告のみ。）

---

## Self-Review

**1. Spec coverage:**
- WCAG token（text_dim 新規 / border_subtle 引き上げ）→ Task 2 Step 1（border_subtle）, Step 4（text_dim const）。
- パターンB ステータス表示（live 2行 / 非live 1行、`info:` 撤廃）→ Task 2 Step 4。
- frame 3桁区切り → Task 1（group_thousands）+ Task 2 Step 3/4/6。
- トレイ単一行化 → Task 2 Step 3/5。
- 停止コピー → Task 2 Step 6。
- group_thousands ユニットテスト → Task 1 Step 1-4。

**2. Placeholder scan:** "TBD"/"適切に" なし。全コードステップに実コードを記載。

**3. Type consistency:**
- `group_thousands(n: u64) -> String`：Task 1 で定義、Task 2 が `frames`（`AtomicU64::load` = u64）に適用。型一致。
- `idle_label(&self) -> String`：Task 2 Step 2 で定義、Step 3（info_line）と Step 4（draw_status）が使用。命名一致。
- `info_line` の戻り値は単一行 String、`sync_tray_status` が `format!` で消費。`replace('\n', ...)` 撤去と整合（改行を生成しない）。
- 色定数 `ACCENT #3996FF` / `TEXT #EEEFF2` / `TEXT_MUTED #C9CCD2` / `TEXT_DIM #9E9E9E` は spec の WCAG 値と一致。

問題なし。
