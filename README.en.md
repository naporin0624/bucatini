# ndi-share

[日本語版 README](README.md)

Receive an NDI video source and republish it as a Syphon Metal texture (macOS).

## Prerequisites

- **Full Xcode** (not just Command Line Tools). If `xcrun` cannot find `MacOSX.sdk`:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  ```
- **Metal Toolchain** (needed to compile Syphon's Metal shaders). If the framework build fails with `cannot execute tool 'metal'`, run once:
  ```bash
  xcodebuild -downloadComponent MetalToolchain
  ```
- NDI runtime: `brew install libndi` (provides `/usr/local/lib/libndi.dylib`).
- Rust (stable).

## Build

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer   # if needed
xcodebuild -downloadComponent MetalToolchain                            # once, if 'metal' tool missing
./scripts/setup-syphon.sh   # builds vendor/Syphon.framework (once)
cargo build --release
```

## Usage

```bash
ndi-share --list                          # list NDI sources
ndi-share --source "STUDIO (Camera 1)"    # publish a source by name (substring)
ndi-share                                 # interactively pick a source
ndi-share --source Cam --name "MyFeed"    # custom Syphon server name
```

Open any Syphon client (Resolume, Syphon Recorder, OBS with Syphon plugin) to
receive the texture. Stop with Ctrl-C.

## GUI (launcher)

A minimal GUI launcher `ndi-share-gui` (built with [egui](https://github.com/emilk/egui))
ships alongside the CLI and exposes the same flow on screen: pick a source from a
dropdown, set the server name, and **Start / Stop**.

> Before building the GUI for the first time, fetch the bundled font (LINE Seed JP):
> ```bash
> ./scripts/fetch-fonts.sh   # downloads LINE Seed JP into vendor/fonts/ (once)
> ```

```bash
# The GUI is a separate binary behind the `gui` feature (not part of the CLI build)
cargo run --release --features gui --bin ndi-share-gui
# or build then run
cargo build --release --features gui --bin ndi-share-gui
./target/release/ndi-share-gui
```

- **Source** — pick a discovered NDI source from the dropdown (**Refresh** to re-scan).
- **Name** — Syphon server name (defaults to the source name; once you edit it by
  hand it no longer auto-follows the selection).
- **Start / Stop** — begin/end republishing; the received frame count updates live.

Discovery and the receive loop both run on worker threads, so the UI never freezes.
On macOS the GUI needs the same prerequisites as the CLI (Xcode, Metal Toolchain,
`vendor/Syphon.framework`).

The window **✕** does not quit the app — it hides to the tray (macOS menu bar /
Windows notification area) and republishing keeps running. Click the tray icon or
its status item to restore the window; **Quit** exits. The theme is a dark palette
referencing the local `cannelloni` project.

## Scope

- **macOS / Syphon** — verified on real hardware (v1), CLI and GUI.
- **Windows / Spout** — compile-verified in CI only (no real-hardware test yet).
