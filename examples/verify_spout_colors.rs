//! End-to-end color verification for the Spout output path (Windows only).
//!
//! Publishes a 256x256 BGRA test pattern via the production send path
//! (SpoutOutput::publish -> StagingSse / mode 2 / spoutCopy::rgba2rgba), then
//! RECEIVES the frame back through a real Spout receiver in the same process and
//! asserts the four corner quadrants come back with the exact colors in the
//! exact positions. This proves there is no R/B channel swap, no corruption, and
//! no vertical flip. The received frame is saved as a PNG for visual inspection.
//!
//! Exits nonzero if verification fails, so the run is unambiguous.

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("verify_spout_colors is Windows-only (Spout backend).");
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use bucatini::output::spout::{SpoutOutput, SpoutReceiver};
    use bucatini::output::{bgra_at, BgraFrame, SharedTextureOutput};

    const W: u32 = 256;
    const H: u32 = 256;
    const STRIDE: u32 = W * 4; // tightly packed
    const NAME: &str = "bucatini-verify";

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Each quadrant's center sample point and its expected BGRA (byte order B,G,R,A).
    // TL=RED, TR=GREEN, BL=BLUE, BR=WHITE.
    let quadrants = [
        ("top-left  (RED)  ", 64u32, 64u32, [0u8, 0, 255, 255]),
        ("top-right (GREEN)", 192, 64, [0, 255, 0, 255]),
        ("bot-left  (BLUE) ", 64, 192, [255, 0, 0, 255]),
        ("bot-right (WHITE)", 192, 192, [255, 255, 255, 255]),
    ];

    // The Spout receive path needs a stream of new frames (its double-buffered
    // staging readback lags one frame). So publish continuously from a dedicated
    // thread while the receiver polls -- all FFI objects stay on one thread.
    let pattern = Arc::new(build_test_pattern(W, H));
    let stop = Arc::new(AtomicBool::new(false));
    let sender = {
        let pattern = Arc::clone(&pattern);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || -> anyhow::Result<()> {
            let mut out = SpoutOutput::new(NAME)?;
            while !stop.load(Ordering::Relaxed) {
                let frame = BgraFrame {
                    data: &pattern,
                    width: W,
                    height: H,
                    stride: STRIDE,
                };
                out.publish(&frame)?;
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(())
        })
    };

    let mut rx = SpoutReceiver::new()?;
    let received = rx.grab(NAME, 5000, W, H);
    stop.store(true, Ordering::Relaxed);
    match sender.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e.context("sender thread failed")),
        Err(_) => return Err(anyhow::anyhow!("sender thread panicked")),
    }

    let (buf, rw, rh) = received.map_err(|e| {
        e.context(format!(
            "receiver never captured a frame from '{NAME}' within 5s"
        ))
    })?;

    println!("received a frame: {rw}x{rh} (expected {W}x{H})");
    if rw != W || rh != H {
        return Err(anyhow::anyhow!("received size {rw}x{rh} != sent {W}x{H}"));
    }
    let rstride = rw * 4; // ReceiveImage packs tightly at sender width

    println!();
    println!(
        "{:<18} {:>18} {:>18}   result",
        "quadrant", "expected BGRA", "received BGRA"
    );
    let mut all_ok = true;
    let mut received_samples = Vec::new();
    for (label, x, y, expected) in quadrants {
        let got = bgra_at(&buf, x, y, rstride);
        received_samples.push(got);
        let ok = channels_match(&got, &expected);
        all_ok &= ok;
        println!(
            "{:<18} {:>18} {:>18}   {}",
            label,
            fmt_bgra(&expected),
            fmt_bgra(&got),
            if ok { "OK" } else { "MISMATCH" }
        );
    }

    // Classic failure-mode diagnosis.
    diagnose_failure_modes(&quadrants, &received_samples);

    // Save the received frame for visual inspection.
    let scratch = r"C:\Users\NAPOCH~1\AppData\Local\Temp\claude\C--Users-napochaan-Documents-ghq-bucatini\69ffa847-b787-4dab-94c8-bb5dc43a436d\scratchpad";
    let bmp_path = format!(r"{scratch}\spout_verify.bmp");
    write_bmp_24(&bmp_path, &buf, rw, rh)?;
    println!();
    println!("saved BMP: {bmp_path}");
    println!("(convert to PNG with: ffmpeg -y -i \"{bmp_path}\" \"{scratch}\\spout_verify.png\")");

    println!();
    if all_ok {
        println!("OVERALL: PASS -- all four quadrants match exactly in position and channels.");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "OVERALL: FAIL -- one or more quadrants did not match (see table above)"
        ))
    }
}

/// Build a 256x256 tightly-packed BGRA test pattern with 4 solid quadrants.
/// Byte order is B,G,R,A. TL=RED, TR=GREEN, BL=BLUE, BR=WHITE.
#[cfg(target_os = "windows")]
fn build_test_pattern(w: u32, h: u32) -> Vec<u8> {
    let stride = (w * 4) as usize;
    let mut buf = vec![0u8; stride * h as usize];
    for y in 0..h {
        for x in 0..w {
            let top = y < h / 2;
            let left = x < w / 2;
            let px = match (top, left) {
                (true, true) => [0u8, 0, 255, 255],     // RED
                (true, false) => [0, 255, 0, 255],      // GREEN
                (false, true) => [255, 0, 0, 255],      // BLUE
                (false, false) => [255, 255, 255, 255], // WHITE
            };
            let o = y as usize * stride + x as usize * 4;
            buf[o..o + 4].copy_from_slice(&px);
        }
    }
    buf
}

/// Match each channel within +-1 to tolerate any driver rounding (should be exact).
#[cfg(target_os = "windows")]
fn channels_match(a: &[u8; 4], b: &[u8; 4]) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| (*x as i32 - *y as i32).abs() <= 1)
}

#[cfg(target_os = "windows")]
fn fmt_bgra(p: &[u8; 4]) -> String {
    format!("[{},{},{},{}]", p[0], p[1], p[2], p[3])
}

/// Explicitly report the two classic failure modes so the diagnosis is obvious.
#[cfg(target_os = "windows")]
fn diagnose_failure_modes(quadrants: &[(&str, u32, u32, [u8; 4]); 4], got: &[[u8; 4]]) {
    // R/B swap: red corner reads as blue (and vice versa) -- i.e. each received
    // pixel equals its expected pixel with B and R channels exchanged.
    let rb_swap = quadrants.iter().zip(got.iter()).all(|((_, _, _, exp), g)| {
        let swapped = [exp[2], exp[1], exp[0], exp[3]];
        channels_match(&swapped, g)
    });
    if rb_swap {
        println!();
        println!(
            "DIAGNOSIS: R/B channel swap detected (each corner reads with red and blue exchanged)."
        );
    }

    // Vertical flip: top corners read as the bottom colors. got is ordered
    // [TL,TR,BL,BR]; a flip makes TL read BL's color, TR read BR's, etc.
    let flip_pairs = [(0usize, 2usize), (1, 3), (2, 0), (3, 1)];
    let vflip = flip_pairs.iter().all(|(i, j)| {
        let expected_other = quadrants[*j].3;
        channels_match(&expected_other, &got[*i])
    });
    if vflip {
        println!();
        println!("DIAGNOSIS: vertical flip detected (top corners read the bottom colors).");
    }
}

/// Write a 24-bit uncompressed BMP. BMP scanlines are stored bottom-up, so the
/// last image row is written first; each row is BGR and padded to a 4-byte
/// boundary. Source is tightly-packed BGRA (stride = w*4).
#[cfg(target_os = "windows")]
fn write_bmp_24(path: &str, buf: &[u8], w: u32, h: u32) -> anyhow::Result<()> {
    use std::io::Write;

    let row_unpadded = (w as usize) * 3;
    let pad = (4 - (row_unpadded % 4)) % 4;
    let row_padded = row_unpadded + pad;
    let pixel_bytes = row_padded * h as usize;
    let file_size = 14 + 40 + pixel_bytes;

    let mut out = Vec::with_capacity(file_size);

    // BITMAPFILEHEADER (14 bytes)
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset

    // BITMAPINFOHEADER (40 bytes)
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes()); // positive => bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (no compression)
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // ~72 DPI x
    out.extend_from_slice(&2835i32.to_le_bytes()); // ~72 DPI y
    out.extend_from_slice(&0u32.to_le_bytes()); // colors used
    out.extend_from_slice(&0u32.to_le_bytes()); // important colors

    let src_stride = (w * 4) as usize;
    let padding = [0u8; 3];
    for y in (0..h as usize).rev() {
        let row = &buf[y * src_stride..y * src_stride + src_stride];
        for x in 0..w as usize {
            // BGRA source -> BGR output (drop alpha).
            out.extend_from_slice(&row[x * 4..x * 4 + 3]);
        }
        out.extend_from_slice(&padding[..pad]);
    }

    let mut f = std::fs::File::create(path)?;
    f.write_all(&out)?;
    Ok(())
}
