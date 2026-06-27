# Task 6 Report: Safe NDI Receiver + Frame Capture

## Build Result
`cargo build` and `cargo test` both pass cleanly (12/12 tests, warnings only — no errors).

## Lifetime Adjustment

**What changed:** In `capture()`, the return type was changed from `CaptureResult` (verbatim brief) to `CaptureResult<'_>` (explicit elision marker).

**Why:** `CaptureResult<'a>` is a generic enum that requires a lifetime argument. Rust's lifetime elision applies to `&self` receivers (uses `&self`'s lifetime as the output lifetime), but this elision applies only when the return type is written as a concrete or `'_`-annotated type. Writing bare `CaptureResult` (omitting `<'_>`) is a type error because the compiler sees an unresolved lifetime parameter. Adding `<'_>` makes the elision explicit without changing any method semantics, field types, FFI calls, or safety invariants. This is the minimal change permitted by the brief.

**Safety is preserved:** `capture<'b>(&'b self) -> CaptureResult<'b>` — VideoFrame cannot outlive the borrow `&self`, and `&self` keeps Receiver alive, so the raw handle inside VideoFrame is always valid during use. `PhantomData<&'a Receiver<'a>>` (where `'a` = `'_` = lifetime of `&self`) enforces this via the borrow checker.

## Smoke Test Output

Live source: `NAPOCHAAN (OBS PGM)` (found via Finder on local network, 3 s timeout)

```
frame 1920x1080 stride=7680 bytes=8294400
```

- stride (7680) == width (1920) × 4 ✓  (`stride >= width*4` satisfied)
- bytes = 1920 × 1080 × 4 = 8,294,400 ✓

## Commit

SHA: `87e4087`  
Subject: `feat: add NDI receiver and frame capture`  
Files: `src/ndi/mod.rs` (+91 lines), `src/main.rs` (reverted to scaffold)

## Drop Verification

- `Receiver<'_>`: `Drop` calls `NDIlib_recv_destroy(self.handle)` ✓
- `VideoFrame<'_>`: `Drop` calls `NDIlib_recv_free_video_v2(self.receiver, &self.frame)` ✓

## Concerns / Notes

None. Both RAII destructors are present, signatures match the brief exactly (modulo the `<'_>` elision fix), and the live smoke confirmed correct BGRX_BGRA output at stride = width × 4.

---

# Task 6 Review Fixes (security hardening)

## Fix 1 — Dead-code warnings silenced

Added `#![allow(dead_code)]` at the top of `src/ndi/mod.rs` (above `pub mod sys;`). Also added missing targeted `#[allow(dead_code)]` attributes to three items in `src/cli.rs` (`SourceMatch`, `match_source`, `parse_selection`) that lacked them, which were necessary to achieve a pristine build.

## Fix 2 — Null-pointer guard in `VideoFrame::data()`

`data()` now returns `&[]` immediately if `self.frame.p_data.is_null()`. A malicious/malformed NDI sender could yield a frame with a null `p_data`; calling `std::slice::from_raw_parts` on a null pointer is undefined behavior. The guard prevents UB in that case.

## Fix 3 — Checked size arithmetic in `VideoFrame::data()`

`stride * height` now uses `.checked_mul()`. If the multiplication overflows `usize`, `data()` returns `&[]` instead of constructing a slice whose claimed length wraps around, which would allow an attacker-influenced frame to cause an OOB read. For valid frames (e.g., 1920×1080, stride=7680, len=8,294,400) the result is `Some(n)` and behavior is identical to before.

## Verification

### `cargo build` tail
```
Compiling ndi-share v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
```
Zero warnings — pristine.

### `cargo test` result
```
running 12 tests
test cli::tests::no_match_returns_none ... ok
test cli::tests::selection_trims_whitespace ... ok
test cli::tests::selection_non_numeric_is_error ... ok
test cli::tests::exact_match_wins_over_substring ... ok
test cli::tests::selection_valid_is_zero_based ... ok
test cli::tests::ambiguous_substring_returns_many ... ok
test cli::tests::selection_out_of_range_is_error ... ok
test cli::tests::selection_zero_is_error ... ok
test cli::tests::unique_substring_returns_one ... ok
test ndi::tests::cstr_to_string_null_is_empty ... ok
test ndi::tests::cstr_to_string_reads_valid ... ok
test ndi::sys::tests::initialize_links_and_succeeds ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
