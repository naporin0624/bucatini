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
