# Syphon 出力パス最適化タスク（macOS）

Spout（Windows）で完了した「CPU計測 → 調査 → 最適化 → 実映像で色検証」を Syphon（macOS/Metal）にも適用する。**Windows では実測・ビルド不可**、macOS 実機で実行する前提。Spout 側の成果物（`bench_spout_cpu.rs` / `spout_recv_bridge.*` / `verify_spout_colors.rs` / `spout_bridge.*`）が雛形。

現状の無駄: `vendor/cpp/syphon_bridge.mm::syphon_bridge_send_rgba` は**毎フレーム `IOSurfaceCreate` と `newTextureWithDescriptor:iosurface:` を確保**している。→ リサイズ時のみ生成して再利用するのが主目的。

## 0. 前提・環境セットアップ
- [ ] macOS 実機（Metal 対応 GPU）で作業する
- [ ] NDI SDK（macOS版）を導入（`libndi` が `/usr/local/lib` or `/opt/homebrew/lib`）
- [ ] `Syphon.framework` が `vendor/` にある（`build.rs::build_macos_syphon`）
- [ ] `cargo build` / `cargo test` がベースラインで通る
- [ ] ref repo の Syphon 送信実装を確認: https://github.com/naporin0624/electron-texture-bridge

## 1. ベースライン CPU ベンチ（現状計測）
- [ ] `examples/bench_syphon_cpu.rs` を作成（非macOSはスタブ）
- [ ] 合成 BGRA フレーム（1080p / 4K, `stride=w*4`）を生成
- [ ] `SyphonOutput::new` → `publish` を warmup 後 N フレーム計測
- [ ] thread-CPU 時間を `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` で採る（Windows `GetThreadTimes` の代替）、wall は `Instant`
- [ ] `src/bench_metrics.rs`（`frame_bytes`/`per_frame_micros`/`throughput_mb_per_s`/`cpu_percent_at_fps`）を**そのまま再利用**して出力
- [ ] 1080p / 4K の現状値を記録（最適化前ベースライン）

## 2. 送信パス最適化（複数案 → 実測 → 勝者採用）
- [ ] `vendor/cpp/syphon_bridge.{mm,h}` に `syphon_bridge_send_rgba_mode(..., mode)` を追加
- [ ] mode 0 = 現状（毎フレーム IOSurface/MTLTexture 生成 + 行memcpy）※比較用に残す
- [ ] mode 1 = **永続 IOSurface + MTLTexture 再利用**（`ensure_surface(w,h)` でリサイズ時のみ再生成）+ 行memcpy
- [ ] mode 2 = mode 1 + **pitch一致時（`bytes_per_row == IOSurfaceGetBytesPerRow`）は単発 bulk memcpy**
- [ ] `SyphonBridge` 構造体に `IOSurfaceRef / id<MTLTexture> / width / height` をキャッシュ
- [ ] `src/output/syphon.rs` に `SendMode` + `publish_with_mode` を追加（`as_u32` は TDD で test→red→green）
- [ ] `bench_syphon_cpu.rs` を3-way（mode 0/1/2）化し 1080p/4K で `speedup_ratio` 付き比較表を出力
- [ ] 勝者を `syphon_bridge_send_rgba`（`publish` が呼ぶ既定）に配線 → 再ビルド・再ベンチで一致確認

## 3. 実映像での色検証（Syphon 受信ハーネス）
- [ ] `vendor/cpp/syphon_recv_bridge.{mm,h}` を作成
- [ ] `SyphonServerDirectory` → `SyphonMetalClient` で名前指定受信（正確なAPIは実機の `Syphon.framework` ヘッダで確定）
- [ ] `-newFrameImage`（`id<MTLTexture>`）→ 共有ストレージテクスチャへ blit → CPU BGRA へ読み戻し
- [ ] `syphon_recv_grab(handle, name, buf, max_w, max_h, *out_w, *out_h, timeout_ms)` を提供（Spout の grab 同シグネチャ）
- [ ] `src/output/syphon.rs` に `SyphonReceiver`（`new`/`grab`/`Drop`）を追加
- [ ] `examples/verify_syphon_colors.rs` を作成
- [ ] 256x256 の4象限パターン（TL=赤/TR=緑/BL=青/BR=白, BGRA厳密値）を送信
- [ ] 送信はバックグラウンドスレッドで**連続publish**、受信はメインで1回の長い grab（Spout で踏んだ「1回publish→pollは不可」の教訓）
- [ ] `src/output/mod.rs::bgra_at` で各隅を厳密比較、R/Bスワップ・上下反転を明示診断
- [ ] 受信フレームを PNG 保存（macOS は `sips` か `ffmpeg` で変換）、**失敗時は非ゼロ終了**
- [ ] ⚠ `flipped:YES` の向きを織り込む（or 検証中は `flipped:NO`）。4象限なら反転は色配置で露見

## 4. 品質チェック & 仕上げ
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`（`bench_metrics` / `SendMode::as_u32` / `bgra_at` の新規テスト含む）
- [ ] `verify_syphon_colors` が **PASS**、保存 PNG を目視確認
- [ ] husky（lint/test 担保）が通る
- [ ] 最適化前/後の CPU 値（1080p/4K）・勝者・改善率を記録

## メモ
- [ ] 本質コストは「CPUバッファ→サーフェスへの memcpy 1回」。Syphon の主無駄は**毎フレーム確保**（Spout で確認済の原則）
- [ ] `src/bench_metrics.rs` と `src/output/mod.rs::bgra_at` は macOS でも流用可（追加不要）
- [ ] 関数は単一責任 / コメントはコードから読めない事だけ（CLAUDE.md 準拠）
