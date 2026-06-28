# src/ refactor — CLAUDE.md 規約準拠 design

- Date: 2026-06-29
- Scope: `src/` の挙動保持リファクタ（A: `gui.rs::ui()` 分解 / B: 過去文脈コメント除去 / C: `cli.rs` 構造体統合）
- Out of scope: D（FFI ラッパのマクロ化）は意図的な明示分離を尊重し見送り

## 背景と目的

`src/` は既に「FFI 薄ラッパ → trait 抽象（`SharedTextureOutput` / `FrameStream`）→ プラットフォーム非依存ループ」の層構造が整っている。本リファクタは大規模な再設計ではなく、CLAUDE.md コーディング規約からの**局所的な逸脱の是正**に限定する。

対象規約:
- 関数は単一責任で実装すること
- コメントはコードを見てもわからないことに関して書くこと。過去の文脈に対するコメントを書くことを禁止する。
- （DRY）同一構造の重複を避ける

## 最重要契約: 挙動完全保持（behavior-preserving）

このリファクタは**外部から観測可能な挙動を一切変えない**。具体的に不変とするもの:

- GUI の描画順序（heading → source row → controls → status → tray sync → window fit）
- ウィンドウ自動フィット挙動（`InnerSize` の再送条件 `|content_h - fit_h| > 1.0`、幅 400 固定）
- リペイント間隔（`request_repaint_after(200ms)`）
- トレイ挙動（✕ で隠す / Cmd-Ctrl+Q と Tray Quit で終了 / クリックで表示）
- CLI 引数の解釈（フラグ名・default・substring マッチ・選択ロジック）

## A. `gui.rs::ui()` の単一責任分解

### 現状

`impl eframe::App for GuiApp` の `ui()` が約130行で以下を一括処理:
入力ショートカット / クローズ→トレイ / トレイイベント / 各 UI 描画 / トレイ status 同期 / ウィンドウフィット / repaint。

### 変更後

責務ごとに `impl GuiApp` の private メソッドへ抽出し、`ui()` は**呼び出し順を保つオーケストレータ**へ縮小する。

| メソッド | 受け取り | 責務（現状の対応行） |
|---|---|---|
| `handle_quit_shortcut(&mut self, ctx)` | `&mut self` | Cmd/Ctrl+Q → `quit()` |
| `handle_close_to_tray(&self, ctx)` | `&self` | `!quitting && close_requested` 時に CancelClose + Visible(false) |
| `handle_tray_events(&mut self, ctx)` | `&mut self` | `MenuEvent`（quit/show）+ `TrayIconEvent`（click→show） |
| `draw_header(&self, ui)` | `&self` | `ui.heading(...)` + add_space |
| `draw_source_row(&mut self, ui, ctx)` | `&mut self` | Source ラベル + refresh ボタン + ComboBox（`selected` 更新含む） |
| `draw_controls(&mut self, ui, ctx)` | `&mut self` | Start / Stop ボタンと `start()` / `stop()` 呼び出し |
| `draw_status(&self, ui)` | `&self` | Running/Idle カラードット + `info:` ラベル（wrap） |
| `sync_tray_status(&self)` | `&self` | トレイ status テキスト更新（改行→`・`） |
| `fit_window(&mut self, ui, ctx)` | `&mut self` | `InnerSize` 自動フィット + `request_repaint_after` |

`ui()` 本体（最終形のイメージ）:

```rust
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
```

### 制約・注意

- 抽出メソッドは逐次呼び出しのため `&self` / `&mut self` の借用は衝突しない。
- ComboBox クロージャの `self.sources` / `self.selected` 借用は `draw_source_row` 内に閉じ込める（現状の局所変数 `sources` / `selected` バインドを踏襲）。
- 各 `&mut self` 描画メソッドが `ui` と `ctx` の両方を取る箇所は、`ctx` を `ui()` で一度だけ clone して渡す現状方針を維持。
- 描画順は上表の順序を厳守（観測挙動の一部）。

## B. 過去文脈コメントの除去

### 対象

grep（`0.35|older|previously|旧|かつて|used to|no longer|deprecated`）の結果、過去文脈コメントは **`gui.rs:351-355` の NOTE ブロック1箇所のみ**。`dll.rs` の `older` はテスト関数名（NDI バージョンの意味）で対象外。

### 変更後

移行経緯（「eframe 0.35 では古い `update` は存在しない」）を削除し、**現在の非自明な事実だけ**を残す。例:

```rust
// `App::ui` hands us the central panel's `Ui` directly (no `CentralPanel`
// wrapper needed); obtain the `Context` via `ui.ctx()`.
```

`run.rs` 等の「なぜそうするか（zero-copy で frame を貸す等）」を説明する現在形コメントは規約準拠なので残す。

## C. `cli.rs` の `RawArgs` / `Args` 統合

### 現状

clap derive 用 private `RawArgs` と公開用 `Args` が同一フィールドを持ち、`parse()` が手動でフィールドコピーしている（DRY 違反）。

### 変更後

clap derive 構造体そのものを `pub struct Args` にし、手動コピーを廃止する。

```rust
#[derive(Parser, Debug)]
#[command(name = "bucatini", about = "Republish an NDI source as a Syphon Metal texture")]
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

pub fn parse() -> Args {
    Args::parse()
}
```

### 注意

- `#[allow(dead_code)]` は**残す**。lib 単体ビルドでは `Args` / `parse` が bin（`main.rs`）からのみ参照されるため、付与しないと dead_code 警告が出る。除去対象は重複構造体と手動コピーのみ。
- 引数のフラグ名・doc・`default_value_t`・型は現状と一致させる（CLI 解釈不変）。
- `SourceMatch` / `match_source` / `parse_selection` とそのテストは変更しない。

## 検証（全作業共通の完了条件）

1. `cargo fmt --check` パス
2. `cargo clippy --all-targets -- -D warnings` パス
3. `cargo test` パス（macOS + Syphon.framework 環境のため syphon テストも通る）
4. 挙動差分ゼロ: GUI 描画順・リサイズ・トレイ、CLI 引数解釈が現状と一致

## 作業の独立性

A / B / C は対象ファイルが分かれており（A・B: `gui.rs`、C: `cli.rs`）相互依存しない。ただし A と B は同一ファイル `gui.rs` を編集するため、**A と B は同一作業単位**として扱い、C は独立作業単位として並行可能。
