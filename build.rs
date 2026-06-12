fn main() {
    // Try pkg-config first
    if pkg_config::probe_library("libavcodec").is_ok() {
        pkg_config::probe_library("libavformat").unwrap();
        pkg_config::probe_library("libavutil").unwrap();
        pkg_config::probe_library("libswresample").unwrap();
        pkg_config::probe_library("libavfilter").unwrap();
    } else {
        // Fallback: try to find FFmpeg in standard locations
        println!("cargo:rustc-link-lib=avcodec");
        println!("cargo:rustc-link-lib=avformat");
        println!("cargo:rustc-link-lib=avutil");
        println!("cargo:rustc-link-lib=swresample");
        println!("cargo:rustc-link-lib=avfilter");
    }
}
