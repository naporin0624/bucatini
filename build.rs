fn main() {
    // Link the NDI runtime; search both the macOS default prefix and the
    // Homebrew arm64 prefix so the build works on machines where libndi
    // lives under either location.
    for dir in ["/usr/local/lib", "/opt/homebrew/lib"] {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
    println!("cargo:rustc-link-lib=dylib=ndi");

    #[cfg(target_os = "macos")]
    build_macos();
}

#[cfg(target_os = "macos")]
fn build_macos() {
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
