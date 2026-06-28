# Bucatini

[日本語版 README](README.ja.md)

A cross-platform CLI + GUI that receives an NDI video source and republishes it
as a **GPU shared texture** — **Syphon** (Metal) on macOS and **Spout** on
Windows. It bridges NDI video into apps such as Resolume and OBS without a CPU
copy on the consuming side. macOS/Syphon is verified on real hardware; the
Windows/Spout backend is currently compile-verified in CI only (see *Scope*).

## How it works

NDI carries **video frames in CPU memory** over the network, while Syphon shares
**Metal textures on the GPU** between processes. This tool receives frames as
BGRA, wraps them into a Metal texture via IOSurface, and publishes them with
`SyphonMetalServer` (no color-conversion shader needed).

```
NDI source ──(network)──▶ bucatini ──(Syphon / Metal)──▶ Resolume / OBS / Syphon Recorder
```

## Install (prebuilt)

Grab an installer from the [Releases](../../releases) page:

- **macOS** — `Bucatini-<version>-macos-universal.dmg`. Open it and drag
  `Bucatini.app` into Applications. The build is **unsigned**, so on first launch
  right-click `Bucatini.app` → **Open** → **Open** (see *READ ME FIRST.txt* in the
  dmg if macOS still blocks it).
- **Windows** — `Bucatini-<version>-windows-x64-setup.exe`. Run it to install the
  GUI + CLI and create Start Menu shortcuts. The build is unsigned, so dismiss the
  SmartScreen prompt with **More info → Run anyway**.

The plain `.tar.gz` / `.zip` archives are also attached for users who prefer not
to install. Either way, the **NDI Runtime is still required** (see below).

## Prerequisites

### Runtime — required to *run* (including the prebuilt release binaries)

- **NDI Runtime** — the app loads the NDI runtime library at run time, so it
  must be installed even if you only use the released binaries.
  - **macOS**: `brew install libndi` (provides `/usr/local/lib/libndi.dylib`),
    or install [NDI Tools](https://ndi.video/tools/) for macOS.
  - **Windows**: install [NDI Tools](https://ndi.video/tools/) (or the
    standalone NDI Runtime); it ships `Processing.NDI.Lib.x64.dll`. The app
    delay-loads that DLL and resolves it at run time from the
    `NDI_RUNTIME_DIR_Vx` environment variable the runtime installer sets, so you
    do **not** need to put the DLL next to the exe or add it to PATH. Without the
    NDI runtime installed, launching fails with a
    "`Processing.NDI.Lib.x64.dll` not found" error.

### Build — macOS

- **Full Xcode** (not just Command Line Tools). If `xcrun` cannot find
  `MacOSX.sdk`:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  ```
- **Metal Toolchain** (needed to compile Syphon's Metal shaders). If the
  framework build fails with `cannot execute tool 'metal'`, run once:
  ```bash
  xcodebuild -downloadComponent MetalToolchain   # ~688 MB download
  ```
- Rust (stable).

## Build (macOS)

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer   # if needed
xcodebuild -downloadComponent MetalToolchain                            # once, if 'metal' tool missing
./scripts/setup-syphon.sh   # builds vendor/Syphon.framework (once)
cargo build --release
```

`scripts/setup-syphon.sh` fetches the git submodule (`vendor/syphon-src` →
Syphon-Framework) and builds `vendor/Syphon.framework` with `xcodebuild`.

## Development

Quality gates run via [rusty-hook](https://github.com/swellaby/rusty-hook)
(a Rust-native git-hook installer — no Node required). The hooks share the same
commands as CI:

| Hook | Command |
|---|---|
| `pre-commit` | `cargo fmt --all -- --check` && `cargo clippy --all-targets --features gui -- -D warnings` |
| `pre-push` | `cargo test --features gui` |

The hooks are installed automatically the first time the dev-dependencies are
built, so after cloning run a build once to activate them:

```bash
cargo build --features gui   # installs the git hooks via rusty-hook
```

## Usage

```bash
bucatini --list                          # list discovered NDI sources and exit
bucatini --source "STUDIO (Camera 1)"    # publish a source by name (substring match)
bucatini                                 # interactively pick a source by number
bucatini --source Cam --name "MyFeed"    # custom Syphon server name (default: source name)
```

### Options

| Option | Description |
|---|---|
| `--list` | List discovered NDI sources and exit |
| `--source <name>` | NDI source name (case-insensitive substring). If omitted (and not `--list`), pick interactively |
| `--name <name>` | Syphon server name (default: the selected NDI source name) |
| `--timeout <ms>` | Discovery / capture timeout in milliseconds (default 5000) |
| `--verbose` | Log received resolution, fps, etc. |

Open a Syphon-capable app (Resolume, Syphon Recorder, OBS with the Syphon
plugin, …) to receive the feed. Stop with **Ctrl-C**.

## GUI (launcher)

A minimal GUI launcher `bucatini-gui` (built with
[egui](https://github.com/emilk/egui)) ships alongside the CLI and exposes the
same flow on screen: pick a source from a dropdown and **Start / Stop** (the
server name is the selected source's name).

> Before building the GUI for the first time, fetch the bundled font (LINE Seed JP):
> ```bash
> ./scripts/fetch-fonts.sh   # downloads LINE Seed JP into vendor/fonts/ (once)
> ```

```bash
# The GUI is a separate binary behind the `gui` feature (not part of the CLI build)
cargo run --release --features gui --bin bucatini-gui
# or build then run
cargo build --release --features gui --bin bucatini-gui
./target/release/bucatini-gui
```

- **Source** — pick a discovered NDI source from the dropdown (**Refresh** to re-scan).
- **Start / Stop** — begin/end republishing; the received frame count updates live.

Discovery and the receive loop both run on worker threads, so the UI never
freezes. On macOS the GUI needs the same build prerequisites as the CLI (Xcode,
Metal Toolchain, `vendor/Syphon.framework`).

The window **✕** does not quit the app — it hides to the tray (macOS menu bar /
Windows notification area) and republishing keeps running. Click the tray icon
or its status item to restore the window; **Quit** exits. **Cmd+Q** (macOS) /
**Ctrl+Q** (Windows/Linux) also quits. The theme is a dark palette referencing
the local `cannelloni` project.

## Windows / Spout (experimental, unverified)

On Windows the output goes to Spout (abstracted behind a `SharedTextureOutput`
trait that selects Syphon on macOS / Spout on Windows).

> ⚠️ **Note:** the Windows/Spout backend is currently **compile-verified in CI
> (windows-latest) only** and has not been tested on real hardware. Color order,
> vertical flip, and SpoutDX initialization may need adjustment once tested on a
> real Windows host.

### To run (including the release binaries)

Install [NDI Tools](https://ndi.video/tools/) so the NDI runtime is present (see
[Prerequisites → Runtime](#runtime--required-to-run-including-the-prebuilt-release-binaries)).
Receive the feed in a Spout-capable app (Resolume, OBS with the Spout plugin, …).

### Build (Windows / PowerShell)

```powershell
./scripts/fetch-spout2.ps1            # fetch the Spout2 SDK into vendor/Spout2
./scripts/install-ndi-sdk.ps1         # silently install the NDI 6 SDK (provides Processing.NDI.Lib.x64.lib)
#   set NDI_SDK_DIR if you installed the SDK somewhere non-standard
cargo build --release
```

The NDI import library is taken from
`%NDI_SDK_DIR%\Lib\x64\Processing.NDI.Lib.x64.lib` (default
`C:\Program Files\NDI\NDI 6 SDK`). Building needs the NDI **SDK**; running needs
the NDI **Runtime** (the SDK bundles the runtime too).

## Scope

- **macOS / Syphon** — verified on real hardware (v1), CLI and GUI.
- **Windows / Spout** — compile-verified in CI only (no real-hardware test yet).

## License / third-party software

This tool builds and bundles the Syphon Framework (BSD) and links the NDI
runtime (libndi, installed separately). See [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES)
for details. NDI® is a registered trademark of Vizrt NV.
