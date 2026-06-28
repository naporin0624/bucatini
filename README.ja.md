# Bucatini

[English README](README.md)

NDI 映像ソースを受信し、その映像を **GPU 共有テクスチャ** として再配信する
クロスプラットフォーム対応の CLI + GUI ツールです。出力は macOS では **Syphon**
（Metal）、Windows では **Spout** を使い、NDI 映像を Resolume・OBS などへ
受信側で CPU コピーを挟まずに橋渡しします。macOS / Syphon は実機検証済みで、
Windows / Spout は現状 CI でのコンパイル確認のみです（「対応範囲」を参照）。

## 仕組み

NDI はネットワーク経由で **CPU メモリ上の映像フレーム** を運び、Syphon は
**GPU 上の Metal テクスチャ** をプロセス間で共有します。本ツールは受信フレームを
BGRA で受け取り、IOSurface 経由で Metal テクスチャ化して `SyphonMetalServer` で
公開します（色変換シェーダ不要）。

```
NDI 送信元 ──(ネットワーク)──▶ bucatini ──(Syphon / Metal)──▶ Resolume / OBS / Syphon Recorder
```

## インストール（ビルド済み）

[Releases](../../releases) からインストーラを入手できます。

- **macOS** — `Bucatini-<version>-macos-universal.dmg`。開いて `Bucatini.app` を
  Applications にドラッグします。**未署名**ビルドのため、初回は `Bucatini.app` を
  右クリック →「**開く**」→「**開く**」で起動してください（それでもブロックされる
  場合は dmg 内の *READ ME FIRST.txt* を参照）。
- **Windows** — `Bucatini-<version>-windows-x64-setup.exe`。実行すると GUI と CLI が
  インストールされ、スタートメニューにショートカットが作成されます。未署名のため
  SmartScreen が出たら「**詳細情報 → 実行**」で進めてください。

インストールを好まない場合のために `.tar.gz` / `.zip` アーカイブも添付しています。
いずれの場合も **NDI ランタイムは別途必要**です（下記参照）。

## 前提条件

### 実行に必要（配布バイナリを使う場合も含む）

- **NDI ランタイム** — 本アプリは実行時に NDI ランタイムライブラリをロードするため、
  配布バイナリを使う場合でもインストールが必要です。
  - **macOS**: `brew install libndi`（`/usr/local/lib/libndi.dylib` を提供）。
    または [NDI Tools](https://ndi.video/tools/) for macOS をインストール。
  - **Windows**: [NDI Tools](https://ndi.video/tools/)（または NDI Runtime 単体）を
    インストール。`Processing.NDI.Lib.x64.dll` が含まれます。本アプリは DLL を遅延
    ロードし、ランタイムインストーラが設定する環境変数 `NDI_RUNTIME_DIR_Vx`
    （例 `NDI_RUNTIME_DIR_V6`）から DLL を実行時に解決します。そのため **DLL を exe
    の隣に置いたり PATH に追加したりする必要はありません**。NDI Tools が未インストール
    だと起動時に「`Processing.NDI.Lib.x64.dll` が見つからない」エラーになります。

### ビルドに必要（macOS）

- **フル版の Xcode**（Command Line Tools だけでは不可）。`xcrun` が `MacOSX.sdk` を
  見つけられない場合:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  ```
- **Metal Toolchain**（Syphon の Metal シェーダのコンパイルに必要）。framework の
  ビルドが `cannot execute tool 'metal'` で失敗する場合、一度だけ実行:
  ```bash
  xcodebuild -downloadComponent MetalToolchain   # 約 688 MB のダウンロード
  ```
- Rust（stable）。

## ビルド（macOS）

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer   # 必要な場合のみ
xcodebuild -downloadComponent MetalToolchain                            # 'metal' ツールが無い場合のみ、一度だけ
./scripts/setup-syphon.sh   # vendor/Syphon.framework をビルド（初回のみ）
cargo build --release
```

`scripts/setup-syphon.sh` は git submodule（`vendor/syphon-src` →
Syphon-Framework）を取得し、`xcodebuild` で `vendor/Syphon.framework` をビルドします。

## 開発

lint / test のゲートは [rusty-hook](https://github.com/swellaby/rusty-hook)
（Rust 製の git-hook インストーラ。Node 不要）で担保しています。フックは CI と
同じコマンドを実行します。

| フック | コマンド |
|---|---|
| `pre-commit` | `cargo fmt --all -- --check` && `cargo clippy --all-targets --features gui -- -D warnings` |
| `pre-push` | `cargo test --features gui` |

フックは dev-dependencies の初回ビルド時に自動でインストールされます。クローン後は
一度ビルドして有効化してください。

```bash
cargo build --features gui   # rusty-hook が git フックを設置
```

## 使い方

```bash
bucatini --list                          # 検出された NDI ソース一覧を表示して終了
bucatini --source "STUDIO (Camera 1)"    # 名前（部分一致）でソースを選んで配信
bucatini                                 # 一覧から番号で対話的に選択
bucatini --source Cam --name "MyFeed"    # Syphon 公開名を指定（既定: ソース名）
```

### オプション

| オプション | 説明 |
|---|---|
| `--list` | 検出された NDI ソースを一覧表示して終了 |
| `--source <名前>` | NDI ソース名（大文字小文字を無視した部分一致）。未指定かつ非 `--list` 時は対話選択 |
| `--name <名前>` | Syphon の公開名（既定: 選択した NDI ソース名） |
| `--timeout <ms>` | 検出・キャプチャのタイムアウト（ミリ秒、既定 5000） |
| `--verbose` | 受信解像度などのログを表示 |

配信を受け取るには Syphon 対応アプリ（Resolume、Syphon Recorder、OBS の Syphon
プラグインなど）を開いてください。停止は **Ctrl-C** です。

## GUI（ランチャー）

CLI と同じ動作を画面から操作できる、最小構成の GUI ランチャー `bucatini-gui` も
同梱しています（[egui](https://github.com/emilk/egui) 製）。ソースをドロップダウンから
選んで **Start / Stop** するだけです（公開名は選択したソース名になります）。

> GUI を初めてビルドする前に、同梱フォント（LINE Seed JP）を取得してください:
> ```bash
> ./scripts/fetch-fonts.sh   # vendor/fonts/ に LINE Seed JP を取得（初回のみ）
> ```

```bash
# GUI は `gui` フィーチャの別バイナリ（CLI ビルドには含まれません）
cargo run --release --features gui --bin bucatini-gui
# またはビルドだけして実行
cargo build --release --features gui --bin bucatini-gui
./target/release/bucatini-gui
```

操作:

- **Source** — 検出された NDI ソースをドロップダウンから選択（**Refresh** で再検索）。
- **Start / Stop** — 再配信の開始・停止。実行中は受信フレーム数がライブ表示されます。

ソース検索も受信ループもワーカースレッドで動くため、UI は固まりません。macOS では
CLI と同じビルド前提条件（Xcode・Metal Toolchain・`vendor/Syphon.framework`）が
必要です。

ウィンドウの **×** はアプリを終了せず、トレイ（macOS=メニューバー /
Windows=通知領域）に格納します。格納中も再配信は継続します。トレイのアイコン／
ステータス項目をクリックすると復帰、**Quit** で終了します。（**Cmd+Q**／Windows は
**Ctrl+Q** でも終了します）。テーマはローカルの `cannelloni` を参考にしたダーク配色です。

## Windows / Spout（実験的・未検証）

Windows では Spout 出力に対応します（`SharedTextureOutput` trait による抽象化で、
macOS=Syphon / Windows=Spout を切り替え）。

> ⚠️ **注意:** Windows/Spout バックエンドは現状 **GitHub Actions（windows-latest）で
> のコンパイル検証のみ**で、実機での動作確認は未実施です。色順・上下反転・SpoutDX
> 初期化まわりは実機検証で調整が必要な可能性があります。

### 実行（配布バイナリを使う場合も含む）

[NDI Tools](https://ndi.video/tools/) をインストールして NDI ランタイムを用意して
ください（上記「前提条件 → 実行に必要」を参照）。受信は Spout 対応アプリ（Resolume、
OBS の Spout プラグインなど）で行います。

### ビルド手順（Windows / PowerShell）

```powershell
./scripts/fetch-spout2.ps1            # vendor/Spout2 へ Spout2 SDK を取得
./scripts/install-ndi-sdk.ps1         # NDI 6 SDK をサイレントインストール（Processing.NDI.Lib.x64.lib を提供）
#   インストール先が標準と異なる場合は環境変数 NDI_SDK_DIR を設定
cargo build --release
```

NDI のインポートライブラリは `%NDI_SDK_DIR%\Lib\x64\Processing.NDI.Lib.x64.lib`
（既定 `C:\Program Files\NDI\NDI 6 SDK`）を参照します。ビルドには NDI **SDK** が、
実行には NDI **Runtime** が必要です（SDK にはランタイムも同梱されます）。

## 対応範囲

- **macOS / Syphon** — 実機検証済み（v1）。
- **Windows / Spout** — コンパイル検証のみ（実機動作は未検証）。

## ライセンス / 第三者ソフトウェア

本ツールは Syphon Framework（BSD）を同梱ビルドし、NDI ランタイム（libndi、別途
インストール）にリンクします。詳細は [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) を
参照してください。NDI® は Vizrt NV の登録商標です。
