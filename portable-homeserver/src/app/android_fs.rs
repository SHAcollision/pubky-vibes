use std::{env, fs, path::PathBuf, sync::OnceLock};

use anyhow::{Context, Result, anyhow};
use jni::{
    JavaVM,
    objects::{JObject, JString},
};

/// Cache of the Android context's files directory so we only query the JVM once.
static FILES_DIR: OnceLock<PathBuf> = OnceLock::new();
/// Cache of the Pubky-specific home directory inside the Android files directory.
static PUBKY_HOME: OnceLock<PathBuf> = OnceLock::new();

/// Prepare the Android filesystem layout used by the homeserver application.
///
/// This ensures we have a writable base directory, configures environment variables
/// (`HOME`, `PUBKY_HOME`, and the various temporary directory hints), and returns the
/// resolved Pubky home directory.
pub(crate) fn prepare_storage() -> Result<&'static PathBuf> {
    PUBKY_HOME.get_or_try_init(|| {
        let files_dir = android_files_dir()?.clone();
        fs::create_dir_all(&files_dir).with_context(|| {
            format!(
                "Failed to create Android files directory at {}",
                files_dir.display()
            )
        })?;

        let pubky_home = files_dir.join("pubky");
        fs::create_dir_all(&pubky_home).with_context(|| {
            format!(
                "Failed to create Pubky home directory at {}",
                pubky_home.display()
            )
        })?;

        let tmp_dir = pubky_home.join("tmp");
        fs::create_dir_all(&tmp_dir).with_context(|| {
            format!(
                "Failed to create temporary directory at {}",
                tmp_dir.display()
            )
        })?;

        env::set_var("HOME", &pubky_home);
        env::set_var("PUBKY_HOME", &pubky_home);
        env::set_var("TMPDIR", &tmp_dir);
        env::set_var("TMP", &tmp_dir);
        env::set_var("TEMP", &tmp_dir);

        Ok(pubky_home)
    })
}

/// Resolve the default data directory for the homeserver and ensure it exists.
pub(crate) fn default_data_dir() -> Result<String> {
    let pubky_home = prepare_storage()?.clone();
    let data_dir = pubky_home.join("portable-homeserver");
    fs::create_dir_all(&data_dir).with_context(|| {
        format!(
            "Failed to create homeserver data directory at {}",
            data_dir.display()
        )
    })?;

    Ok(data_dir.to_string_lossy().into_owned())
}

fn android_files_dir() -> Result<&'static PathBuf> {
    FILES_DIR.get_or_try_init(|| {
        let ctx = ndk_context::android_context();
        if ctx.vm().is_null() || ctx.context().is_null() {
            return Err(anyhow!("Android context is not available"));
        }

        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }
            .context("Failed to acquire Java VM from Android context")?;
        let env = vm
            .attach_current_thread()
            .context("Failed to attach the current thread to the Java VM")?;

        let context = unsafe { JObject::from_raw(ctx.context().cast()) };
        let files_dir = env
            .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])
            .context("Context.getFilesDir() call failed")?
            .l()
            .context("Context.getFilesDir() returned null")?;

        if files_dir.is_null() {
            return Err(anyhow!("Context.getFilesDir() returned null"));
        }

        let absolute_path = env
            .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .context("File.getAbsolutePath() call failed")?
            .l()
            .context("File.getAbsolutePath() returned null")?;

        if absolute_path.is_null() {
            return Err(anyhow!("File.getAbsolutePath() returned null"));
        }

        let path: String = env
            .get_string(JString::from(absolute_path))
            .context("Failed to read absolute path string from Java")?
            .into();

        Ok(PathBuf::from(path))
    })
}
