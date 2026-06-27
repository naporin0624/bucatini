# NDI → Syphon/Spout 変換 CLI 設計書

- 日付: 2026-06-27
- ステータス: 承認済み（実装計画へ移行）
- スコープ: v1 = macOS / Syphon（Spout/Windows は抽象スタブのみ）

## 1. 目的

ネットワーク上の NDI 映像ソースを受信し、その映像フレームを GPU 共有テクスチャ
（macOS: Syphon Metal）として再公開する単機能 CLI ツール。VJ ソフト・OBS など
Syphon 対応アプリへ NDI 映像をゼロコピー相当で届けることを狙う。

将来的に Windows / Spout 出力へ拡張できるよう、出力側は trait で抽象化する。
ただし v1 では Spout 実装本体は作らない（抽象境界とスタブのみ）。

## 2. 前提・依存物

確認済みの環境（2026-06-27 時点）:

- Rust 1.95（cargo は mise 管理）
- NDI ランタイム: `/usr/local/lib/libndi.dylib`（Homebrew `libndi`）
- clang / xcrun / cc 利用可

要対応（実装前の準備）:

- **Syphon.framework が未インストール**。`vendor/syphon-src`（`Syphon-Framework`）を
  submodule として取得しビルドして `vendor/Syphon.framework` を配置する。
  参照実装: `naporin0624/electron-texture-bridge`（`naporin0624/Syphon-Framework` fork）。
- **macOS SDK 参照の修復**: `xcrun` が `MacOSX.sdk` を解決できない状態。
  Objective-C++ シムや framework のビルド前に `xcode-select` の修復が必要。
  README の前提条件に明記する。
- NDI ヘッダ（`Processing.NDI.Lib.h`）はランタイム dylib しか無い。Rust バインディングで
  ヘッダが必要な場合は SDK ヘッダを vendor するか、ヘッダ同梱クレートを使う。

## 3. 先行実装の流用（electron-texture-bridge）

参照: `https://github.com/naporin0624/electron-texture-bridge`

送信（Syphon サーバ）側が既に実装されており、本ツールの Syphon 出力にそのまま使える。
`packages/native/cpp/mac/syphon_bridge.{mm,h}` の**送信側サブセット**を vendor する。

利用する C API:

```c
typedef void* SyphonBridgeHandle;
SyphonBridgeHandle syphon_bridge_create(const char* name);     // SyphonMetalServer 生成
void               syphon_bridge_destroy(SyphonBridgeHandle handle);
int                syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                                           const uint8_t* data,   // BGRA バイト列
                                           uint32_t width,
                                           uint32_t height,
                                           uint32_t bytes_per_row); // stride
```

`syphon_bridge_send_rgba` の内部処理（流用元で確認済み）:

1. `IOSurfaceCreate`（`kCVPixelFormatType_32BGRA`, 指定 stride）
2. ロックして **行単位で memcpy**（src stride ≠ IOSurface stride を吸収）
3. `newTextureWithDescriptor:iosurface:plane:` で Metal テクスチャ化（ゼロコピー）
4. `publishFrameTexture:onCommandBuffer:imageRegion:flipped:YES`

結論: **Rust 側に metal-rs は不要**。Rust は BGRA バイト列 + stride + 幅高を C 関数へ
渡す薄い FFI のみ。受信側・discovery など不要部分は持ち込まない。

build.rs（macOS）も流用元を踏襲:

- `cc` で `.mm` を `-ObjC++ -std=c++17 -fobjc-arc` でコンパイル
- `-F <vendor>` を付与し `Syphon.framework` を探索
- リンク: `framework=Syphon, Metal, IOSurface, Cocoa, QuartzCore` ＋ `c++`
- rpath: `@loader_path` 系 + `@executable_path/../Frameworks` + `vendor/` への相対
- 本プロジェクトは **bin クレート**なので `rustc-cdylib-link-arg` ではなく
  `rustc-link-arg` で rpath を渡す（流用元の test バイナリ向け記述に準拠）

## 4. CLI 仕様

バイナリ名: `ndi-share`（将来の Spout 対応を見越した中立名）。

```
ndi-share --list                         # 検出された NDI ソース一覧を表示して終了
ndi-share --source "MACHINE (Camera 1)"  # 名前(部分一致)で選択して実行
ndi-share                                # 未指定なら検出一覧から番号で対話選択
```

オプション:

- `--source <name>`: NDI ソース名（部分一致）。未指定かつ非 `--list` 時は対話選択。
- `--name <syphon-name>`: Syphon 公開名（既定: 選択した NDI ソース名）。
- `--list`: 検出ソース一覧を表示して終了。
- `--timeout <ms>`: 検出待ち / capture のタイムアウト（既定値を設ける）。
- `--verbose`: 受信解像度・FPS 等のログを出す。

## 5. アーキテクチャ（モジュール境界）

- `cli` — clap による引数解析・対話選択ロジック（純ロジック、テスト可能）
- `ndi` — libndi の安全ラッパ。`find`（検出）・`receiver`（BGRA 受信）。
  バインディングは保守された NDI crate を第一候補、不可なら vendor ヘッダ + bindgen。
- `output` — `trait SharedTextureOutput { fn resize(&mut self, w, h); fn publish(&mut self, frame: &BgraFrame) -> Result<()>; }`。
  将来 Spout を足すための抽象境界。
- `syphon` — `output` の macOS 実装。`extern "C"` FFI で vendor 済みシムを呼ぶ。
- shim（vendor）— `cpp/mac/syphon_bridge.{mm,h}`（送信側サブセット）。build.rs で `cc` ビルド。
- `app`（main）— 検出 → 受信 → 公開のループ配線、リサイズ対応、Ctrl-C で graceful 停止。

各ユニットは単一責務・明確なインタフェースを持ち、`cli` / `output` trait / `ndi` ラッパは
ハードウェア無しに単体テストできる粒度に保つ。

## 6. データフロー

1. `NDIlib_find` でソース検出。
   - `--list` なら一覧表示して終了。
   - `--source` 指定で部分一致選択（複数一致時は候補提示）。
   - 未指定なら一覧から番号で対話選択。
2. NDI 受信を **BGRX_BGRA** 指定で生成（YUV→RGB 変換シェーダ不要）。
3. `syphon_bridge_create(name)` で SyphonMetalServer を生成。
4. ループ: `NDIlib_recv_capture_v2`。
   - 映像フレーム受信時、`data / line_stride / xres / yres` を取得。
   - 解像度が変わっても `send_rgba` 側が IOSurface を都度生成するため Rust 側の
     テクスチャ再確保は不要（サイズはフレームごとに渡す）。
   - `syphon_bridge_send_rgba(handle, data, xres, yres, line_stride)` で公開。
   - フレームを `NDIlib_recv_free_video_v2` で解放。
5. Ctrl-C（SIGINT）で受信停止 → `syphon_bridge_destroy` → libndi 解放。

v1 は **単一スレッドのループ**（NDI capture のタイムアウトで回す）。

## 7. エラーハンドリング

- libndi 初期化失敗 → SDK / dylib 未検出を明示して終了。
- `--source` 不一致 → 候補一覧を表示して非ゼロ終了。
- 検出ゼロ（タイムアウト）→ 案内メッセージを表示。
- `send_rgba` 戻り値 < 0 → エラーログ（`--verbose` 時は詳細）、致命的でなければ継続。
- Syphon.framework ロード失敗 → 起動時に表面化（rpath / vendor 配置を案内）。

## 8. テスト

TDD 対象（純ロジック、ハードウェア不要）:

- ソース名マッチング（部分一致・複数一致・不一致）。
- CLI 引数解析（オプション組み合わせ、`--list` 排他など）。
- 対話選択の入力 → 選択インデックス変換。

手動 / 統合検証:

- NDI テスト送信（NDI Tools の Test Pattern 等）に対して `ndi-share` を実行。
- Syphon クライアント（Resolume / Syphon Recorder 等）で映像受信を目視確認。
- 手順を README に記録。

## 9. ビルド・配布

- `vendor/syphon-src`（submodule）→ Syphon.framework をビルドして `vendor/` へ。
- build.rs が `.mm` シムをコンパイルし framework をリンク・rpath 設定。
- libndi リンク: `-L/usr/local/lib -lndi`（または使用クレートのリンク設定）。
- setup スクリプトで submodule 取得 + framework ビルドを自動化。
- THIRD-PARTY-NOTICES に Syphon Framework のライセンスを記載。

## 10. スコープ外（YAGNI / v1 では作らない）

- 音声（Syphon は映像のみ）。
- Spout / Windows 実装本体（`output` trait のスタブのみ）。
- 複数 NDI ソースの同時公開。
- 本格的な TUI（対話選択は単純な番号入力のみ）。
- YUV→RGB 変換シェーダ（NDI から BGRA を要求するため不要）。
- 解像度変換 / スケーリング / フィルタ。
