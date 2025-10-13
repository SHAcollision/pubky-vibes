use dioxus::LaunchBuilder;

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::{Config, WindowBuilder};

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
    super::platform::ensure_initialized();
    LaunchBuilder::mobile().launch(super::App);
}
