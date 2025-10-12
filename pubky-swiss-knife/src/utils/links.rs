use anyhow::{Context, Result, bail};

#[cfg(target_os = "android")]
use anyhow::anyhow;

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

    let mut error = None;
    with_current_env(|env| {
        if let Err(err) = Intent::new_with_uri(env, "ACTION_VIEW", url).start_activity() {
            error = Some(err);
        }
    });

    if let Some(err) = error {
        Err(anyhow!(err)).context("failed to hand off pubkyauth link")
    } else {
        Ok(())
    }
}
