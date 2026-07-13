#![allow(dead_code)]

use super::{BgraFrame, SharedTextureOutput};
use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

type SpoutBridgeHandle = *mut c_void;

extern "C" {
    fn spout_bridge_create(name: *const c_char) -> SpoutBridgeHandle;
    fn spout_bridge_destroy(handle: SpoutBridgeHandle);
    fn spout_bridge_send_rgba(
        handle: SpoutBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
    ) -> i32;
    fn spout_bridge_send_rgba_mode(
        handle: SpoutBridgeHandle,
        data: *const u8,
        width: u32,
        height: u32,
        bytes_per_row: u32,
        mode: u32,
    ) -> i32;
}

type SpoutRecvHandle = *mut c_void;

extern "C" {
    fn spout_recv_create() -> SpoutRecvHandle;
    fn spout_recv_destroy(handle: SpoutRecvHandle);
    fn spout_recv_grab(
        handle: SpoutRecvHandle,
        name: *const c_char,
        buf: *mut u8,
        max_w: u32,
        max_h: u32,
        out_w: *mut u32,
        out_h: *mut u32,
        timeout_ms: u32,
    ) -> i32;
}

/// CPU->Spout send strategy, selectable for benchmarking. The numeric mapping
/// matches the `mode` argument the C++ bridge dispatches on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendMode {
    /// DYNAMIC staging + row-by-row memcpy + SendTexture (original path).
    StagingRowCopy,
    /// SendImage: UpdateSubresource straight into the shared texture (no staging).
    SendImage,
    /// DYNAMIC staging + SpoutCopy SSE2 (pitch-aware) + SendTexture.
    StagingSse,
}

impl SendMode {
    pub fn as_u32(self) -> u32 {
        match self {
            SendMode::StagingRowCopy => 0,
            SendMode::SendImage => 1,
            SendMode::StagingSse => 2,
        }
    }
}

pub struct SpoutOutput {
    handle: SpoutBridgeHandle,
}

impl SpoutOutput {
    pub fn new(name: &str) -> Result<SpoutOutput> {
        let c_name = CString::new(name)?;
        let handle = unsafe { spout_bridge_create(c_name.as_ptr()) };
        if handle.is_null() {
            return Err(anyhow!("spout_bridge_create failed (DirectX11/Spout init)"));
        }
        Ok(SpoutOutput { handle })
    }

    /// Publish via a specific send strategy. Used by the CPU benchmark to A/B/C
    /// the strategies; production `publish` uses the shipped winner.
    pub fn publish_with_mode(&mut self, frame: &BgraFrame, mode: SendMode) -> Result<()> {
        let rc = unsafe {
            spout_bridge_send_rgba_mode(
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
            Err(anyhow!("spout_bridge_send_rgba_mode returned {rc}"))
        }
    }
}

impl SharedTextureOutput for SpoutOutput {
    fn publish(&mut self, frame: &BgraFrame) -> Result<()> {
        let rc = unsafe {
            spout_bridge_send_rgba(
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
            Err(anyhow!("spout_bridge_send_rgba returned {rc}"))
        }
    }
}

impl Drop for SpoutOutput {
    fn drop(&mut self) {
        unsafe { spout_bridge_destroy(self.handle) };
    }
}

/// A Spout receiver that reads a named sender's shared texture back into a CPU
/// BGRA buffer. Used by the color-verification harness to prove the send path.
pub struct SpoutReceiver {
    handle: SpoutRecvHandle,
}

impl SpoutReceiver {
    pub fn new() -> Result<SpoutReceiver> {
        let handle = unsafe { spout_recv_create() };
        if handle.is_null() {
            return Err(anyhow!("spout_recv_create failed (DirectX11 init)"));
        }
        Ok(SpoutReceiver { handle })
    }

    /// Poll up to `timeout_ms` for a frame from sender `name`. On success returns
    /// `(buf, width, height)` where `buf` is tightly packed BGRA (stride = w*4).
    pub fn grab(
        &mut self,
        name: &str,
        timeout_ms: u32,
        max_w: u32,
        max_h: u32,
    ) -> Result<(Vec<u8>, u32, u32)> {
        let c_name = CString::new(name)?;
        let mut buf = vec![0u8; (max_w as usize) * (max_h as usize) * 4];
        let mut out_w: u32 = 0;
        let mut out_h: u32 = 0;
        let rc = unsafe {
            spout_recv_grab(
                self.handle,
                c_name.as_ptr(),
                buf.as_mut_ptr(),
                max_w,
                max_h,
                &mut out_w,
                &mut out_h,
                timeout_ms,
            )
        };
        match rc {
            0 => {
                // ReceiveImage packs tightly at the sender width, so shrink the
                // buffer to exactly out_w*out_h*4 (stride == out_w*4).
                buf.truncate((out_w as usize) * (out_h as usize) * 4);
                Ok((buf, out_w, out_h))
            }
            -2 => Err(anyhow!(
                "sender {out_w}x{out_h} exceeds receiver capacity {max_w}x{max_h}"
            )),
            _ => Err(anyhow!("spout_recv_grab timed out (no sender '{name}')")),
        }
    }
}

impl Drop for SpoutReceiver {
    fn drop(&mut self) {
        unsafe { spout_recv_destroy(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::SendMode;

    #[test]
    fn send_mode_maps_to_bridge_dispatch_codes() {
        assert_eq!(SendMode::StagingRowCopy.as_u32(), 0);
        assert_eq!(SendMode::SendImage.as_u32(), 1);
        assert_eq!(SendMode::StagingSse.as_u32(), 2);
    }
}
