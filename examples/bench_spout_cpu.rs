//! CPU micro-benchmark for the Spout output path (Windows only).
//! A/B/C-compares the three CPU->Spout send strategies (StagingRowCopy,
//! SendImage, StagingSse) at 1080p and 4K, reporting per-frame wall/CPU time,
//! memcpy throughput, single-core CPU% at 60 fps, and CPU-time speedup vs the
//! original StagingRowCopy. Names the fastest-by-CPU strategy per resolution.
//! No NDI source required.

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("bench_spout_cpu is Windows-only (Spout backend).");
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use bucatini::output::spout::SendMode;

    let modes = [
        ("StagingRowCopy", SendMode::StagingRowCopy),
        ("SendImage", SendMode::SendImage),
        ("StagingSse", SendMode::StagingSse),
    ];

    // (label, width, height, frames, warmup)
    let cases = [
        ("1080p", 1920u32, 1080u32, 600u64, 60u64),
        ("4K", 3840u32, 2160u32, 300u64, 30u64),
    ];

    for (label, width, height, frames, warmup) in cases {
        bench_resolution(label, width, height, frames, warmup, &modes)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn bench_resolution(
    label: &str,
    width: u32,
    height: u32,
    frames: u64,
    warmup: u64,
    modes: &[(&str, bucatini::output::spout::SendMode)],
) -> anyhow::Result<()> {
    use bucatini::bench_metrics::{
        cpu_percent_at_fps, frame_bytes, per_frame_micros, speedup_ratio, throughput_mb_per_s,
    };
    use bucatini::output::BgraFrame;
    use std::time::Duration;

    let stride = width * 4;
    let bytes = frame_bytes(width, height);
    let target_fps = 60.0;

    // Non-uniform content so the memcpy isn't optimised against a zero page.
    let mut buf = vec![0u8; (stride as usize) * (height as usize)];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i & 0xff) as u8;
    }

    println!();
    println!("== Spout output CPU benchmark: {label} ==");
    println!(
        "resolution : {width}x{height}  ({:.2} MB/frame, BGRA)",
        bytes as f64 / 1e6
    );
    println!("frames     : {frames} (warmup {warmup})");
    println!(
        "{:<15} {:>12} {:>12} {:>12} {:>12} {:>12}",
        "mode", "wall us/f", "cpu us/f", "MB/s", "CPU%@60", "speedup"
    );

    // CPU time per frame for each mode; index 0 is the StagingRowCopy baseline.
    let mut baseline_cpu = Duration::ZERO;
    let mut best: Option<(&str, Duration)> = None;

    for (i, (name, mode)) in modes.iter().enumerate() {
        // Fresh sender per mode so state doesn't bleed between strategies.
        let mut out = bucatini::output::spout::SpoutOutput::new("bucatini-bench")?;
        let frame = BgraFrame {
            data: &buf,
            width,
            height,
            stride,
        };

        for _ in 0..warmup {
            publish(&mut out, &frame, *mode)?;
        }

        let cpu0 = thread_cpu_time();
        let wall0 = std::time::Instant::now();
        for _ in 0..frames {
            publish(&mut out, &frame, *mode)?;
        }
        let wall = wall0.elapsed();
        let cpu = thread_cpu_time().saturating_sub(cpu0);
        drop(out);

        let wall_us = per_frame_micros(wall, frames);
        let cpu_us = per_frame_micros(cpu, frames);
        let mb_s = throughput_mb_per_s(bytes, frames, wall);
        let cpu_per_frame = cpu.checked_div(frames as u32).unwrap_or(Duration::ZERO);
        let cpu_pct = cpu_percent_at_fps(cpu_per_frame, target_fps);

        if i == 0 {
            baseline_cpu = cpu;
        }
        let speedup = speedup_ratio(baseline_cpu, cpu);

        println!(
            "{:<15} {:>12.2} {:>12.2} {:>12.0} {:>12.2} {:>11.2}x",
            name, wall_us, cpu_us, mb_s, cpu_pct, speedup
        );

        match best {
            Some((_, best_cpu)) if best_cpu <= cpu => {}
            _ => best = Some((name, cpu)),
        }
    }

    if let Some((winner, _)) = best {
        println!("WINNER (lowest CPU): {winner}");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn publish(
    out: &mut bucatini::output::spout::SpoutOutput,
    frame: &bucatini::output::BgraFrame,
    mode: bucatini::output::spout::SendMode,
) -> anyhow::Result<()> {
    out.publish_with_mode(frame, mode)
}

/// User+kernel CPU time consumed by the *calling* thread so far.
#[cfg(target_os = "windows")]
fn thread_cpu_time() -> std::time::Duration {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Filetime {
        low: u32,
        high: u32,
    }
    extern "system" {
        fn GetCurrentThread() -> isize;
        fn GetThreadTimes(
            thread: isize,
            creation: *mut Filetime,
            exit: *mut Filetime,
            kernel: *mut Filetime,
            user: *mut Filetime,
        ) -> i32;
    }
    fn ticks(ft: &Filetime) -> u64 {
        ((ft.high as u64) << 32) | ft.low as u64
    } // 100-ns units
    let mut c = Filetime { low: 0, high: 0 };
    let mut e = c;
    let mut k = c;
    let mut u = c;
    unsafe {
        // Ignore the BOOL: on failure the fields stay zero, yielding a zero duration.
        let _ = GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u);
    }
    std::time::Duration::from_nanos((ticks(&k) + ticks(&u)) * 100)
}
