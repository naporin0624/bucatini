pub mod sys;

use anyhow::{anyhow, Result};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

pub struct Ndi {
    _private: (),
}

impl Ndi {
    pub fn new() -> Result<Ndi> {
        if unsafe { sys::NDIlib_initialize() } {
            Ok(Ndi { _private: () })
        } else {
            Err(anyhow!(
                "NDIlib_initialize failed (libndi present but CPU unsupported?)"
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub url: String,
}

pub struct Finder<'a> {
    handle: sys::NDIlib_find_instance_t,
    _ndi: &'a Ndi,
}

impl<'a> Finder<'a> {
    pub fn new(ndi: &'a Ndi) -> Result<Finder<'a>> {
        let create = sys::NDIlib_find_create_t {
            show_local_sources: true,
            p_groups: ptr::null(),
            p_extra_ips: ptr::null(),
        };
        let handle = unsafe { sys::NDIlib_find_create_v2(&create) };
        if handle.is_null() {
            return Err(anyhow!("NDIlib_find_create_v2 returned null"));
        }
        Ok(Finder { handle, _ndi: ndi })
    }

    pub fn list(&self, timeout_ms: u32) -> Vec<Source> {
        unsafe { sys::NDIlib_find_wait_for_sources(self.handle, timeout_ms) };
        let mut count: u32 = 0;
        let ptr = unsafe { sys::NDIlib_find_get_current_sources(self.handle, &mut count) };
        if ptr.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(ptr, count as usize) };
        slice
            .iter()
            .map(|s| Source {
                name: cstr_to_string(s.p_ndi_name),
                url: cstr_to_string(s.p_url_address),
            })
            .collect()
    }
}

impl Drop for Finder<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_find_destroy(self.handle) };
    }
}

pub struct Receiver<'a> {
    handle: sys::NDIlib_recv_instance_t,
    _ndi: &'a Ndi,
}

impl<'a> Receiver<'a> {
    pub fn new(ndi: &'a Ndi, source: &Source, recv_name: &str) -> Result<Receiver<'a>> {
        let c_name = std::ffi::CString::new(source.name.clone())?;
        let c_url = std::ffi::CString::new(source.url.clone())?;
        let c_recv = std::ffi::CString::new(recv_name)?;
        let create = sys::NDIlib_recv_create_v3_t {
            source_to_connect_to: sys::NDIlib_source_t {
                p_ndi_name: c_name.as_ptr(),
                p_url_address: c_url.as_ptr(),
            },
            color_format: sys::NDIlib_recv_color_format_BGRX_BGRA,
            bandwidth: sys::NDIlib_recv_bandwidth_highest,
            allow_video_fields: false,
            p_ndi_recv_name: c_recv.as_ptr(),
        };
        let handle = unsafe { sys::NDIlib_recv_create_v3(&create) };
        if handle.is_null() {
            return Err(anyhow!("NDIlib_recv_create_v3 returned null"));
        }
        // c_name/c_url/c_recv are copied by the SDK during create; safe to drop now.
        Ok(Receiver { handle, _ndi: ndi })
    }

    pub fn capture(&self, timeout_ms: u32) -> CaptureResult<'_> {
        let mut frame: sys::NDIlib_video_frame_v2_t = unsafe { std::mem::zeroed() };
        let t = unsafe {
            sys::NDIlib_recv_capture_v2(
                self.handle,
                &mut frame,
                ptr::null_mut(),
                ptr::null_mut(),
                timeout_ms,
            )
        };
        match t {
            sys::NDIlib_frame_type_video => CaptureResult::Video(VideoFrame {
                receiver: self.handle,
                frame,
                _marker: std::marker::PhantomData,
            }),
            sys::NDIlib_frame_type_error => CaptureResult::Error,
            _ => CaptureResult::None,
        }
    }
}

impl Drop for Receiver<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_recv_destroy(self.handle) };
    }
}

pub enum CaptureResult<'a> {
    Video(VideoFrame<'a>),
    None,
    Error,
}

pub struct VideoFrame<'a> {
    receiver: sys::NDIlib_recv_instance_t,
    frame: sys::NDIlib_video_frame_v2_t,
    _marker: std::marker::PhantomData<&'a Receiver<'a>>,
}

impl VideoFrame<'_> {
    pub fn width(&self) -> u32 {
        self.frame.xres as u32
    }
    pub fn height(&self) -> u32 {
        self.frame.yres as u32
    }
    pub fn stride(&self) -> u32 {
        self.frame.line_stride_or_size as u32
    }
    pub fn data(&self) -> &[u8] {
        let len = self.stride() as usize * self.height() as usize;
        unsafe { std::slice::from_raw_parts(self.frame.p_data, len) }
    }
}

impl Drop for VideoFrame<'_> {
    fn drop(&mut self) {
        unsafe { sys::NDIlib_recv_free_video_v2(self.receiver, &self.frame) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn cstr_to_string_reads_valid() {
        let c = CString::new("STUDIO (Cam 1)").unwrap();
        assert_eq!(cstr_to_string(c.as_ptr()), "STUDIO (Cam 1)");
    }

    #[test]
    fn cstr_to_string_null_is_empty() {
        assert_eq!(cstr_to_string(std::ptr::null()), "");
    }
}
