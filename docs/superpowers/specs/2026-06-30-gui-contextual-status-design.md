# GUI contextual status + WCAG AA tokens — design

- Date: 2026-06-30
- Scope: `src/bin/gui.rs` のステータス表示を文脈的・非機械的に再設計し、color token を WCAG 2.1 AA 準拠にする
- Branch: `feat/gui-contextual-status`（`refactor/src-claude-md-compliance` の上にスタック。PR #12 の 9 メソッド構成を前提）
- Out of scope: ロジック/FFI/CLI、ウィンドウ枠・フォント・テーマ全体の作り替え

## 背景

CLAUDE.md の `## ui rules`（WCAG 2.1 AA / 文脈に沿った非機械的 UI）に照らし、現状 GUI に 2 つの逸脱がある。

1. **WCAG 違反**: idle ステータス「○ Idle」の文字色 `border` `#696969` は canvas `#121212` 上で **3.41:1** しかなく、本文 AA 基準 4.5:1 未満。装飾境界 `border_subtle` `#424242` も 1.86:1 で UI 基準 3:1 未満。
2. **機械的 UI**: running 表示が `info: 3 frames・NAME・Syphon` とデバッグ文字列で、本来一番伝えたい「下流アプリが探す公開サーバ名」が区切り文字に埋もれている。

実測コントラスト（canvas `#121212` 比）:

| token | 比 | 判定 |
|---|---|---|
| text `#EEEFF2` | 16.29 | OK |
| text_muted `#C9CCD2` | 11.64 | OK |
| accent `#3996FF` | 6.23 | OK |
| border `#696969` | 3.41 (surface 3.10) | UI OK / 本文 NG |
| border_subtle `#424242` | 1.86 | NG |
| border_strong `#868686` | 5.15 | OK |

## 決定事項（ユーザー確定済み）

- 表示パターン: **B（ラベル付きインライン、カードなし）**
- コピー言語: **全面英語のまま構造のみ改善**（idle→Ready / running→Live、`info:` 接頭辞撤廃）
- WCAG: **文字 + 境界線まで**修正

## 1. WCAG color token 変更

`install_style` 内のトークンを次のとおり変更する。値は WCAG 計算で検証済み。

| token | 変更 | 検証 |
|---|---|---|
| `text_dim`（**新規**） | `#9E9E9E` | canvas 6.99:1（本文 4.5 クリア） |
| `border_subtle` | `#424242` → `#696969` | surface 3.10:1（UI 3:1 クリア） |
| accent / text / text_muted / border / border_strong | 据え置き | 既に AA 合格 |

注: `#696969` より暗いグレーでは surface `#1C1C1C` 上で 3:1 を満たせないため、`border_subtle` は `border` と同階層へ統一する（階層反転を避ける唯一の整合解）。装飾境界が `border` と同程度に見えるようになるが、これは AA のための意図的変化。

`text_dim` は egui の Visuals に直接対応するスロットがないため、**`draw_status` 内のローカル定数**として持ち、idle 系の `colored_label` の色に使う（テーマ全体の `widgets.*` には触れない）。

## 2. ステータス表示の再設計（パターン B / 英語）

現状の「`● Running`/`○ Idle` の colored_label」+「`info:` ラベル」の二重表示を、1 つの文脈ブロックへ統合する。

### 状態と表示

| 状態 | 1 行目 | 2 行目 |
|---|---|---|
| live（実行中） | `● Live · {frames} frames`（`●`=accent、文字=text `#EEEFF2`） | `Publishing as {kind} "{name}"`（text_muted、wrap） |
| ready（idle・ソースあり・status 空） | `○ Ready`（`○`+文字=text_dim） | — |
| searching（discovery 中） | `○ Searching…`（text_dim） | — |
| no source（ソース 0 件） | `○ No NDI sources found`（text_dim） | — |
| stopped（停止後） | `○ Stopped · {frames} frames`（text_dim） | — |
| error（worker 異常終了） | `○ Error: {msg}`（text_dim、wrap） | — |

- `{frames}` は 3 桁区切り（`1,240`）。新ヘルパ `group_thousands(n: u64) -> String`。
- `{kind}` = `bucatini::output::output_kind()`（"Syphon"/"Spout"）。
- `{name}` = GUI が `make_output` に渡す公開名（現状は選択ソース名と同一）。
- live 2 行目のみ `Label::new(...).wrap()`、error も wrap。他は 1 行。

### 既存メソッドへの対応

PR #12 で分割済みの構成を維持し、本文のみ変更する。

- `draw_status(&self, ui)`: 上表の分岐で描画。`text_dim` ローカル定数をここに置く。
- `info_line(&self) -> String`: 現行の多目的文字列をやめ、**トレイ用 1 行サマリ**を返す責務に限定（`sync_tray_status` から使用）。live は `Live · {frames} frames · {kind} "{name}"`、他は上表 1 行目のラベル文言（記号 `●/○` は除く）。
- `sync_tray_status(&self)`: `format!("Bucatini — {}", self.info_line())`。改行置換は不要になる（`info_line` が単一行を返すため）。
- 新規 `group_thousands`: ファイル内の自由関数（`GuiApp` に依存しない純関数）。

### レイアウトシフト

live は 2 行、その他は 1 行。idle→live で 1 行分高くなるが、既存の `fit_window`（`InnerSize` 高さ自動追従）が吸収する。これは「情報が増えたら窓が伸びる」自然な挙動で許容する。

## 3. 影響範囲

`src/bin/gui.rs` のみ。`install_style`（トークン 2 箇所）、`draw_status` / `info_line` / `sync_tray_status`（本文）、`group_thousands`（追加）。他ファイル・ロジック・FFI・CLI 不変。

## テスト

- `group_thousands` は純関数なので**ユニットテストを追加**（`0→"0"`, `42→"42"`, `1000→"1,000"`, `1240→"1,240"`, `1234567→"1,234,567"`）。`gui.rs` は bin だが `#[cfg(test)] mod tests` を置ける。
- それ以外（描画）は GUI 自動テスト不可のため、`cargo clippy`/build + 静的レビューで挙動確認。
- 完了ゲート: `cargo fmt --all -- --check` / `cargo clippy --all-targets --features gui -- -D warnings` / `cargo test --lib`（18 維持）/ `cargo test --features gui --bin bucatini-gui`（group_thousands テスト）/ `cargo build --features gui --bin bucatini-gui`。

## 検証用 WCAG 値（参考・実測済み）

- `text_dim #9E9E9E`: canvas 6.99:1
- `border_subtle #696969`: canvas 3.41 / surface 3.10
- `accent #3996FF`（live ドット）: 6.23:1
