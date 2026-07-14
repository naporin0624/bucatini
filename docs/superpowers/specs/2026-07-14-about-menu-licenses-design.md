# App menu (About / Usage / Licenses) + tray refresh — design

Date: 2026-07-14
Status: approved (user confirmed via AskUserQuestion, execution requested)

## Goal

Move first-run information out of bundled README files and into the app itself:
a native About panel, a Usage window (README-equivalent, JA/EN), and an Open
Source Licenses viewer — reachable from the macOS menu bar and, on both OSes,
from the tray menu. Refresh the tray (real icon, richer menu) and split the
606-line `src/bin/gui.rs` into modules while doing so. Reference
implementation: naporin0624/gemelli (`crates/gui/src/{menu,tray,licenses}.rs`,
`crates/xtask`).

## Decisions (user-confirmed)

1. **About UI**: native About panel (`muda::PredefinedMenuItem::about`) +
   separate `Help ▸ 使い方…` egui window for README-like content.
2. **Licenses**: full gemelli approach — `cargo-bundle-licenses` output merged
   with a hand-written appendix into a committed JSON asset, rendered in a
   searchable/filterable egui viewer.
3. **Windows entry point**: tray menu gains `About… / 使い方… / Licenses…`
   (no Windows menu bar this iteration).
4. **Tray**: replace the code-generated blue square with a real icon PNG,
   extend the menu, and split gui.rs into gemelli-style modules.
5. **Usage window language**: JA/EN toggle tabs, curated GUI-relevant content
   (not a raw README embed).
6. **License generation**: workspace-ify the repo and add an `xtask` crate
   (`cargo xtask gen-licenses [--check]`), matching gemelli.

## Module layout

Convert `src/bin/gui.rs` to a directory-style binary (no behavior change to
the NDI pipeline / run loop):

```
src/bin/gui/
├── main.rs      eframe bootstrap + GuiApp (discovery/start/stop/status as-is)
├── theme.rs     install_style + color tokens (moved verbatim)
├── menu.rs      muda app menu; macOS-only init_for_nsapp
├── tray.rs      tray icon + menu (status / About… / 使い方… / Licenses… / Quit)
├── usage.rs     Usage window, JA/EN toggle
└── licenses.rs  licenses viewer (search + All|Library|Font|Native filter,
                 click row to expand full text)
```

`Cargo.toml`: `path = "src/bin/gui.rs"` → `"src/bin/gui/main.rs"`.

## Menus and event routing

- macOS menu bar: `bucatini ▸ About / Quit`, `Help ▸ 使い方… / Open Source
  Licenses…` via `muda`; `Menu::init_for_nsapp()` behind
  `#[cfg(target_os = "macos")]`, called from `GuiApp::new` (NSApplication
  already exists there).
- Tray menu (both OSes): status line, separator, `About Bucatini…`,
  `使い方…`, `Open Source Licenses…`, separator, `Quit`. Tray About uses the
  same `PredefinedMenuItem::about` metadata.
- `tray-icon` re-exports `muda`, so app-menu and tray-menu events arrive on
  one global `MenuEvent` channel; a single poll in `main.rs` maps `MenuId` →
  action (`Show`, `Quit`, `OpenUsage`, `OpenLicenses`). Verify the version
  pairing with `cargo tree -i muda`; depend on `muda` directly at the version
  `tray-icon 0.24` re-exports.
- About metadata: name `Bucatini`, version `CARGO_PKG_VERSION`, authors
  `naporitan`, copyright `© 2026 naporitan`, website `https://napochaan.com`.
- Menu/tray build failures never abort the app (existing `Option` pattern).

## Usage window

- `egui` viewport (`show_viewport_immediate`, mirroring gemelli's licenses
  window rationale: immediate callback can borrow `&mut self`).
- Top toggle `EN | 日本語`; content is two embedded static strings curated for
  GUI use: what the tool does (NDI → Syphon/Spout), pick a source → Start,
  ✕ hides to tray, Cmd/Ctrl+Q quits, link to the repository.

## License data pipeline

- Root `Cargo.toml` gains `[workspace] members = [".", "xtask"]` (root package
  stays the primary crate).
- `xtask/` (clap subcommand `gen-licenses [--check]`):
  1. run `cargo bundle-licenses --format json` (subprocess, like gemelli),
  2. normalize + merge `licenses/appendix.json`,
  3. write `assets/third-party-licenses.json` (embedded via `include_str!`)
     and regenerate `THIRD-PARTY-NOTICES`;
  `--check` regenerates in memory and byte-compares, exiting nonzero if stale.
- `licenses/appendix.json` entries: Syphon Framework (BSD-3, native), Spout2
  SDK (BSD-2, native), NDI trademark/runtime notice (native), LINE Seed JP
  (OFL-1.1, font).
- Viewer schema identical to gemelli (`name/version/license/text/homepage/
  category`), categories `library|font|native`.
- CI + pre-push run `cargo xtask gen-licenses --check`.

## Tray icon asset

Generate a 44×44 PNG from `assets/icon/bucatini-1024.png` (sips), commit as
`assets/icon/tray-44.png`, embed with `include_bytes!`, decode with the `png`
crate (gui-feature-gated dependency), rejecting non-8-bit-RGBA like gemelli.

## Testing

- Unit tests mirroring gemelli: menu/tray `action_for` routing, about
  metadata fields, `parse_licenses`, `filter_entries` (query/category/AND),
  committed JSON contains every appendix entry, tray PNG decodes as 44×44
  RGBA, usage content non-empty in both languages.
- Existing 30 tests must stay green; finish with `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --features gui -- -D warnings`,
  `cargo test --features gui`.

## Out of scope

Windows menu bar (`init_for_hwnd`), packaging changes (DMG/Inno README
bundling stays), CLI changes, localization of the main window labels.
