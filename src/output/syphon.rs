#![allow(dead_code)]

use super::{BgraFrame, SharedTextureOutput};
use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

type SyphonBridgeHandle = *mut c_void;

extern "C" {
    fn syphon_bridge_create(name: *const c_char) -> SyphonBridgeHandle;
    fn syphon_bridge_destroy(handle: SyphonBridgeHandle);
    fn syphon_bridge_send_rgba(
        handle: SyphonBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
    ) -> i32;
    fn syphon_bridge_send_rgba_mode(
        handle: SyphonBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
        mode: u32,
    ) -> i32;
}

/// CPU->Syphon send strategy, selectable for benchmarking. The numeric mapping
/// matches the `mode` argument the ObjC++ bridge dispatches on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendMode {
    /// Per-frame IOSurface + MTLTexture allocation + row-by-row memcpy (original).
    PerFrameRowCopy,
    /// Cached IOSurface + MTLTexture reused across frames + row-by-row memcpy.
    PersistentRowCopy,
    /// Cached surface, single bulk memcpy when the frame stride equals the
    /// surface's row pitch, else falls back to row-by-row.
    PersistentBulkCopy,
}

impl SendMode {
    pub fn as_u32(self) -> u32 {
        match self {
            SendMode::PerFrameRowCopy => 0,
            SendMode::PersistentRowCopy => 1,
            SendMode::PersistentBulkCopy => 2,
        }
    }
}

pub struct SyphonOutput {
    handle: SyphonBridgeHandle,
}

impl SyphonOutput {
    pub fn new(name: &str) -> Result<SyphonOutput> {
        let c_name = CString::new(name)?;
        let handle = unsafe { syphon_bridge_create(c_name.as_ptr()) };
        if handle.is_null() {
            return Err(anyhow!("syphon_bridge_create failed (Metal/Syphon init)"));
        }
        Ok(SyphonOutput { handle })
    }

    /// Publish via a specific send strategy. Used by the CPU benchmark to A/B/C
    /// the strategies; production `publish` uses the shipped winner.
    pub fn publish_with_mode(&mut self, frame: &BgraFrame, mode: SendMode) -> Result<()> {
        let rc = unsafe {
            syphon_bridge_send_rgba_mode(
                self.handle,
                frame.data.as_ptr(),
                frame.width,
                frame.height,
                frame.stride,
                mode.as_u32(),
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(anyhow!("syphon_bridge_send_rgba_mode returned {rc}"))
        }
    }
}

impl SharedTextureOutput for SyphonOutput {
    fn publish(&mut self, frame: &BgraFrame) -> Result<()> {
        let rc = unsafe {
            syphon_bridge_send_rgba(
                self.handle,
                frame.data.as_ptr(),
                frame.width,
                frame.height,
                frame.stride,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(anyhow!("syphon_bridge_send_rgba returned {rc}"))
        }
    }
}

impl Drop for SyphonOutput {
    fn drop(&mut self) {
        unsafe { syphon_bridge_destroy(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_mode_maps_to_bridge_dispatch_codes() {
        assert_eq!(SendMode::PerFrameRowCopy.as_u32(), 0);
        assert_eq!(SendMode::PersistentRowCopy.as_u32(), 1);
        assert_eq!(SendMode::PersistentBulkCopy.as_u32(), 2);
    }

    #[test]
    fn publishes_a_synthetic_frame() {
        let mut out = SyphonOutput::new("bucatini-test").expect("create server");
        let w = 16u32;
        let h = 16u32;
        let stride = w * 4;
        let data = vec![0u8; (stride * h) as usize];
        let frame = BgraFrame {
            data: &data,
            width: w,
            height: h,
            stride,
        };
        out.publish(&frame).expect("publish ok");
    }
}
