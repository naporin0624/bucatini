//! Capture loop core shared by the CLI and GUI.

use crate::output::{BgraFrame, SharedTextureOutput};

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
}
