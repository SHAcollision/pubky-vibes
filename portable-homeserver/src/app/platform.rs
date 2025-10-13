use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub(super) struct PlatformPaths {
    pub data_dir: PathBuf,
}

static PATHS: OnceLock<PlatformPaths> = OnceLock::new();

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(super) fn ensure_initialized() {
    let _ = paths();
}

pub(super) fn paths() -> &'static PlatformPaths {
    PATHS.get_or_init(|| {
        let paths = platform_paths();

        #[cfg(target_os = "android")]
        configure_android_environment(&paths);

        paths
    })
}

fn platform_paths() -> PlatformPaths {
    #[cfg(target_os = "android")]
    {
        android::paths()
    }

    #[cfg(not(target_os = "android"))]
    {
        desktop::paths()
    }
}

fn fallback_data_dir() -> PathBuf {
    let mut fallback = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    fallback.push(".pubky");
    fallback
}

#[cfg(not(target_os = "android"))]
mod desktop {
    use super::{PlatformPaths, fallback_data_dir};
    use directories::ProjectDirs;

    pub(super) fn paths() -> PlatformPaths {
        if let Some(project_dirs) = ProjectDirs::from("io", "Pubky", "PortableHomeserver") {
            PlatformPaths {
                data_dir: project_dirs.data_dir().to_path_buf(),
            }
        } else {
            PlatformPaths {
                data_dir: fallback_data_dir(),
            }
        }
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::{PlatformPaths, fallback_data_dir};
    use ndk::native_activity::NativeActivity;
    use ndk_context::android_context;
    use std::path::PathBuf;
    use std::ptr::NonNull;
    use tracing::warn;

    pub(super) fn paths() -> PlatformPaths {
        match android_internal_data_path() {
            Some(base) => PlatformPaths {
                data_dir: base.join("pubky"),
            },
            None => {
                warn!("Falling back to home directory for data storage on Android");
                PlatformPaths {
                    data_dir: fallback_data_dir(),
                }
            }
        }
    }

    fn android_internal_data_path() -> Option<PathBuf> {
        let context = android_context();
        let raw_ptr = context.context();
        if raw_ptr.is_null() {
            return None;
        }

        let activity_ptr = NonNull::new(raw_ptr.cast::<ndk_sys::ANativeActivity>())?;
        let activity = unsafe { NativeActivity::from_ptr(activity_ptr) };
        Some(activity.internal_data_path().to_path_buf())
    }
}

#[cfg(target_os = "android")]
fn configure_android_environment(paths: &PlatformPaths) {
    use std::fs;
    use tracing::warn;

    let temp_dir = paths
        .data_dir
        .parent()
        .map(|base| base.join("tmp"))
        .unwrap_or_else(|| paths.data_dir.join("tmp"));

    if let Err(err) = fs::create_dir_all(&temp_dir) {
        warn!(
            error = %err,
            path = %temp_dir.display(),
            "Failed to create Android temp directory"
        );
        return;
    }

    // SAFETY: On Android, modifying process environment variables is only safe
    // before other threads start interacting with the environment. The Android
    // launcher calls into [`ensure_initialized`] from the main thread before the
    // rest of the app boots, ensuring this runs during single-threaded
    // initialization.
    unsafe {
        std::env::set_var("TMPDIR", temp_dir.as_os_str());
    }
}
