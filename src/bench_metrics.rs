//! Pure metric calculations for the Spout-output CPU benchmark, kept free of
//! platform/FFI code so they are unit-testable on every target.

use std::time::Duration;

/// Bytes in a tightly-packed BGRA frame (4 bytes per pixel).
pub fn frame_bytes(width: u32, height: u32) -> u64 {
    (width as u64) * (height as u64) * 4
}

/// Mean time for one frame, in microseconds. Returns 0.0 when `frames` is 0.
pub fn per_frame_micros(total: Duration, frames: u64) -> f64 {
    if frames == 0 {
        return 0.0;
    }
    total.as_secs_f64() * 1e6 / frames as f64
}

/// memcpy throughput in MB/s (MB = 1_000_000 bytes). Returns 0.0 when elapsed <= 0.
pub fn throughput_mb_per_s(frame_bytes: u64, frames: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    (frame_bytes as f64) * (frames as f64) / secs / 1e6
}

/// Estimated CPU utilisation (%) of one core when `cpu_per_frame` is spent every frame at `fps`.
pub fn cpu_percent_at_fps(cpu_per_frame: Duration, fps: f64) -> f64 {
    cpu_per_frame.as_secs_f64() * fps * 100.0
}

/// How many times faster `candidate` is than `baseline` (>1.0 = candidate faster).
/// Returns 0.0 if candidate is zero-length.
pub fn speedup_ratio(baseline: Duration, candidate: Duration) -> f64 {
    let c = candidate.as_secs_f64();
    if c <= 0.0 {
        return 0.0;
    }
    baseline.as_secs_f64() / c
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn frame_bytes_1080p_is_four_bytes_per_pixel() {
        assert_eq!(frame_bytes(1920, 1080), 8_294_400);
    }

    #[test]
    fn per_frame_micros_averages_over_frames() {
        assert!((per_frame_micros(Duration::from_millis(600), 600) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn per_frame_micros_zero_frames_is_zero() {
        assert_eq!(per_frame_micros(Duration::from_secs(1), 0), 0.0);
    }

    #[test]
    fn throughput_scales_with_bytes_and_frames() {
        assert!(
            (throughput_mb_per_s(1_000_000, 1000, Duration::from_secs(1)) - 1000.0).abs() < 1e-6
        );
    }

    #[test]
    fn throughput_zero_elapsed_is_zero() {
        assert_eq!(throughput_mb_per_s(8_294_400, 100, Duration::ZERO), 0.0);
    }

    #[test]
    fn cpu_percent_at_fps_scales_with_time_and_rate() {
        assert!((cpu_percent_at_fps(Duration::from_micros(1000), 60.0) - 6.0).abs() < 1e-6);
    }

    #[test]
    fn speedup_ratio_faster_candidate_is_above_one() {
        assert!(
            (speedup_ratio(Duration::from_millis(2), Duration::from_millis(1)) - 2.0).abs() < 1e-6
        );
    }

    #[test]
    fn speedup_ratio_slower_candidate_is_below_one() {
        assert!(
            (speedup_ratio(Duration::from_millis(1), Duration::from_millis(2)) - 0.5).abs() < 1e-6
        );
    }

    #[test]
    fn speedup_ratio_zero_candidate_is_zero() {
        assert_eq!(speedup_ratio(Duration::from_millis(1), Duration::ZERO), 0.0);
    }
}
