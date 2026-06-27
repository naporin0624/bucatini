#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

pub type NDIlib_find_instance_t = *mut c_void;
pub type NDIlib_recv_instance_t = *mut c_void;

// color formats
pub const NDIlib_recv_color_format_BGRX_BGRA: c_int = 0;
// bandwidth
pub const NDIlib_recv_bandwidth_highest: c_int = 100;
// frame types
pub const NDIlib_frame_type_none: c_int = 0;
pub const NDIlib_frame_type_video: c_int = 1;
pub const NDIlib_frame_type_error: c_int = 4;

#[repr(C)]
pub struct NDIlib_source_t {
    pub p_ndi_name: *const c_char,
    /// union { p_url_address; p_ip_address } — single pointer
    pub p_url_address: *const c_char,
}

#[repr(C)]
pub struct NDIlib_find_create_t {
    pub show_local_sources: bool,
    pub p_groups: *const c_char,
    pub p_extra_ips: *const c_char,
}

#[repr(C)]
pub struct NDIlib_recv_create_v3_t {
    pub source_to_connect_to: NDIlib_source_t,
    pub color_format: c_int,
    pub bandwidth: c_int,
    pub allow_video_fields: bool,
    pub p_ndi_recv_name: *const c_char,
}

#[repr(C)]
pub struct NDIlib_video_frame_v2_t {
    pub xres: c_int,
    pub yres: c_int,
    pub four_cc: c_int,
    pub frame_rate_n: c_int,
    pub frame_rate_d: c_int,
    pub picture_aspect_ratio: f32,
    pub frame_format_type: c_int,
    pub timecode: i64,
    pub p_data: *mut u8,
    /// union { line_stride_in_bytes; data_size_in_bytes }
    pub line_stride_or_size: c_int,
    pub p_metadata: *const c_char,
    pub timestamp: i64,
}

extern "C" {
    pub fn NDIlib_initialize() -> bool;
    pub fn NDIlib_find_create_v2(p_create_settings: *const NDIlib_find_create_t) -> NDIlib_find_instance_t;
    pub fn NDIlib_find_destroy(p_instance: NDIlib_find_instance_t);
    pub fn NDIlib_find_get_current_sources(
        p_instance: NDIlib_find_instance_t,
        p_no_sources: *mut u32,
    ) -> *const NDIlib_source_t;
    pub fn NDIlib_find_wait_for_sources(
        p_instance: NDIlib_find_instance_t,
        timeout_in_ms: u32,
    ) -> bool;
    pub fn NDIlib_recv_create_v3(p_create_settings: *const NDIlib_recv_create_v3_t) -> NDIlib_recv_instance_t;
    pub fn NDIlib_recv_destroy(p_instance: NDIlib_recv_instance_t);
    pub fn NDIlib_recv_capture_v2(
        p_instance: NDIlib_recv_instance_t,
        p_video_data: *mut NDIlib_video_frame_v2_t,
        p_audio_data: *mut c_void,
        p_metadata: *mut c_void,
        timeout_in_ms: u32,
    ) -> c_int;
    pub fn NDIlib_recv_free_video_v2(
        p_instance: NDIlib_recv_instance_t,
        p_video_data: *const NDIlib_video_frame_v2_t,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_links_and_succeeds() {
        // Proves the dylib is linked and the symbol resolves at runtime.
        assert!(unsafe { NDIlib_initialize() });
    }
}
