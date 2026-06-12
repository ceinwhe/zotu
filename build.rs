fn main() {
    // Try pkg-config first
    if pkg_config::probe_library("libavcodec").is_ok() {
        pkg_config::probe_library("libavformat")
            .expect("libavformat dev headers not found (install libavformat-dev)");
        pkg_config::probe_library("libavutil")
            .expect("libavutil dev headers not found (install libavutil-dev)");
        pkg_config::probe_library("libswresample")
            .expect("libswresample dev headers not found (install libswresample-dev)");
        pkg_config::probe_library("libavfilter")
            .expect("libavfilter dev headers not found (install libavfilter-dev)");
    } else {
        // Fallback: try to find FFmpeg in standard locations
        println!(
            "cargo:warning=FFmpeg libraries not found via pkg-config. \
             Assuming libraries are on the system search path. \
             If linking fails, install FFmpeg dev packages (e.g. libavcodec-dev)."
        );
        println!("cargo:rustc-link-lib=avcodec");
        println!("cargo:rustc-link-lib=avformat");
        println!("cargo:rustc-link-lib=avutil");
        println!("cargo:rustc-link-lib=swresample");
        println!("cargo:rustc-link-lib=avfilter");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
