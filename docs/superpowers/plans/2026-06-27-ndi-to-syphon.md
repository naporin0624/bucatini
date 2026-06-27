# NDI → Syphon CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS CLI (`ndi-share`) that receives an NDI video source and republishes its frames as a Syphon Metal texture.

**Architecture:** Hand-rolled `#[repr(C)]` FFI bindings to the system `libndi.dylib` provide source discovery and BGRA frame capture. Frames (BGRA bytes + stride + width/height) are passed to a vendored Objective-C++ Syphon shim (lifted from `naporin0624/electron-texture-bridge`) via a thin `extern "C"` FFI, which builds an `IOSurface`-backed `MTLTexture` and publishes it through `SyphonMetalServer`. A `SharedTextureOutput` trait keeps the output side swappable for a future Spout backend.

**Tech Stack:** Rust 2021, `clap` (CLI), `ctrlc` (signal), `anyhow` (errors), `cc` (build), system `libndi.dylib`, vendored `Syphon.framework`, Metal/IOSurface.

## Global Constraints

- Target platform v1: macOS only. Spout/Windows is a stub, not implemented.
- Binary name: `ndi-share`.
- NDI binding: hand-rolled FFI (NOT a crate). This supersedes the spec's "crate first" note — decision finalized because grafton-ndi 1.0.0 hides line stride, which `syphon_bridge_send_rgba` requires.
- NDI color format requested: `NDIlib_recv_color_format_BGRX_BGRA` (= 0). No YUV→RGB conversion.
- libndi location: `/usr/local/lib/libndi.dylib` (Homebrew `libndi`).
- Syphon shim source is the **sender subset only** of `electron-texture-bridge`'s `syphon_bridge.{mm,h}` (`syphon_bridge_create` / `syphon_bridge_send_rgba` / `syphon_bridge_destroy`). Do not vendor the receiver/discovery code.
- Syphon.framework is built from the `vendor/syphon-src` submodule into `vendor/Syphon.framework` before any macOS build.
- Rust edition 2021. Frequent commits, one per task.
- THIRD-PARTY-NOTICES must credit Syphon Framework (BSD).

## File Structure

- `Cargo.toml` — package + deps (`clap`, `ctrlc`, `anyhow`; build-dep `cc`).
- `build.rs` — links `libndi`; on macOS compiles the shim and links frameworks.
- `.gitmodules` — `vendor/syphon-src` → Syphon-Framework fork.
- `vendor/cpp/syphon_bridge.h` — sender-subset C header.
- `vendor/cpp/syphon_bridge.mm` — sender-subset Objective-C++ implementation.
- `scripts/setup-syphon.sh` — init submodule + build `Syphon.framework` into `vendor/`.
- `src/main.rs` — app wiring: discovery, receiver, output, capture loop, Ctrl-C.
- `src/cli.rs` — `clap` args, source-name matching, interactive selection (pure logic).
- `src/ndi/sys.rs` — raw FFI: extern fns + `#[repr(C)]` structs/enums.
- `src/ndi/mod.rs` — safe RAII wrapper: `Ndi`, `Finder`, `Source`, `Receiver`, `VideoFrame`.
- `src/output/mod.rs` — `SharedTextureOutput` trait + `BgraFrame` view type.
- `src/output/syphon.rs` — `extern "C"` shim FFI + `SyphonOutput` impl (macOS).
- `README.md`, `THIRD-PARTY-NOTICES` — docs/licenses.

---

### Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `build.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: a buildable binary crate named `ndi-share`; `build.rs` that links `libndi`.

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "ndi-share"
version = "0.1.0"
edition = "2021"
description = "Receive an NDI source and republish it as a Syphon Metal texture (macOS)"

[[bin]]
name = "ndi-share"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
ctrlc = "3"
anyhow = "1"

[build-dependencies]
cc = "1"
```

- [ ] **Step 2: Write `build.rs` (NDI link only for now)**

```rust
fn main() {
    // Link the Homebrew-installed NDI runtime.
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=dylib=ndi");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/local/lib");
}
```

- [ ] **Step 3: Write a placeholder `src/main.rs`**

```rust
fn main() {
    println!("ndi-share: scaffold");
}
```

- [ ] **Step 4: Build and run to verify the toolchain + NDI link**

Run: `cargo run`
Expected: compiles, prints `ndi-share: scaffold`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock build.rs src/main.rs
git commit -m "chore: scaffold ndi-share crate with libndi linkage"
```

---

### Task 2: CLI args + source-name matching (pure logic, TDD)

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs` (add `mod cli;`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Args { pub list: bool, pub source: Option<String>, pub name: Option<String>, pub timeout_ms: u32, pub verbose: bool }`
  - `pub fn parse() -> Args`
  - `pub enum SourceMatch { None, One(usize), Many(Vec<usize>) }`
  - `pub fn match_source(names: &[String], query: &str) -> SourceMatch` — case-insensitive substring match; exact (case-insensitive) match wins and returns `One` even if it is also a substring of others.

- [ ] **Step 1: Write failing tests in `src/cli.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec![
            "STUDIO (Camera 1)".to_string(),
            "STUDIO (Camera 2)".to_string(),
            "LAPTOP (Screen)".to_string(),
        ]
    }

    #[test]
    fn no_match_returns_none() {
        assert!(matches!(match_source(&names(), "xyz"), SourceMatch::None));
    }

    #[test]
    fn unique_substring_returns_one() {
        match match_source(&names(), "laptop") {
            SourceMatch::One(i) => assert_eq!(i, 2),
            other => panic!("expected One, got {:?}", other),
        }
    }

    #[test]
    fn ambiguous_substring_returns_many() {
        match match_source(&names(), "camera") {
            SourceMatch::Many(v) => assert_eq!(v, vec![0, 1]),
            other => panic!("expected Many, got {:?}", other),
        }
    }

    #[test]
    fn exact_match_wins_over_substring() {
        let n = vec!["Cam".to_string(), "Cam (extra)".to_string()];
        match match_source(&n, "cam") {
            SourceMatch::One(i) => assert_eq!(i, 0),
            other => panic!("expected One, got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli`
Expected: FAIL (cannot find `match_source` / `SourceMatch`).

- [ ] **Step 3: Implement `src/cli.rs`**

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ndi-share", about = "Republish an NDI source as a Syphon Metal texture")]
struct RawArgs {
    /// List discovered NDI sources and exit
    #[arg(long)]
    list: bool,
    /// NDI source name (case-insensitive substring match)
    #[arg(long)]
    source: Option<String>,
    /// Syphon server name to publish under (default: the NDI source name)
    #[arg(long)]
    name: Option<String>,
    /// Discovery / capture timeout in milliseconds
    #[arg(long, default_value_t = 5000)]
    timeout_ms: u32,
    /// Verbose logging (resolution, fps)
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug)]
pub struct Args {
    pub list: bool,
    pub source: Option<String>,
    pub name: Option<String>,
    pub timeout_ms: u32,
    pub verbose: bool,
}

pub fn parse() -> Args {
    let r = RawArgs::parse();
    Args {
        list: r.list,
        source: r.source,
        name: r.name,
        timeout_ms: r.timeout_ms,
        verbose: r.verbose,
    }
}

#[derive(Debug)]
pub enum SourceMatch {
    None,
    One(usize),
    Many(Vec<usize>),
}

pub fn match_source(names: &[String], query: &str) -> SourceMatch {
    let q = query.to_lowercase();
    if let Some(i) = names.iter().position(|n| n.to_lowercase() == q) {
        return SourceMatch::One(i);
    }
    let hits: Vec<usize> = names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();
    match hits.len() {
        0 => SourceMatch::None,
        1 => SourceMatch::One(hits[0]),
        _ => SourceMatch::Many(hits),
    }
}
```

- [ ] **Step 4: Wire module + run tests**

Add to `src/main.rs` top: `mod cli;`
Run: `cargo test --lib cli`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add CLI args and source-name matching"
```

---

### Task 3: Interactive selection parsing (pure logic, TDD)

**Files:**
- Modify: `src/cli.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub fn parse_selection(input: &str, count: usize) -> Result<usize, String>` — parses a 1-based index typed by the user, returns 0-based index; errors on non-numeric, zero, or out-of-range.

- [ ] **Step 1: Write failing tests (append to `tests` mod in `src/cli.rs`)**

```rust
    #[test]
    fn selection_valid_is_zero_based() {
        assert_eq!(parse_selection("2", 3), Ok(1));
    }

    #[test]
    fn selection_trims_whitespace() {
        assert_eq!(parse_selection("  1\n", 3), Ok(0));
    }

    #[test]
    fn selection_zero_is_error() {
        assert!(parse_selection("0", 3).is_err());
    }

    #[test]
    fn selection_out_of_range_is_error() {
        assert!(parse_selection("4", 3).is_err());
    }

    #[test]
    fn selection_non_numeric_is_error() {
        assert!(parse_selection("abc", 3).is_err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli`
Expected: FAIL (cannot find `parse_selection`).

- [ ] **Step 3: Implement `parse_selection` in `src/cli.rs`**

```rust
pub fn parse_selection(input: &str, count: usize) -> Result<usize, String> {
    let n: usize = input
        .trim()
        .parse()
        .map_err(|_| format!("'{}' is not a number", input.trim()))?;
    if n == 0 || n > count {
        return Err(format!("choose 1..={}", count));
    }
    Ok(n - 1)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib cli`
Expected: PASS (9 tests total).

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add interactive selection parsing"
```

---

### Task 4: NDI raw FFI bindings + link check

**Files:**
- Create: `src/ndi/sys.rs`
- Create: `src/ndi/mod.rs` (temporary: `pub mod sys;` only)
- Modify: `src/main.rs` (add `mod ndi;`)

**Interfaces:**
- Consumes: `libndi` (linked in Task 1).
- Produces (in `src/ndi/sys.rs`, all `pub`):
  - Opaque handles: `pub type NDIlib_find_instance_t = *mut c_void;` `pub type NDIlib_recv_instance_t = *mut c_void;`
  - Structs: `NDIlib_source_t`, `NDIlib_find_create_t`, `NDIlib_recv_create_v3_t`, `NDIlib_video_frame_v2_t`.
  - Enums as constants: color format `BGRX_BGRA = 0`, bandwidth `HIGHEST = 100`, frame type `VIDEO = 1`.
  - extern fns: `NDIlib_initialize`, `NDIlib_find_create_v2`, `NDIlib_find_destroy`, `NDIlib_find_get_current_sources`, `NDIlib_find_wait_for_sources`, `NDIlib_recv_create_v3`, `NDIlib_recv_destroy`, `NDIlib_recv_capture_v2`, `NDIlib_recv_free_video_v2`.

- [ ] **Step 1: Write `src/ndi/sys.rs`**

```rust
#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

pub type NDIlib_find_instance_t = *mut c_void;
pub type NDIlib_recv_instance_t = *mut c_void;

// color formats
pub const NDIlib_recv_color_format_BGRX_BGRA: c_int = 0;
// bandwidth
pub const NDIlib_recv_bandwidth_highest: c_int = 100;
// frame types
pub const NDIlib_frame_type_none: c_int = 0;
pub const NDIlib_frame_type_video: c_int = 1;
pub const NDIlib_frame_type_error: c_int = 4;

#[repr(C)]
pub struct NDIlib_source_t {
    pub p_ndi_name: *const c_char,
    /// union { p_url_address; p_ip_address } — single pointer
    pub p_url_address: *const c_char,
}

#[repr(C)]
pub struct NDIlib_find_create_t {
    pub show_local_sources: bool,
    pub p_groups: *const c_char,
    pub p_extra_ips: *const c_char,
}

#[repr(C)]
pub struct NDIlib_recv_create_v3_t {
    pub source_to_connect_to: NDIlib_source_t,
    pub color_format: c_int,
    pub bandwidth: c_int,
    pub allow_video_fields: bool,
    pub p_ndi_recv_name: *const c_char,
}

#[repr(C)]
pub struct NDIlib_video_frame_v2_t {
    pub xres: c_int,
    pub yres: c_int,
    pub four_cc: c_int,
    pub frame_rate_n: c_int,
    pub frame_rate_d: c_int,
    pub picture_aspect_ratio: f32,
    pub frame_format_type: c_int,
    pub timecode: i64,
    pub p_data: *mut u8,
    /// union { line_stride_in_bytes; data_size_in_bytes }
    pub line_stride_or_size: c_int,
    pub p_metadata: *const c_char,
    pub timestamp: i64,
}

extern "C" {
    pub fn NDIlib_initialize() -> bool;
    pub fn NDIlib_find_create_v2(p_create_settings: *const NDIlib_find_create_t) -> NDIlib_find_instance_t;
    pub fn NDIlib_find_destroy(p_instance: NDIlib_find_instance_t);
    pub fn NDIlib_find_get_current_sources(
        p_instance: NDIlib_find_instance_t,
        p_no_sources: *mut u32,
    ) -> *const NDIlib_source_t;
    pub fn NDIlib_find_wait_for_sources(
        p_instance: NDIlib_find_instance_t,
        timeout_in_ms: u32,
    ) -> bool;
    pub fn NDIlib_recv_create_v3(p_create_settings: *const NDIlib_recv_create_v3_t) -> NDIlib_recv_instance_t;
    pub fn NDIlib_recv_destroy(p_instance: NDIlib_recv_instance_t);
    pub fn NDIlib_recv_capture_v2(
        p_instance: NDIlib_recv_instance_t,
        p_video_data: *mut NDIlib_video_frame_v2_t,
        p_audio_data: *mut c_void,
        p_metadata: *mut c_void,
        timeout_in_ms: u32,
    ) -> c_int;
    pub fn NDIlib_recv_free_video_v2(
        p_instance: NDIlib_recv_instance_t,
        p_video_data: *const NDIlib_video_frame_v2_t,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_links_and_succeeds() {
        // Proves the dylib is linked and the symbol resolves at runtime.
        assert!(unsafe { NDIlib_initialize() });
    }
}
```

- [ ] **Step 2: Create `src/ndi/mod.rs` and wire it**

`src/ndi/mod.rs`:
```rust
pub mod sys;
```
Add to `src/main.rs` top: `mod ndi;`

- [ ] **Step 3: Run the link/init test**

Run: `cargo test --lib ndi::sys`
Expected: PASS (`initialize_links_and_succeeds`). A failure here means the dylib is not linked — verify `/usr/local/lib/libndi.dylib` exists.

- [ ] **Step 4: Commit**

```bash
git add src/ndi/sys.rs src/ndi/mod.rs src/main.rs
git commit -m "feat: add hand-rolled NDI FFI bindings"
```

---

### Task 5: Safe NDI discovery wrapper (`Ndi`, `Finder`, `Source`)

**Files:**
- Modify: `src/ndi/mod.rs`

**Interfaces:**
- Consumes: `src/ndi/sys.rs`.
- Produces (all `pub`):
  - `struct Ndi` with `fn new() -> anyhow::Result<Ndi>` (calls `NDIlib_initialize`).
  - `struct Source { pub name: String, pub url: String }`.
  - `struct Finder<'a>` with `fn new(ndi: &'a Ndi) -> anyhow::Result<Finder<'a>>` and `fn list(&self, timeout_ms: u32) -> Vec<Source>` (waits then snapshots current sources). `Drop` calls `NDIlib_find_destroy`.
  - Internal helper `fn cstr_to_string(p: *const c_char) -> String` — null/invalid → empty string. (testable)

- [ ] **Step 1: Write a failing test for the helper in `src/ndi/mod.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn cstr_to_string_reads_valid() {
        let c = CString::new("STUDIO (Cam 1)").unwrap();
        assert_eq!(cstr_to_string(c.as_ptr()), "STUDIO (Cam 1)");
    }

    #[test]
    fn cstr_to_string_null_is_empty() {
        assert_eq!(cstr_to_string(std::ptr::null()), "");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib ndi::tests`
Expected: FAIL (cannot find `cstr_to_string`).

- [ ] **Step 3: Implement the wrapper in `src/ndi/mod.rs`**

```rust
pub mod sys;

use anyhow::{anyhow, Result};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

pub struct Ndi {
    _private: (),
}

impl Ndi {
    pub fn new() -> Result<Ndi> {
        if unsafe { sys::NDIlib_initialize() } {
            Ok(Ndi { _private: () })
        } else {
            Err(anyhow!(
                "NDIlib_initialize failed (libndi present but CPU unsupported?)"
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub url: String,
}

pub struct Finder<'a> {
    handle: sys::NDIlib_find_instance_t,
    _ndi: &'a Ndi,
}

impl<'a> Finder<'a> {
    pub fn new(ndi: &'a Ndi) -> Result<Finder<'a>> {
        let create = sys::NDIlib_find_create_t {
            show_local_sources: true,
            p_groups: ptr::null(),
            p_extra_ips: ptr::null(),
        };
        let handle = unsafe { sys::NDIlib_find_create_v2(&create) };
        if handle.is_null() {
            return Err(anyhow!("NDIlib_find_create_v2 returned null"));
        }
        Ok(Finder { handle, _ndi: ndi })
    }

    pub fn list(&self, timeout_ms: u32) -> Vec<Source> {
        unsafe { sys::NDIlib_find_wait_for_sources(self.handle, timeout_ms) };
        let mut count: u32 = 0;
        let ptr = unsafe { sys::NDIlib_find_get_current_sources(self.handle, &mut count) };
        if ptr.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(ptr, count as usize) };
        slice
            .iter()
            .map(|s| Source {
                name: cstr_to_string(s.p_ndi_name),
                url: cstr_to_string(s.p_url_address),
            })
            .collect()
    }
}

impl Drop for Finder<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_find_destroy(self.handle) };
    }
}
```

- [ ] **Step 4: Run unit tests**

Run: `cargo test --lib ndi::tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Manual discovery smoke test**

Add a temporary call in `src/main.rs` `main()`:
```rust
let ndi = ndi::Ndi::new().unwrap();
let finder = ndi::Finder::new(&ndi).unwrap();
for s in finder.list(3000) { println!("{} -> {}", s.name, s.url); }
```
Run (with at least one NDI source on the network — e.g. NDI Tools Test Pattern): `cargo run`
Expected: prints discovered source name(s). Then revert the temporary `main()` body to `println!("ndi-share: scaffold");`.

- [ ] **Step 6: Commit**

```bash
git add src/ndi/mod.rs src/main.rs
git commit -m "feat: add safe NDI discovery wrapper"
```

---

### Task 6: Safe NDI receiver + frame capture

**Files:**
- Modify: `src/ndi/mod.rs`

**Interfaces:**
- Consumes: `Ndi`, `Source`, `sys`.
- Produces (all `pub`):
  - `struct Receiver<'a>` with `fn new(ndi: &'a Ndi, source: &Source, recv_name: &str) -> Result<Receiver<'a>>` (requests `BGRX_BGRA`, `highest`, `allow_video_fields=false`). `Drop` → `NDIlib_recv_destroy`.
  - `fn capture(&self, timeout_ms: u32) -> CaptureResult` where `pub enum CaptureResult { Video(VideoFrame<'_>), None, Error }`.
  - `struct VideoFrame<'a>` borrowing the receiver, with `fn width(&self)->u32`, `fn height(&self)->u32`, `fn stride(&self)->u32`, `fn data(&self)->&[u8]`. `Drop` → `NDIlib_recv_free_video_v2`.

- [ ] **Step 1: Append the receiver implementation to `src/ndi/mod.rs`**

```rust
pub struct Receiver<'a> {
    handle: sys::NDIlib_recv_instance_t,
    _ndi: &'a Ndi,
}

impl<'a> Receiver<'a> {
    pub fn new(ndi: &'a Ndi, source: &Source, recv_name: &str) -> Result<Receiver<'a>> {
        let c_name = std::ffi::CString::new(source.name.clone())?;
        let c_url = std::ffi::CString::new(source.url.clone())?;
        let c_recv = std::ffi::CString::new(recv_name)?;
        let create = sys::NDIlib_recv_create_v3_t {
            source_to_connect_to: sys::NDIlib_source_t {
                p_ndi_name: c_name.as_ptr(),
                p_url_address: c_url.as_ptr(),
            },
            color_format: sys::NDIlib_recv_color_format_BGRX_BGRA,
            bandwidth: sys::NDIlib_recv_bandwidth_highest,
            allow_video_fields: false,
            p_ndi_recv_name: c_recv.as_ptr(),
        };
        let handle = unsafe { sys::NDIlib_recv_create_v3(&create) };
        if handle.is_null() {
            return Err(anyhow!("NDIlib_recv_create_v3 returned null"));
        }
        // c_name/c_url/c_recv are copied by the SDK during create; safe to drop now.
        Ok(Receiver { handle, _ndi: ndi })
    }

    pub fn capture(&self, timeout_ms: u32) -> CaptureResult {
        let mut frame: sys::NDIlib_video_frame_v2_t = unsafe { std::mem::zeroed() };
        let t = unsafe {
            sys::NDIlib_recv_capture_v2(
                self.handle,
                &mut frame,
                ptr::null_mut(),
                ptr::null_mut(),
                timeout_ms,
            )
        };
        match t {
            sys::NDIlib_frame_type_video => CaptureResult::Video(VideoFrame {
                receiver: self.handle,
                frame,
                _marker: std::marker::PhantomData,
            }),
            sys::NDIlib_frame_type_error => CaptureResult::Error,
            _ => CaptureResult::None,
        }
    }
}

impl Drop for Receiver<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_recv_destroy(self.handle) };
    }
}

pub enum CaptureResult<'a> {
    Video(VideoFrame<'a>),
    None,
    Error,
}

pub struct VideoFrame<'a> {
    receiver: sys::NDIlib_recv_instance_t,
    frame: sys::NDIlib_video_frame_v2_t,
    _marker: std::marker::PhantomData<&'a Receiver<'a>>,
}

impl VideoFrame<'_> {
    pub fn width(&self) -> u32 {
        self.frame.xres as u32
    }
    pub fn height(&self) -> u32 {
        self.frame.yres as u32
    }
    pub fn stride(&self) -> u32 {
        self.frame.line_stride_or_size as u32
    }
    pub fn data(&self) -> &[u8] {
        let len = self.stride() as usize * self.height() as usize;
        unsafe { std::slice::from_raw_parts(self.frame.p_data, len) }
    }
}

impl Drop for VideoFrame<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_recv_free_video_v2(self.receiver, &self.frame) };
    }
}
```

- [ ] **Step 2: Build to verify it compiles**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 3: Manual capture smoke test**

Temporarily set `src/main.rs` `main()` to:
```rust
let ndi = ndi::Ndi::new().unwrap();
let finder = ndi::Finder::new(&ndi).unwrap();
let sources = finder.list(3000);
let src = sources.first().expect("need an NDI source on the network");
let recv = ndi::Receiver::new(&ndi, src, "ndi-share-smoke").unwrap();
for _ in 0..120 {
    if let ndi::CaptureResult::Video(f) = recv.capture(1000) {
        println!("frame {}x{} stride={} bytes={}", f.width(), f.height(), f.stride(), f.data().len());
        break;
    }
}
```
Run: `cargo run`
Expected: prints a frame with sane dimensions and `stride >= width*4`. Then revert `main()` to `println!("ndi-share: scaffold");`.

- [ ] **Step 4: Commit**

```bash
git add src/ndi/mod.rs src/main.rs
git commit -m "feat: add NDI receiver and frame capture"
```

---

### Task 7: Output trait + `BgraFrame`

**Files:**
- Create: `src/output/mod.rs`
- Modify: `src/main.rs` (add `mod output;`)

**Interfaces:**
- Consumes: nothing.
- Produces (all `pub`):
  - `struct BgraFrame<'a> { pub data: &'a [u8], pub width: u32, pub height: u32, pub stride: u32 }`.
  - `trait SharedTextureOutput { fn publish(&mut self, frame: &BgraFrame) -> anyhow::Result<()>; }`.

- [ ] **Step 1: Write `src/output/mod.rs`**

```rust
#[cfg(target_os = "macos")]
pub mod syphon;

/// A borrowed view of one BGRA frame ready to publish.
pub struct BgraFrame<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    /// bytes per row (may exceed width*4 due to padding)
    pub stride: u32,
}

pub trait SharedTextureOutput {
    fn publish(&mut self, frame: &BgraFrame) -> anyhow::Result<()>;
}
```

- [ ] **Step 2: Wire module + build**

Add to `src/main.rs` top: `mod output;`
Run: `cargo build`
Expected: compiles (the `syphon` submodule is empty/absent — that is fine; this step only declares the trait. If `cargo build` errors on the missing `syphon` module, create an empty `src/output/syphon.rs` now; it is filled in Task 9.)

- [ ] **Step 3: Commit**

```bash
git add src/output/mod.rs src/main.rs
git commit -m "feat: add SharedTextureOutput trait and BgraFrame"
```

---

### Task 8: Vendor the Syphon shim + framework build infra

**Files:**
- Create: `.gitmodules`
- Create: `vendor/cpp/syphon_bridge.h`
- Create: `vendor/cpp/syphon_bridge.mm`
- Create: `scripts/setup-syphon.sh`
- Modify: `build.rs` (add macOS section)

**Interfaces:**
- Consumes: `Syphon.framework` (produced by the setup script into `vendor/`).
- Produces: C symbols `syphon_bridge_create`, `syphon_bridge_send_rgba`, `syphon_bridge_destroy` linked into the binary.

- [ ] **Step 1: Add the submodule + setup script**

`.gitmodules`:
```
[submodule "vendor/syphon-src"]
	path = vendor/syphon-src
	url = https://github.com/naporin0624/Syphon-Framework.git
```

`scripts/setup-syphon.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. Fetch the Syphon framework source.
git submodule update --init --recursive

# 2. Build Syphon.framework (Release) and copy it into vendor/.
cd vendor/syphon-src
xcodebuild -project Syphon.xcodeproj -target Syphon -configuration Release \
  -derivedDataPath build SYMROOT="$PWD/build"
cd "$ROOT"
rm -rf vendor/Syphon.framework
cp -R vendor/syphon-src/build/Release/Syphon.framework vendor/Syphon.framework
echo "Syphon.framework installed at vendor/Syphon.framework"
```
Then: `chmod +x scripts/setup-syphon.sh`

> Prerequisite: if `xcrun` cannot find `MacOSX.sdk`, run `sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer` (or `xcode-select --install`) before this script.

- [ ] **Step 2: Run the setup script**

Run: `git init` (if needed) then `./scripts/setup-syphon.sh`
Expected: `vendor/Syphon.framework` exists (`ls vendor/Syphon.framework/Syphon`).

- [ ] **Step 3: Write `vendor/cpp/syphon_bridge.h` (sender subset)**

```c
#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef void* SyphonBridgeHandle;

SyphonBridgeHandle syphon_bridge_create(const char* name);
void               syphon_bridge_destroy(SyphonBridgeHandle handle);

// Publish a BGRA buffer. bytes_per_row is the source stride (>= width*4).
// Returns 0 on success, -1 on error.
int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row);

#ifdef __cplusplus
}
#endif
```

- [ ] **Step 4: Write `vendor/cpp/syphon_bridge.mm` (sender subset)**

```objc
#import "syphon_bridge.h"
#import <Metal/Metal.h>
#import <IOSurface/IOSurface.h>
#import <Syphon/Syphon.h>
#import <Cocoa/Cocoa.h>

struct SyphonBridge {
    id<MTLDevice>       device;
    id<MTLCommandQueue> commandQueue;
    SyphonMetalServer*  server;
};

extern "C" {

SyphonBridgeHandle syphon_bridge_create(const char* name) {
    @autoreleasepool {
        auto* bridge = new SyphonBridge();
        bridge->device = MTLCreateSystemDefaultDevice();
        if (!bridge->device) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create Metal device");
            delete bridge;
            return nullptr;
        }
        bridge->commandQueue = [bridge->device newCommandQueue];
        NSString* serverName = [NSString stringWithUTF8String:name];
        bridge->server = [[SyphonMetalServer alloc] initWithName:serverName
                                                          device:bridge->device
                                                         options:nil];
        if (!bridge->server) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create SyphonMetalServer");
            delete bridge;
            return nullptr;
        }
        return static_cast<SyphonBridgeHandle>(bridge);
    }
}

void syphon_bridge_destroy(SyphonBridgeHandle handle) {
    if (!handle) return;
    @autoreleasepool {
        auto* bridge = static_cast<SyphonBridge*>(handle);
        [bridge->server stop];
        bridge->server       = nil;
        bridge->commandQueue = nil;
        bridge->device       = nil;
        delete bridge;
    }
}

int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row) {
    if (!handle || !data) return -1;
    @autoreleasepool {
        auto* bridge = static_cast<SyphonBridge*>(handle);

        NSDictionary* surfaceProps = @{
            (NSString*)kIOSurfaceWidth: @(width),
            (NSString*)kIOSurfaceHeight: @(height),
            (NSString*)kIOSurfaceBytesPerElement: @4,
            (NSString*)kIOSurfaceBytesPerRow: @(bytes_per_row),
            (NSString*)kIOSurfacePixelFormat: @(kCVPixelFormatType_32BGRA),
            (NSString*)kIOSurfaceAllocSize: @(bytes_per_row * height)
        };
        IOSurfaceRef surface = IOSurfaceCreate((__bridge CFDictionaryRef)surfaceProps);
        if (!surface) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create IOSurface");
            return -1;
        }

        IOSurfaceLock(surface, 0, nullptr);
        void* baseAddr = IOSurfaceGetBaseAddress(surface);
        size_t surfaceBytesPerRow = IOSurfaceGetBytesPerRow(surface);
        const uint8_t* srcRow = data;
        uint8_t* dstRow = static_cast<uint8_t*>(baseAddr);
        size_t copyWidth = (size_t)width * 4;
        for (uint32_t y = 0; y < height; y++) {
            memcpy(dstRow, srcRow, copyWidth);
            srcRow += bytes_per_row;
            dstRow += surfaceBytesPerRow;
        }
        IOSurfaceUnlock(surface, 0, nullptr);

        MTLTextureDescriptor* desc =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:width
                                                              height:height
                                                           mipmapped:NO];
        desc.usage = MTLTextureUsageShaderRead;
        desc.storageMode = MTLStorageModeShared;
        id<MTLTexture> texture = [bridge->device newTextureWithDescriptor:desc
                                                               iosurface:surface
                                                                   plane:0];
        CFRelease(surface);
        if (!texture) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create Metal texture from IOSurface");
            return -1;
        }

        id<MTLCommandBuffer> cmdBuf = [bridge->commandQueue commandBuffer];
        [bridge->server publishFrameTexture:texture
                            onCommandBuffer:cmdBuf
                                imageRegion:NSMakeRect(0, 0, width, height)
                                    flipped:YES];
        [cmdBuf commit];
        return 0;
    }
}

} // extern "C"
```

- [ ] **Step 5: Add the macOS section to `build.rs`**

Replace `build.rs` with:
```rust
fn main() {
    // Link the Homebrew-installed NDI runtime.
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=dylib=ndi");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/local/lib");

    #[cfg(target_os = "macos")]
    build_macos();
}

#[cfg(target_os = "macos")]
fn build_macos() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor = std::path::Path::new(&manifest).join("vendor");
    let vendor_str = vendor.to_str().unwrap();

    println!("cargo:rerun-if-changed=vendor/cpp/syphon_bridge.mm");
    println!("cargo:rerun-if-changed=vendor/cpp/syphon_bridge.h");

    cc::Build::new()
        .file("vendor/cpp/syphon_bridge.mm")
        .include("vendor/cpp")
        .flag("-ObjC++")
        .flag("-std=c++17")
        .flag("-fobjc-arc")
        .flag("-F")
        .flag(vendor_str)
        .compile("syphon_bridge");

    println!("cargo:rustc-link-lib=c++");
    println!("cargo:rustc-link-lib=framework=Syphon");
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=IOSurface");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rustc-link-lib=framework=QuartzCore");
    println!("cargo:rustc-link-search=framework={vendor_str}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{vendor_str}");
}
```

- [ ] **Step 6: Build to verify the shim compiles and links**

Run: `cargo build`
Expected: compiles; `cc` builds `syphon_bridge.mm` and the binary links against `Syphon.framework`. A linker error about `-framework Syphon` means Step 2 (setup script) was not run.

- [ ] **Step 7: Commit**

```bash
git add .gitmodules scripts/setup-syphon.sh vendor/cpp build.rs
git commit -m "build: vendor Syphon sender shim and framework build infra"
```

---

### Task 9: `SyphonOutput` implementation

**Files:**
- Modify (or create if empty): `src/output/syphon.rs`

**Interfaces:**
- Consumes: `BgraFrame`, `SharedTextureOutput` (Task 7); shim C symbols (Task 8).
- Produces: `pub struct SyphonOutput` with `pub fn new(name: &str) -> anyhow::Result<SyphonOutput>`, `impl SharedTextureOutput`, and `Drop`.

- [ ] **Step 1: Write `src/output/syphon.rs`**

```rust
use super::{BgraFrame, SharedTextureOutput};
use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

type SyphonBridgeHandle = *mut c_void;

extern "C" {
    fn syphon_bridge_create(name: *const c_char) -> SyphonBridgeHandle;
    fn syphon_bridge_destroy(handle: SyphonBridgeHandle);
    fn syphon_bridge_send_rgba(
        handle: SyphonBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
    ) -> i32;
}

pub struct SyphonOutput {
    handle: SyphonBridgeHandle,
}

impl SyphonOutput {
    pub fn new(name: &str) -> Result<SyphonOutput> {
        let c_name = CString::new(name)?;
        let handle = unsafe { syphon_bridge_create(c_name.as_ptr()) };
        if handle.is_null() {
            return Err(anyhow!("syphon_bridge_create failed (Metal/Syphon init)"));
        }
        Ok(SyphonOutput { handle })
    }
}

impl SharedTextureOutput for SyphonOutput {
    fn publish(&mut self, frame: &BgraFrame) -> Result<()> {
        let rc = unsafe {
            syphon_bridge_send_rgba(
                self.handle,
                frame.data.as_ptr(),
                frame.width,
                frame.height,
                frame.stride,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(anyhow!("syphon_bridge_send_rgba returned {rc}"))
        }
    }
}

impl Drop for SyphonOutput {
    fn drop(&mut self) {
        unsafe { syphon_bridge_destroy(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_a_synthetic_frame() {
        let mut out = SyphonOutput::new("ndi-share-test").expect("create server");
        let w = 16u32;
        let h = 16u32;
        let stride = w * 4;
        let data = vec![0u8; (stride * h) as usize];
        let frame = BgraFrame { data: &data, width: w, height: h, stride };
        out.publish(&frame).expect("publish ok");
    }
}
```

- [ ] **Step 2: Run the publish test**

Run: `cargo test --lib output::syphon`
Expected: PASS. This creates a real `SyphonMetalServer` named `ndi-share-test` and publishes one 16×16 frame; no client required. A failure indicates a Metal/Syphon linkage or framework-load problem.

- [ ] **Step 3: Commit**

```bash
git add src/output/syphon.rs
git commit -m "feat: implement SyphonOutput backend"
```

---

### Task 10: Application wiring (discovery → receive → publish loop)

**Files:**
- Rewrite: `src/main.rs`

**Interfaces:**
- Consumes: `cli`, `ndi`, `output` modules.
- Produces: the full CLI behavior described in the spec.

- [ ] **Step 1: Rewrite `src/main.rs`**

```rust
mod cli;
mod ndi;
mod output;

use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cli::SourceMatch;
use ndi::{CaptureResult, Finder, Ndi, Receiver, Source};
use output::{BgraFrame, SharedTextureOutput};

fn main() -> Result<()> {
    let args = cli::parse();
    let ndi = Ndi::new()?;
    let finder = Finder::new(&ndi)?;
    let sources = finder.list(args.timeout_ms);

    if args.list {
        print_sources(&sources);
        return Ok(());
    }
    if sources.is_empty() {
        return Err(anyhow!(
            "no NDI sources found within {} ms (is a source online?)",
            args.timeout_ms
        ));
    }

    let source = select_source(&sources, &args.source)?;
    let server_name = args.name.clone().unwrap_or_else(|| source.name.clone());

    let receiver = Receiver::new(&ndi, &source, "ndi-share")?;
    let mut out = make_output(&server_name)?;

    println!("Publishing '{}' as Syphon server '{}'. Ctrl-C to stop.", source.name, server_name);
    run_loop(&receiver, &mut *out, args.verbose)
}

fn print_sources(sources: &[Source]) {
    if sources.is_empty() {
        println!("(no NDI sources found)");
        return;
    }
    for (i, s) in sources.iter().enumerate() {
        println!("{}: {} ({})", i + 1, s.name, s.url);
    }
}

fn select_source(sources: &[Source], query: &Option<String>) -> Result<Source> {
    let names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();
    match query {
        Some(q) => match cli::match_source(&names, q) {
            SourceMatch::One(i) => Ok(sources[i].clone()),
            SourceMatch::None => Err(anyhow!("no source matches '{}'. Available:\n{}", q, list_str(sources))),
            SourceMatch::Many(v) => Err(anyhow!(
                "'{}' is ambiguous ({} matches). Be more specific:\n{}",
                q,
                v.len(),
                list_str(sources)
            )),
        },
        None => prompt_select(sources),
    }
}

fn list_str(sources: &[Source]) -> String {
    sources
        .iter()
        .enumerate()
        .map(|(i, s)| format!("  {}: {}", i + 1, s.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn prompt_select(sources: &[Source]) -> Result<Source> {
    print_sources(sources);
    print!("Select source [1-{}]: ", sources.len());
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    match cli::parse_selection(&line, sources.len()) {
        Ok(i) => Ok(sources[i].clone()),
        Err(e) => Err(anyhow!("invalid selection: {e}")),
    }
}

#[cfg(target_os = "macos")]
fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(output::syphon::SyphonOutput::new(name)?))
}

#[cfg(not(target_os = "macos"))]
fn make_output(_name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Err(anyhow!("only macOS/Syphon output is implemented in v1"))
}

fn run_loop(receiver: &Receiver, out: &mut dyn SharedTextureOutput, verbose: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))?;
    }

    let mut last_dims = (0u32, 0u32);
    while running.load(Ordering::SeqCst) {
        match receiver.capture(1000) {
            CaptureResult::Video(frame) => {
                let dims = (frame.width(), frame.height());
                if verbose && dims != last_dims {
                    eprintln!("frame {}x{} stride={}", dims.0, dims.1, frame.stride());
                    last_dims = dims;
                }
                let bgra = BgraFrame {
                    data: frame.data(),
                    width: frame.width(),
                    height: frame.height(),
                    stride: frame.stride(),
                };
                if let Err(e) = out.publish(&bgra) {
                    eprintln!("publish error: {e}");
                }
            }
            CaptureResult::Error => eprintln!("NDI capture error"),
            CaptureResult::None => {} // timeout / non-video frame; keep polling
        }
    }
    println!("\nStopped.");
    Ok(())
}
```

- [ ] **Step 2: Build**

Run: `cargo build`
Expected: compiles cleanly.

- [ ] **Step 3: End-to-end manual verification**

1. Start an NDI source (NDI Tools → Test Pattern, or any NDI sender).
2. Run: `cargo run -- --verbose`
3. Pick the source at the prompt (or use `--source <name>`).
4. Open a Syphon client (Syphon Recorder, or Resolume "Syphon" source).
5. Confirm the live NDI video appears in the Syphon client.
6. Press Ctrl-C; confirm clean "Stopped." exit.

Also verify: `cargo run -- --list` prints sources and exits; `cargo run -- --source nonexistent` errors with the available list.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire NDI discovery, receive, and Syphon publish loop"
```

---

### Task 11: README, license notices, .gitignore

**Files:**
- Create: `README.md`
- Create: `THIRD-PARTY-NOTICES`
- Modify: `.gitignore` (ensure `vendor/syphon-src/build` and `vendor/Syphon.framework` handling)

**Interfaces:**
- Consumes: nothing.
- Produces: setup/usage docs and license attribution.

- [ ] **Step 1: Write `README.md`**

````markdown
# ndi-share

Receive an NDI video source and republish it as a Syphon Metal texture (macOS).

## Prerequisites

- macOS with Xcode command line tools. If `xcrun` cannot find `MacOSX.sdk`:
  `sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer`
- NDI runtime: `brew install libndi` (provides `/usr/local/lib/libndi.dylib`).
- Rust (stable).

## Build

```bash
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
````

- [ ] **Step 2: Write `THIRD-PARTY-NOTICES`**

```
THIRD-PARTY SOFTWARE NOTICES AND INFORMATION

Syphon Framework
https://github.com/Syphon/Syphon-Framework
Copyright 2010 bangnoise (Tom Butterworth) & vade (Anton Marini).
Licensed under the BSD 3-Clause License. See the Syphon-Framework source for
the full license text.

NDI(R) is a registered trademark of Vizrt NV. This tool links against the NDI
runtime (libndi) installed separately by the user; no NDI SDK code is
redistributed here.
```

- [ ] **Step 3: Update `.gitignore`**

Ensure it contains:
```
/target
**/*.rs.bk
.DS_Store
/vendor/Syphon.framework
/vendor/syphon-src/build
```

- [ ] **Step 4: Commit**

```bash
git add README.md THIRD-PARTY-NOTICES .gitignore
git commit -m "docs: add README and third-party notices"
```

---

## Self-Review

**Spec coverage:**
- CLI (`--list`/`--source`/interactive/`--name`/`--timeout`/`--verbose`) → Tasks 2, 3, 10. ✓
- NDI discovery & BGRA receive → Tasks 4, 5, 6. ✓
- `SharedTextureOutput` trait + Syphon backend → Tasks 7, 9. ✓
- Vendored sender shim + framework infra (submodule, setup script, build.rs) → Task 8. ✓
- Data flow (BGRX_BGRA, stride-aware publish, resize handled by per-frame IOSurface) → Tasks 6, 8, 9, 10. ✓
- Error handling (init fail, no match, ambiguous, no sources, publish error) → Tasks 5, 9, 10. ✓
- Tests (name matching, selection, FFI link, synthetic publish) + manual E2E → Tasks 2, 3, 4, 9, 10. ✓
- Build/distribution + license + xcode-select note → Tasks 8, 11. ✓
- YAGNI exclusions (audio, Spout body, multi-source, TUI, YUV shader) honored. ✓

**Placeholder scan:** No TBD/TODO; the only empty file (`src/output/syphon.rs` in Task 7) is explicitly filled in Task 9.

**Type consistency:** `match_source`/`SourceMatch`/`parse_selection` (cli) used consistently in Task 10; `BgraFrame{data,width,height,stride}` and `SharedTextureOutput::publish` identical across Tasks 7/9/10; `CaptureResult`/`VideoFrame::{width,height,stride,data}` consistent across Tasks 6/10; shim symbols `syphon_bridge_create/send_rgba/destroy` identical in Tasks 8/9.
