use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

const BRIDGE_SOURCE: &str = "src/audio/ffmpeg/bridge.c";
const LINK_LIBRARIES: [&str; 4] = ["avformat", "avcodec", "avutil", "swresample"];
const PKG_CONFIG_PACKAGES: [&str; 4] = ["libavformat", "libavcodec", "libavutil", "libswresample"];

struct FfmpegInstallation {
    description: String,
    include_paths: Vec<PathBuf>,
    library_paths: Vec<PathBuf>,
    libraries: Vec<String>,
    framework_paths: Vec<PathBuf>,
    frameworks: Vec<String>,
}

impl FfmpegInstallation {
    fn emit_link_metadata(&self) {
        println!("cargo:warning=Using FFmpeg from {}", self.description);
        for path in &self.library_paths {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        for path in &self.framework_paths {
            println!("cargo:rustc-link-search=framework={}", path.display());
        }
        for library in &self.libraries {
            println!("cargo:rustc-link-lib={library}");
        }
        for framework in &self.frameworks {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={BRIDGE_SOURCE}");
    for variable in [
        "ZOTU_FFMPEG_DIR",
        "FFMPEG_DIR",
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_ALLOW_CROSS",
        "PATH",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let installation = if let Some(root) = configured_root() {
        configure_root(&root)
    } else if let Some(installation) = configure_pkg_config() {
        installation
    } else if let Some(root) = root_from_path() {
        configure_root(&root)
    } else {
        panic!(
            "FFmpeg development files were not found. Set ZOTU_FFMPEG_DIR (or FFMPEG_DIR) to an FFmpeg directory containing include/ and lib/, install FFmpeg pkg-config files, or add an FFmpeg bin directory to PATH."
        );
    };

    let mut bridge = cc::Build::new();
    bridge.file(BRIDGE_SOURCE).warnings(true);
    for include_path in &installation.include_paths {
        bridge.include(include_path);
    }
    bridge.compile("zotu_ffmpeg_bridge");

    // Native dependencies must follow the static bridge on linkers where order matters.
    installation.emit_link_metadata();
}

fn configured_root() -> Option<PathBuf> {
    ["ZOTU_FFMPEG_DIR", "FFMPEG_DIR"]
        .into_iter()
        .find_map(|name| {
            env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn configure_root(root: &Path) -> FfmpegInstallation {
    let include_path = root.join("include");
    let library_path = root.join("lib");
    let required_header = include_path.join("libavformat").join("avformat.h");

    if !required_header.is_file() || !library_path.is_dir() {
        panic!(
            "Invalid FFmpeg directory '{}': expected '{}' and '{}'.",
            root.display(),
            required_header.display(),
            library_path.display()
        );
    }

    FfmpegInstallation {
        description: root.display().to_string(),
        include_paths: vec![include_path],
        library_paths: vec![library_path],
        libraries: LINK_LIBRARIES.iter().map(ToString::to_string).collect(),
        framework_paths: Vec::new(),
        frameworks: Vec::new(),
    }
}

fn configure_pkg_config() -> Option<FfmpegInstallation> {
    let mut include_paths = BTreeSet::new();
    let mut library_paths = Vec::new();
    let mut libraries = Vec::new();
    let mut framework_paths = Vec::new();
    let mut frameworks = Vec::new();

    for package in PKG_CONFIG_PACKAGES {
        let library = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe(package)
            .ok()?;
        include_paths.extend(library.include_paths);
        extend_unique(&mut library_paths, library.link_paths);
        extend_unique(&mut libraries, library.libs);
        extend_unique(&mut framework_paths, library.framework_paths);
        extend_unique(&mut frameworks, library.frameworks);
    }

    Some(FfmpegInstallation {
        description: "pkg-config".to_string(),
        include_paths: include_paths.into_iter().collect(),
        library_paths,
        libraries,
        framework_paths,
        frameworks,
    })
}

fn extend_unique<T: PartialEq>(target: &mut Vec<T>, values: impl IntoIterator<Item = T>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn root_from_path() -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .and_then(|executable| executable.parent()?.parent().map(Path::to_path_buf))
        .filter(|root| {
            root.join("include/libavformat/avformat.h").is_file() && root.join("lib").is_dir()
        })
}
