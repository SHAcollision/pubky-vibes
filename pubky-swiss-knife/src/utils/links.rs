use anyhow::{Context, Result, bail};

#[cfg(target_os = "android")]
use anyhow::anyhow;
#[cfg(target_os = "android")]
use jni::errors::Error as JniError;

/// Attempt to open a pubkyauth:// deep link on the local system.
///
/// This allows the Swiss Knife tool to hand off an authorization request to a
/// locally-installed handler, such as the Pubky signer app.
pub fn open_pubkyauth_link(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("No pubkyauth link to open");
    }

    open_on_platform(trimmed)
}

#[cfg(not(target_os = "android"))]
fn open_on_platform(url: &str) -> Result<()> {
    open::that(url).context("failed to hand off pubkyauth link")?;
    Ok(())
}

#[cfg(target_os = "android")]
fn open_on_platform(url: &str) -> Result<()> {
    use android_intent::{Intent, with_current_env};

    let mut activity_result: std::result::Result<(), JniError> = Ok(());
    with_current_env(|jni_env| {
        let error_env = jni_env.clone();
        match Intent::new_with_uri(jni_env, "ACTION_VIEW", url).start_activity() {
            Ok(()) => {}
            Err(err) => {
                if let JniError::JavaException = &err {
                    let _ = error_env.exception_describe();
                    let _ = error_env.exception_clear();
                }
                activity_result = Err(err);
            }
        }
    });

    activity_result
        .map_err(|err| anyhow!(err))
        .context("failed to hand off pubkyauth link")
}
