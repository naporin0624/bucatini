# ndi-share

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

## Scope

v1 is macOS/Syphon only. Spout/Windows is not yet implemented.
