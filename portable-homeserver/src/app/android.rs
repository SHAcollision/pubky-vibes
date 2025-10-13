use std::{env, fs, path::PathBuf, sync::OnceLock};

use anyhow::{Context, Result};
use jni::{
    JavaVM,
    objects::{JObject, JString},
};
use tracing::{debug, warn};

static INTERNAL_BASE_DIR: OnceLock<PathBuf> = OnceLock::new();
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

fn try_resolve_internal_base_dir() -> Result<PathBuf> {
    let android_context = ndk_context::android_context();
    let java_vm = unsafe { JavaVM::from_raw(android_context.vm().cast()) }
        .context("Failed to acquire JavaVM from Android context")?;
    let env = java_vm
        .attach_current_thread()
        .context("Failed to attach JNI thread for Android context")?;

    let ctx_object = unsafe { JObject::from_raw(android_context.context() as jni::sys::jobject) };

    let files_dir = env
        .call_method(&ctx_object, "getFilesDir", "()Ljava/io/File;", &[])
        .context("Calling android.content.Context.getFilesDir()")?
        .l()
        .context("Context.getFilesDir() returned null")?;

    let absolute_path = env
        .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .context("Calling java.io.File.getAbsolutePath()")?
        .l()
        .context("File.getAbsolutePath() returned null")?;

    let path: String = env
        .get_string(JString::from(absolute_path))
        .context("Reading files directory path from Java string")?
        .into();

    // Prevent the activity reference from being released when `ctx_object` is dropped.
    let _ = ctx_object.into_raw();

    Ok(PathBuf::from(path))
}

fn internal_base_dir() -> PathBuf {
    if let Some(existing) = INTERNAL_BASE_DIR.get() {
        return existing.clone();
    }

    match try_resolve_internal_base_dir() {
        Ok(path) => {
            let _ = INTERNAL_BASE_DIR.set(path.clone());
            path
        }
        Err(err) => {
            warn!(error = %err, "Falling back to temporary directory for Android storage");
            PathBuf::from("/data/local/tmp")
        }
    }
}

/// Ensure the environment variables and filesystem locations we rely on exist on Android.
///
/// Android applications run inside a sandboxed root without the traditional Unix environment
/// variables that desktop code expects. We derive the app-specific storage root from the
/// Android activity context provided by `ndk_context` and wire it up to the standard `HOME` and `TMPDIR`
/// environment variables that the homeserver stack uses indirectly (for `tempfile`, config
/// persistence, etc.).
pub(crate) fn ensure_android_environment() -> PathBuf {
    if let Some(existing) = APP_DATA_DIR.get() {
        return existing.clone();
    }

    let base = internal_base_dir();

    if INTERNAL_BASE_DIR.get().is_none() {
        // The Android context has not been initialized yet. Try again later.
        return base;
    }

    let home_before = env::var_os("HOME");
    if home_before.as_ref() != Some(base.as_os_str()) {
        debug!(path = %base.display(), "Setting HOME for Android runtime");
        env::set_var("HOME", &base);
    }

    let data_dir = base.join("pubky");
    if let Err(err) = fs::create_dir_all(&data_dir) {
        warn!(
            error = %err,
            path = %data_dir.display(),
            "Failed to create Android data directory"
        );
    }

    let tmp_dir = data_dir.join("tmp");
    match fs::create_dir_all(&tmp_dir) {
        Ok(()) => {
            let tmp_before = env::var_os("TMPDIR");
            if tmp_before.as_ref() != Some(tmp_dir.as_os_str()) {
                debug!(path = %tmp_dir.display(), "Setting TMPDIR for Android runtime");
                env::set_var("TMPDIR", &tmp_dir);
            }
        }
        Err(err) => warn!(
            error = %err,
            path = %tmp_dir.display(),
            "Failed to create Android tmp directory"
        ),
    }

    let _ = APP_DATA_DIR.set(data_dir.clone());
    data_dir
}

pub(crate) fn android_default_data_dir() -> PathBuf {
    ensure_android_environment()
}
