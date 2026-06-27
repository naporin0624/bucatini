fn main() {
    // Link the Homebrew-installed NDI runtime.
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=dylib=ndi");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/local/lib");
}
