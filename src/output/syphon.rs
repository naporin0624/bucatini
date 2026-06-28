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
