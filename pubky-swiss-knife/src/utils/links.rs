use anyhow::{Context, Result, bail};

/// Attempt to open a pubkyauth:// deep link on the local system.
///
/// This allows the Swiss Knife tool to hand off an authorization request to a
/// locally-installed handler, such as the Pubky signer app.
pub fn open_pubkyauth_link(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("No pubkyauth link to open");
    }

    open::that(trimmed).context("failed to hand off pubkyauth link")?;
    Ok(())
}
