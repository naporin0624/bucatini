# ndi-share-gui 設計（最小ランチャー GUI）

- 日付: 2026-06-28
- 対象: `ndi-share`（NDI → Syphon/Spout 再配信 CLI）への GUI 追加
- ステータス: 設計確定（実装計画前）

## 目的

既存 CLI の操作（ソースを探す → 選ぶ → 再配信 → 停止）を、そのまま画面化した
**最小ランチャー GUI** を追加する。プレビューや常駐機能は持たない。

スコープ判断:

- スコープ: ランチャー最小構成（プレビューなし）
- スタック: egui / eframe（純 Rust 即時モード GUI）
- バイナリ構成: CLI とは別バイナリ（`ndi-share-gui`）

## 非目標（YAGNI）

- 受信映像の GUI 内プレビュー（サムネ/ライブ）
- メニューバー/タスクトレイ常駐・自動再接続・設定永続化
- 複数ソースの同時再配信
- 既存 CLI の挙動・既存テストの変更

## クレート構成

現状は全モジュールが bin (`main.rs`) からのみ参照されている。これを lib 化し、
CLI と GUI の両バイナリから同じコアを共有する。

```
src/
  lib.rs          ← 新規。ndi / output / cli を pub 再公開（コア API）
  main.rs         ← CLI バイナリ（既存。lib を使う形に薄く変更）
  bin/gui.rs      ← 新規。egui アプリ（ndi-share-gui）
  ndi/            ← 既存のまま
  output/         ← 既存のまま
  cli.rs          ← 既存のまま
```

Cargo.toml:

- `[lib]` を追加（crate 名 `ndi_share`）。
- `[[bin]] name = "ndi-share-gui"` / `path = "src/bin/gui.rs"` を追加。
- `eframe`（egui 同梱）を **feature `gui`** に隠す。
  - 既定 feature には含めない。`cargo build --bin ndi-share` は GUI 依存なしで軽量。
  - GUI ビルドは `cargo build --bin ndi-share-gui --features gui`。
  - `src/bin/gui.rs` 先頭で `#![cfg(feature = "gui")]` 相当のガード（feature 無効時は空 main）にし、
    feature 無しでもワークスペースのビルドが壊れないようにする。
- macOS=Syphon / Windows=Spout の出力切替は既存の `cfg` 分岐と `output_kind()` を流用。

## スレッド／状態モデル

UI スレッドを固めないため、ブロッキング処理（ソース探索・受信ループ）は
すべてワーカースレッドへ逃がす。

FFI オブジェクト（`Ndi` / `Receiver` / Syphon・Spout 出力）はスレッドをまたいで
送らない。ワーカー側で生成して使い切る方針とし、UI からは plain data
（`Source` のクローン、サーバー名 `String`）だけを渡す。

```rust
enum AppState {
    Idle,                              // 起動直後・停止後
    Discovering,                       // ソース探索中（探索ワーカー実行中）
    Running { shared: Arc<RunState> }, // 再配信中
}

struct RunState {                      // UI ⇄ 受信ワーカーの共有状態
    stop:   AtomicBool,                // UI が立てる → ワーカーが抜ける
    frames: AtomicU64,                 // ワーカーが increment → UI が表示
    error:  Mutex<Option<String>>,     // 異常終了時にワーカーが書く
}
```

### データフロー

- 探索: 起動時とリフレッシュ時に `Finder::list(timeout_ms)` をワーカーで実行し、
  結果 `Vec<Source>` を `mpsc::Sender<DiscoverMsg>` で UI に返す。
  UI は受信して `AppState::Idle` のソース一覧を更新。
- 開始: 選択中 `Source`（クローン）とサーバー名を渡してワーカー起動。
  ワーカーが `Ndi::new` → `Receiver::new` → 出力生成を行い、`stop` が立つまで受信ループ。
  毎フレーム `frames` を increment。
- 停止: UI が `stop = true` をセット → ワーカーを join → `AppState::Idle`。
- UI は毎フレーム `frames` / `error` を読んで表示するだけ。

### 受信ループ

既存 `run_loop`（Ctrl-C 専用）は CLI 用にそのまま残す。
GUI 用に、停止フラグ `&AtomicBool` とフレームカウンタを受け取る薄い関数を
コアに追加する（例: `run_loop_until(receiver, out, stop, frames) -> Result<()>`）。
CLI の `run_loop` はこの関数に Ctrl-C のフラグを渡す薄いラッパへ寄せてもよいが、
既存テストを壊さない範囲に留める。

## 画面（1ウィンドウ・縦並び）

```
┌─ NDI → Syphon/Spout ─────────────────┐
│ Source:  [ Camera 1 (HOST)      ▼ ]  │
│          [ 🔄 Refresh ]               │
│ Name:    [ Camera 1____________ ]     │
│                                       │
│ [ ▶ Start ]   ● Running  1234 frames  │
│                                       │
│ status: Publishing as Syphon …        │
└───────────────────────────────────────┘
```

挙動:

- ソース無し: ドロップダウン無効 ＋「No NDI sources（Refresh を押す）」表示。
- ソース選択を変えたら Name 欄の既定値（= ソース名）を追従更新。
  ただしユーザーが Name を手動編集済みなら上書きしない。
- Running 中: Source / Name / Refresh を無効化。ボタンは `■ Stop`。
- 出力種別表示は `output_kind()` を流用（macOS=Syphon / Windows=Spout）。
- ウィンドウは固定小サイズ（およそ 420×220）。リサイズ可。

## エラー処理

- 探索失敗・生成失敗（NDI 初期化失敗、ソース消失など）は `RunState.error`
  または mpsc 経由で UI に伝え、状態を `Idle` に戻す。**パニックでアプリを落とさない。**
- ワーカーが受信ループ中にエラー → `error` に格納して終了 → UI が次フレームで検知し、
  赤いステータス行を表示して `Start` に戻す。
- すべての `unwrap`/`expect` は GUI コードでは避け、エラーは UI に表示する。

## テスト方針

- コアのロジック（`run_loop_until` の停止挙動など）は既存テスト同様に単体テスト可能な形にする。
  停止フラグを立てたらループが抜けること、フレームカウンタが進むことをモック出力で検証。
- GUI 描画そのものは自動テスト対象外（手動確認）。
- 受け入れ確認: `cargo build --bin ndi-share`（GUI 依存なし）と
  `cargo build --bin ndi-share-gui --features gui` の両方が通ること、
  既存 `cargo test` が緑のままであること。
