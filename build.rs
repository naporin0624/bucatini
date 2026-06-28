fn main() {
    #[cfg(target_os = "macos")]
    {
        link_ndi_macos();
        build_macos_syphon();
    }

    #[cfg(target_os = "windows")]
    {
        link_ndi_windows();
        build_windows_spout();
    }
}

// ---------------------------------------------------------------------------
// macOS: link the NDI runtime + build the Syphon Metal sender shim.
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn link_ndi_macos() {
    // Link the NDI runtime; search both the macOS default prefix and the
    // Homebrew arm64 prefix so the build works on machines where libndi
    // lives under either location.
    for dir in ["/usr/local/lib", "/opt/homebrew/lib"] {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
    println!("cargo:rustc-link-lib=dylib=ndi");
}

#[cfg(target_os = "macos")]
fn build_macos_syphon() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor = std::path::Path::new(&manifest).join("vendor");
    let vendor_str = vendor.to_str().unwrap();

    println!("cargo:rerun-if-changed=vendor/cpp/syphon_bridge.mm");
    println!("cargo:rerun-if-changed=vendor/cpp/syphon_bridge.h");

    cc::Build::new()
        .file("vendor/cpp/syphon_bridge.mm")
        .include("vendor/cpp")
        .flag("-ObjC++")
        .flag("-std=c++17")
        .flag("-fobjc-arc")
        .flag("-F")
        .flag(vendor_str)
        .compile("syphon_bridge");

    println!("cargo:rustc-link-lib=c++");
    println!("cargo:rustc-link-lib=framework=Syphon");
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=IOSurface");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rustc-link-lib=framework=QuartzCore");
    println!("cargo:rustc-link-search=framework={vendor_str}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{vendor_str}");
}

// ---------------------------------------------------------------------------
// Windows: link the NDI import library + build the Spout (SpoutDX) sender shim.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn link_ndi_windows() {
    // The NDI SDK ships an import library `Processing.NDI.Lib.x64.lib` under
    // `<NDI SDK>/Lib/x64`. Its location is taken from the `NDI_SDK_DIR`
    // environment variable (set by the SDK installer); fall back to the
    // standard NDI 6 install path.
    println!("cargo:rerun-if-env-changed=NDI_SDK_DIR");
    let sdk_dir = std::env::var("NDI_SDK_DIR")
        .unwrap_or_else(|_| r"C:\Program Files\NDI\NDI 6 SDK".to_string());
    let lib_dir = std::path::Path::new(&sdk_dir).join("Lib").join("x64");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=Processing.NDI.Lib.x64");

    // Delay-load the NDI DLL so the loader does not resolve
    // `Processing.NDI.Lib.x64.dll` at process startup. `ndi::dll` adds the NDI
    // runtime directory (NDI_RUNTIME_DIR_Vx) to the search path before the first
    // NDI call, so the app runs without copying the DLL next to the executable.
    println!("cargo:rustc-link-arg=/DELAYLOAD:Processing.NDI.Lib.x64.dll");
    println!("cargo:rustc-link-lib=dylib=delayimp");
}

#[cfg(target_os = "windows")]
fn build_windows_spout() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor = std::path::Path::new(&manifest).join("vendor");

    // Spout2 SDK is fetched into vendor/Spout2 (see scripts/fetch-spout2.ps1),
    // preserving the upstream layout because SpoutDX.h uses relative includes
    // (e.g. ../../SpoutGL/SpoutCommon.h).
    let spout2 = vendor.join("Spout2");
    let spout_dx = spout2.join("SpoutDirectX").join("SpoutDX");
    let spout_gl = spout2.join("SpoutGL");

    println!("cargo:rerun-if-changed=vendor/cpp/spout_bridge.cpp");
    println!("cargo:rerun-if-changed=vendor/cpp/spout_bridge.h");

    cc::Build::new()
        .cpp(true)
        .file("vendor/cpp/spout_bridge.cpp")
        .file(spout_dx.join("SpoutDX.cpp"))
        .file(spout_gl.join("SpoutDirectX.cpp"))
        .file(spout_gl.join("SpoutSenderNames.cpp"))
        .file(spout_gl.join("SpoutFrameCount.cpp"))
        .file(spout_gl.join("SpoutUtils.cpp"))
        .file(spout_gl.join("SpoutCopy.cpp"))
        .file(spout_gl.join("SpoutSharedMemory.cpp"))
        .include(&spout_dx)
        .include(&spout_gl)
        .include("vendor/cpp")
        .flag("/EHsc")
        .flag("/std:c++17")
        .compile("spout_bridge");

    for lib in [
        "d3d11", "dxgi", "user32", "gdi32", "shell32", "ole32", "comdlg32", "comctl32", "shlwapi",
    ] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
