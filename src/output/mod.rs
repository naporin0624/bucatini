#![allow(dead_code)]

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
