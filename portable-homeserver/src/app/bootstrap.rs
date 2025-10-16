use dioxus::LaunchBuilder;

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::{Config, WindowBuilder};

#[cfg(not(target_os = "android"))]
pub fn launch_desktop() -> Result<()> {
    super::logs::init_logging()?;

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(WindowBuilder::new().with_title("Portable Pubky Homeserver")),
        )
        .launch(super::App);

    Ok(())
}

#[cfg(target_os = "android")]
pub fn launch_mobile() {
    if let Err(err) = super::logs::init_logging() {
        eprintln!("failed to initialize logging: {err:?}");
    }

    LaunchBuilder::mobile().launch(super::App);
}
