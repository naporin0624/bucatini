//! Capture loop core shared by the CLI and GUI.

use crate::output::{BgraFrame, SharedTextureOutput};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// True if `data_len` is large enough to hold a `stride * height` BGRA frame.
pub fn frame_within_bounds(data_len: usize, stride: u32, height: u32) -> bool {
    let needed = (stride as usize).saturating_mul(height as usize);
    data_len >= needed
}

/// Validate one frame and publish it. Returns whether it was published.
pub fn handle_video_frame(
    out: &mut dyn SharedTextureOutput,
    frame: &BgraFrame,
) -> anyhow::Result<bool> {
    if !frame_within_bounds(frame.data.len(), frame.stride, frame.height) {
        return Ok(false);
    }
    out.publish(frame)?;
    Ok(true)
}

/// Outcome of one capture attempt.
pub enum CaptureSignal {
    /// A video frame was delivered to the callback.
    Got,
    /// Timeout / non-video frame — keep polling.
    Idle,
    /// The source reported a capture error.
    Error,
}

/// A source of BGRA frames. Implemented by `Receiver` (real) and mocks (tests).
///
/// The frame is *lent* to `on_frame` for the duration of the call so production
/// stays zero-copy (the NDI buffer is freed right after the callback returns).
pub trait FrameStream {
    fn capture_into(&self, timeout_ms: u32, on_frame: &mut dyn FnMut(BgraFrame)) -> CaptureSignal;
}

/// Pump frames from `stream` to `out` until `stop` is set false.
/// Each published frame increments `frames`.
pub fn run_capture_loop<S: FrameStream>(
    stream: &S,
    out: &mut dyn SharedTextureOutput,
    stop: &AtomicBool,
    frames: &AtomicU64,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut last_dims = (0u32, 0u32);
    while stop.load(Ordering::SeqCst) {
        let mut publish_err: Option<anyhow::Error> = None;
        let signal = stream.capture_into(1000, &mut |bgra| {
            let dims = (bgra.width, bgra.height);
            if verbose && dims != last_dims {
                eprintln!("frame {}x{} stride={}", dims.0, dims.1, bgra.stride);
                last_dims = dims;
            }
            match handle_video_frame(out, &bgra) {
                Ok(true) => {
                    frames.fetch_add(1, Ordering::SeqCst);
                }
                Ok(false) => {
                    if verbose {
                        eprintln!("skipping malformed frame");
                    }
                }
                Err(e) => publish_err = Some(e),
            }
        });
        match signal {
            CaptureSignal::Got => {
                if let Some(e) = publish_err {
                    eprintln!("publish error: {e}");
                }
            }
            CaptureSignal::Error => eprintln!("NDI capture error"),
            CaptureSignal::Idle => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records how many frames a fake backend received.
    struct MockOutput {
        published: usize,
    }
    impl SharedTextureOutput for MockOutput {
        fn publish(&mut self, _frame: &BgraFrame) -> anyhow::Result<()> {
            self.published += 1;
            Ok(())
        }
    }

    #[test]
    fn bounds_rejects_short_buffer() {
        // 40 bytes/row * 10 rows = 400 needed
        assert!(!frame_within_bounds(399, 40, 10));
        assert!(frame_within_bounds(400, 40, 10));
    }

    #[test]
    fn bounds_does_not_overflow() {
        // saturating_mul keeps this from panicking on huge dims
        assert!(!frame_within_bounds(0, u32::MAX, u32::MAX));
    }

    #[test]
    fn publishes_valid_frame() {
        let mut out = MockOutput { published: 0 };
        let data = vec![0u8; 16]; // 2x2, stride 8 -> 16 bytes
        let frame = BgraFrame { data: &data, width: 2, height: 2, stride: 8 };
        assert!(handle_video_frame(&mut out, &frame).unwrap());
        assert_eq!(out.published, 1);
    }

    #[test]
    fn skips_malformed_frame() {
        let mut out = MockOutput { published: 0 };
        let data = vec![0u8; 8]; // needs 16, has 8
        let frame = BgraFrame { data: &data, width: 2, height: 2, stride: 8 };
        assert!(!handle_video_frame(&mut out, &frame).unwrap());
        assert_eq!(out.published, 0);
    }

    use std::cell::Cell;

    /// Yields `target` frames, then sets `stop` false and reports Idle.
    struct MockStream<'s> {
        target: u32,
        seen: Cell<u32>,
        stop: &'s AtomicBool,
        buf: Vec<u8>,
    }
    impl<'s> FrameStream for MockStream<'s> {
        fn capture_into(
            &self,
            _timeout_ms: u32,
            on_frame: &mut dyn FnMut(BgraFrame),
        ) -> CaptureSignal {
            if self.seen.get() < self.target {
                self.seen.set(self.seen.get() + 1);
                on_frame(BgraFrame { data: &self.buf, width: 2, height: 2, stride: 8 });
                CaptureSignal::Got
            } else {
                self.stop.store(false, Ordering::SeqCst);
                CaptureSignal::Idle
            }
        }
    }

    #[test]
    fn loop_stops_and_counts_frames() {
        let stop = AtomicBool::new(true);
        let frames = AtomicU64::new(0);
        let mut out = MockOutput { published: 0 };
        let stream = MockStream {
            target: 3,
            seen: Cell::new(0),
            stop: &stop,
            buf: vec![0u8; 16],
        };
        run_capture_loop(&stream, &mut out, &stop, &frames, false).unwrap();
        assert_eq!(frames.load(Ordering::SeqCst), 3);
        assert_eq!(out.published, 3);
    }
}
