use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
    println!("cargo:rerun-if-env-changed=NDK_HOME");
    println!("cargo:rerun-if-env-changed=NDK_ROOT");
    println!("cargo:rerun-if-env-changed=NDKROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_SDK_ROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_HOME");
    println!("cargo:rerun-if-env-changed=NDK_HOST_TAG");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os != "android" || target_arch != "x86" {
        return;
    }

    let Some(ndk_root) = locate_ndk_root() else {
        println!("cargo:warning=Skipping libatomic search: ANDROID_NDK_HOME/NDK_HOME not set");
        return;
    };

    if let Some(lib_dir) = locate_libatomic_dir(&ndk_root) {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=atomic");
    } else {
        println!("cargo:warning=Unable to locate libatomic in {}", ndk_root.display());
    }
}

fn locate_ndk_root() -> Option<PathBuf> {
    const DIRECT_KEYS: &[&str] = &[
        "ANDROID_NDK_HOME",
        "ANDROID_NDK_ROOT",
        "NDK_HOME",
        "NDK_ROOT",
        "NDKROOT",
    ];

    for key in DIRECT_KEYS {
        if let Some(path) = env_path(key) {
            if path.is_dir() {
                return Some(path);
            }
        }
    }

    let sdk_keys = ["ANDROID_SDK_ROOT", "ANDROID_HOME"];
    for key in sdk_keys {
        if let Some(base) = env_path(key) {
            if let Some(ndk) = latest_subdir(base.join("ndk")) {
                return Some(ndk);
            }
        }
    }

    None
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

fn latest_subdir(dir: PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(&dir).ok()?;
    let mut dirs: Vec<(OsString, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| (entry.file_name(), entry.path()))
        .filter(|(_, path)| path.is_dir())
        .collect();

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    dirs.pop().map(|(_, path)| path)
}

fn locate_libatomic_dir(ndk_root: &Path) -> Option<PathBuf> {
    let prebuilt = ndk_root
        .join("toolchains")
        .join("llvm")
        .join("prebuilt");

    if !prebuilt.is_dir() {
        return None;
    }

    let host_tag = env::var("NDK_HOST_TAG").ok().or_else(|| detect_host_tag());

    let search_roots: Vec<PathBuf> = if let Some(tag) = host_tag {
        let tagged = prebuilt.join(&tag);
        if tagged.is_dir() {
            vec![tagged]
        } else {
            fs::read_dir(prebuilt)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_dir() { Some(path) } else { None }
                })
                .collect()
        }
    } else {
        fs::read_dir(prebuilt)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_dir() { Some(path) } else { None }
            })
            .collect()
    };

    for root in search_roots {
        if let Some(dir) = locate_libatomic_in_prebuilt(&root) {
            return Some(dir);
        }
    }

    None
}

fn locate_libatomic_in_prebuilt(root: &Path) -> Option<PathBuf> {
    let clang_root = root.join("lib").join("clang");
    let mut candidates = Vec::new();

    if let Ok(entries) = fs::read_dir(&clang_root) {
        for entry in entries.filter_map(Result::ok) {
            let version_path = entry.path();
            if !version_path.is_dir() {
                continue;
            }

            let lib_root = version_path.join("lib");
            if !lib_root.is_dir() {
                continue;
            }

            if let Ok(platforms) = fs::read_dir(&lib_root) {
                for platform in platforms.filter_map(Result::ok) {
                    let path = platform.path();
                    if path.is_dir() && path.join("libatomic.a").is_file() {
                        candidates.push(path);
                    }
                }
            }
        }
    }

    if candidates.is_empty() {
        let sysroot = root.join("sysroot").join("usr").join("lib").join("i686-linux-android");
        if let Ok(entries) = fs::read_dir(&sysroot) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() && path.join("libatomic.a").is_file() {
                    candidates.push(path);
                }
            }
        }
    }

    candidates.sort();
    candidates.pop()
}

fn detect_host_tag() -> Option<String> {
    let host = env::var("HOST").ok()?;
    let host = host.to_lowercase();

    let tag = if host.contains("linux") {
        "linux-x86_64"
    } else if host.contains("apple-darwin") {
        if host.contains("aarch64") || host.contains("arm") {
            "darwin-arm64"
        } else {
            "darwin-x86_64"
        }
    } else if host.contains("windows") {
        if host.contains("aarch64") || host.contains("arm") {
            "windows-arm64"
        } else {
            "windows-x86_64"
        }
    } else {
        return None;
    };

    Some(tag.to_string())
}
