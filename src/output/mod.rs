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
