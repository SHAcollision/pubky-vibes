use anyhow::{Result, bail};

#[cfg(not(target_os = "android"))]
use anyhow::Context;

fn validated_link(url: &str) -> Result<&str> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("No pubkyauth link to open");
    }
    Ok(trimmed)
}

/// Attempt to open a pubkyauth:// deep link on the local system.
///
/// This allows the Swiss Knife tool to hand off an authorization request to a
/// locally-installed handler, such as the Pubky signer app.
#[cfg(not(target_os = "android"))]
pub fn open_pubkyauth_link(url: &str) -> Result<()> {
    let trimmed = validated_link(url)?;
    open::that(trimmed).context("failed to hand off pubkyauth link")?;
    Ok(())
}

/// Android relies on the webview opening a new browser tab for the deep link,
/// so the helper only validates the provided URL there.
#[cfg(target_os = "android")]
pub fn open_pubkyauth_link(url: &str) -> Result<()> {
    let _ = validated_link(url)?;
    Ok(())
}
