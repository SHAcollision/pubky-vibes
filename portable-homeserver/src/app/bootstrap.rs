use dioxus::LaunchBuilder;

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::{Config, WindowBuilder};

#[cfg(target_os = "android")]
use tracing::error;

#[cfg(not(target_os = "android"))]
pub fn launch_desktop() -> Result<()> {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(WindowBuilder::new().with_title("Portable Pubky Homeserver")),
        )
        .launch(super::App);

    Ok(())
}

#[cfg(target_os = "android")]
pub fn launch_mobile() {
    if let Err(err) = super::android_fs::prepare_storage() {
        error!(?err, "failed to prepare Android storage");
    }

    LaunchBuilder::mobile().launch(super::App);
}
