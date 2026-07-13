#![allow(dead_code)]

#[cfg(target_os = "macos")]
pub mod syphon;

#[cfg(target_os = "windows")]
pub mod spout;

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

use anyhow::Result;

/// Construct the platform's shared-texture output backend.
#[cfg(target_os = "macos")]
pub fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(syphon::SyphonOutput::new(name)?))
}

#[cfg(target_os = "windows")]
pub fn make_output(name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Ok(Box::new(spout::SpoutOutput::new(name)?))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn make_output(_name: &str) -> Result<Box<dyn SharedTextureOutput>> {
    Err(anyhow::anyhow!(
        "no shared-texture backend on this platform (macOS=Syphon, Windows=Spout)"
    ))
}

/// The 4 BGRA bytes of pixel (x,y) in a tightly-or-padded BGRA buffer with
/// `stride` bytes/row.
pub fn bgra_at(buf: &[u8], x: u32, y: u32, stride: u32) -> [u8; 4] {
    let o = (y as usize) * (stride as usize) + (x as usize) * 4;
    [buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]
}

/// Human-facing name of the active shared-texture protocol.
pub fn output_kind() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Syphon"
    }
    #[cfg(target_os = "windows")]
    {
        "Spout"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "shared-texture"
    }
}

#[cfg(test)]
mod tests {
    use super::bgra_at;

    #[test]
    fn bgra_at_reads_the_right_pixel_with_padded_stride() {
        // 2x2 BGRA image with a padded stride of 12 bytes/row (8 used + 4 pad).
        // Pixel (x,y) is filled with a distinct BGRA quad so mis-indexing shows.
        let stride = 12u32;
        let mut buf = vec![0u8; (stride as usize) * 2];
        // row 0: (0,0)=[1,2,3,4] (1,0)=[5,6,7,8]
        buf[0..4].copy_from_slice(&[1, 2, 3, 4]);
        buf[4..8].copy_from_slice(&[5, 6, 7, 8]);
        // row 1: (0,1)=[9,10,11,12] (1,1)=[13,14,15,16]
        buf[12..16].copy_from_slice(&[9, 10, 11, 12]);
        buf[16..20].copy_from_slice(&[13, 14, 15, 16]);

        assert_eq!(bgra_at(&buf, 0, 0, stride), [1, 2, 3, 4]);
        assert_eq!(bgra_at(&buf, 1, 0, stride), [5, 6, 7, 8]);
        assert_eq!(bgra_at(&buf, 0, 1, stride), [9, 10, 11, 12]);
        assert_eq!(bgra_at(&buf, 1, 1, stride), [13, 14, 15, 16]);
    }
}
