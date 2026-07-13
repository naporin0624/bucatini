# Syphon 出力パス最適化タスク（macOS）

Spout（Windows）で完了した「CPU計測 → 調査 → 最適化 → 実映像で色検証」を Syphon（macOS/Metal）にも適用する。**Windows では実測・ビルド不可**、macOS 実機で実行する前提。Spout 側の成果物（`bench_spout_cpu.rs` / `spout_recv_bridge.*` / `verify_spout_colors.rs` / `spout_bridge.*`）が雛形。

現状の無駄: `vendor/cpp/syphon_bridge.mm::syphon_bridge_send_rgba` は**毎フレーム `IOSurfaceCreate` と `newTextureWithDescriptor:iosurface:` を確保**している。→ リサイズ時のみ生成して再利用するのが主目的。

## 0. 前提・環境セットアップ
- [x] macOS 実機（Metal 対応 GPU）で作業する
- [x] NDI SDK（macOS版）を導入（`libndi` が `/usr/local/lib` or `/opt/homebrew/lib`）
- [x] `Syphon.framework` が `vendor/` にある（`build.rs::build_macos_syphon`）
- [x] `cargo build` / `cargo test` がベースラインで通る
- [x] ref repo の Syphon 送信実装を確認: https://github.com/naporin0624/electron-texture-bridge

## 1. ベースライン CPU ベンチ（現状計測）
- [x] `examples/bench_syphon_cpu.rs` を作成（非macOSはスタブ）
- [x] 合成 BGRA フレーム（1080p / 4K, `stride=w*4`）を生成
- [x] `SyphonOutput::new` → `publish` を warmup 後 N フレーム計測
- [x] thread-CPU 時間を `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` で採る（Windows `GetThreadTimes` の代替）、wall は `Instant`
- [x] `src/bench_metrics.rs`（`frame_bytes`/`per_frame_micros`/`throughput_mb_per_s`/`cpu_percent_at_fps`）を**そのまま再利用**して出力
- [x] 1080p / 4K の現状値を記録（最適化前ベースライン）

## 2. 送信パス最適化（複数案 → 実測 → 勝者採用）
- [x] `vendor/cpp/syphon_bridge.{mm,h}` に `syphon_bridge_send_rgba_mode(..., mode)` を追加
- [x] mode 0 = 現状（毎フレーム IOSurface/MTLTexture 生成 + 行memcpy）※比較用に残す
- [x] mode 1 = **永続 IOSurface + MTLTexture 再利用**（`ensure_surface(w,h)` でリサイズ時のみ再生成）+ 行memcpy
- [x] mode 2 = mode 1 + **pitch一致時（`bytes_per_row == IOSurfaceGetBytesPerRow`）は単発 bulk memcpy**
- [x] `SyphonBridge` 構造体に `IOSurfaceRef / id<MTLTexture> / width / height` をキャッシュ
- [x] `src/output/syphon.rs` に `SendMode` + `publish_with_mode` を追加（`as_u32` は TDD で test→red→green）
- [x] `bench_syphon_cpu.rs` を3-way（mode 0/1/2）化し 1080p/4K で `speedup_ratio` 付き比較表を出力
- [x] 勝者を `syphon_bridge_send_rgba`（`publish` が呼ぶ既定）に配線 → 再ビルド・再ベンチで一致確認

## 3. 実映像での色検証（Syphon 受信ハーネス）
- [x] `vendor/cpp/syphon_recv_bridge.{mm,h}` を作成
- [x] `SyphonServerDirectory` → `SyphonMetalClient` で名前指定受信（正確なAPIは実機の `Syphon.framework` ヘッダで確定）
- [x] `-newFrameImage`（`id<MTLTexture>`）→ 共有ストレージテクスチャへ blit → CPU BGRA へ読み戻し
- [x] `syphon_recv_grab(handle, name, buf, max_w, max_h, *out_w, *out_h, timeout_ms)` を提供（Spout の grab 同シグネチャ）
- [x] `src/output/syphon.rs` に `SyphonReceiver`（`new`/`grab`/`Drop`）を追加
- [x] `examples/verify_syphon_colors.rs` を作成
- [x] 256x256 の4象限パターン（TL=赤/TR=緑/BL=青/BR=白, BGRA厳密値）を送信
- [x] 送信はバックグラウンドスレッドで**連続publish**、受信はメインで1回の長い grab（Spout で踏んだ「1回publish→pollは不可」の教訓）
- [x] `src/output/mod.rs::bgra_at` で各隅を厳密比較、R/Bスワップ・上下反転を明示診断
- [x] 受信フレームを PNG 保存（macOS は `sips` か `ffmpeg` で変換）、**失敗時は非ゼロ終了**
- [x] ⚠ `flipped:YES` の向きを織り込む（or 検証中は `flipped:NO`）。4象限なら反転は色配置で露見

## 4. 品質チェック & 仕上げ
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`（`bench_metrics` / `SendMode::as_u32` / `bgra_at` の新規テスト含む）
- [x] `verify_syphon_colors` が **PASS**、保存 PNG を目視確認
- [x] husky（lint/test 担保）が通る
- [x] 最適化前/後の CPU 値（1080p/4K）・勝者・改善率を記録

## メモ
- [x] 本質コストは「CPUバッファ→サーフェスへの memcpy 1回」。Syphon の主無駄は**毎フレーム確保**（Spout で確認済の原則）
- [x] `src/bench_metrics.rs` と `src/output/mod.rs::bgra_at` は macOS でも流用可（追加不要）
- [x] 関数は単一責任 / コメントはコードから読めない事だけ（CLAUDE.md 準拠）

## 結果（2026-07-13 実測, macOS 26.4.1 / Metal）

### CPUベンチ（`cargo run --release --example bench_syphon_cpu`, thread-CPU µs/frame）

| 解像度 | mode 0 現状 | mode 1 永続+行copy | mode 2 永続+bulk | 勝者 | 改善率 |
|---|---|---|---|---|---|
| 1080p | 588.97 | **129.89** | 131.45 | mode 1 | **4.53x** |
| 4K    | 2274.05 | **701.44** | 724.16 | mode 1 | **3.24x** |

- 勝者 **mode 1 (PersistentRowCopy)** を `syphon_bridge_send_rgba` 既定に配線済み。`publish` 実測 131.33 / 702.55 µs で mode 1 と一致（ノイズ範囲）
- mode 2 は 1080p で mode 1 と統計的同値、4K でやや劣後（NDI フレームは密パックで bulk 化の利得なし）。ベンチ比較用に温存
- 単一 IOSurface 再利用は Syphon の契約上安全（SyphonMetalServer.h: publish は「texture is copied ... once this method has returned」）

### 色検証（`cargo run --release --example verify_syphon_colors`）

- PASS ×4、全隅 BGRA 厳密一致（bit-exact）、R/B スワップなし、ティアリングなし
- 受信 raw フレームは送信比で上下ミラー（sender `flipped:YES` の織り込み済み・診断で明示）。実コンシューマでの向きは目視確認推奨
