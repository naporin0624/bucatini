//! CPU micro-benchmark for the Syphon output path (macOS only).
//! Measures the current `SyphonOutput::publish` baseline at 1080p and 4K,
//! reporting per-frame wall/CPU time, memcpy throughput, and single-core
//! CPU% at 60 fps. No NDI source required.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("bench_syphon_cpu is macOS-only (Syphon backend).");
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    // (label, width, height, frames, warmup)
    let cases = [
        ("1080p", 1920u32, 1080u32, 600u64, 60u64),
        ("4K", 3840u32, 2160u32, 300u64, 30u64),
    ];

    for (label, width, height, frames, warmup) in cases {
        bench_resolution(label, width, height, frames, warmup)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn bench_resolution(
    label: &str,
    width: u32,
    height: u32,
    frames: u64,
    warmup: u64,
) -> anyhow::Result<()> {
    use bucatini::bench_metrics::{
        cpu_percent_at_fps, frame_bytes, per_frame_micros, throughput_mb_per_s,
    };
    use bucatini::output::syphon::SyphonOutput;
    use bucatini::output::{BgraFrame, SharedTextureOutput};
    use std::time::{Duration, Instant};

    let stride = width * 4;
    let bytes = frame_bytes(width, height);
    let target_fps = 60.0;

    // Non-uniform content so the memcpy isn't optimised against a zero page.
    let mut buf = vec![0u8; (stride as usize) * (height as usize)];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i & 0xff) as u8;
    }

    println!();
    println!("== Syphon output CPU benchmark: {label} ==");
    println!(
        "resolution : {width}x{height}  ({:.2} MB/frame, BGRA)",
        bytes as f64 / 1e6
    );
    println!("frames     : {frames} (warmup {warmup})");
    println!(
        "{:<15} {:>12} {:>12} {:>12} {:>12}",
        "mode", "wall us/f", "cpu us/f", "MB/s", "CPU%@60"
    );

    let mut out = SyphonOutput::new("bucatini-bench")?;
    let frame = BgraFrame {
        data: &buf,
        width,
        height,
        stride,
    };

    for _ in 0..warmup {
        out.publish(&frame)?;
    }

    let cpu0 = thread_cpu_time();
    let wall0 = Instant::now();
    for _ in 0..frames {
        out.publish(&frame)?;
    }
    let wall = wall0.elapsed();
    let cpu = thread_cpu_time().saturating_sub(cpu0);
    drop(out);

    let wall_us = per_frame_micros(wall, frames);
    let cpu_us = per_frame_micros(cpu, frames);
    let mb_s = throughput_mb_per_s(bytes, frames, wall);
    let cpu_per_frame = cpu.checked_div(frames as u32).unwrap_or(Duration::ZERO);
    let cpu_pct = cpu_percent_at_fps(cpu_per_frame, target_fps);

    println!(
        "{:<15} {:>12.2} {:>12.2} {:>12.0} {:>12.2}",
        "baseline", wall_us, cpu_us, mb_s, cpu_pct
    );

    Ok(())
}

/// User+kernel CPU time consumed by the *calling* thread so far.
#[cfg(target_os = "macos")]
fn thread_cpu_time() -> std::time::Duration {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        // Ignore the return code: on failure the fields stay zero, yielding a zero duration.
        let _ = libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts);
    }
    std::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}
