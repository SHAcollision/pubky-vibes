use anyhow::Result;
use mimalloc::MiMalloc;

pub mod app;
pub mod components;
pub mod style;
pub mod tabs;
pub mod utils;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod runtime {
    use super::app;
    use anyhow::Result;
    use dioxus::LaunchBuilder;

    #[cfg(not(target_os = "android"))]
    use dioxus_desktop::{
        Config,
        tao::{dpi::LogicalSize, window::WindowBuilder},
    };

    #[cfg(not(target_os = "android"))]
    const TITLE: &str = "Pubky Swiss Knife";

    #[cfg(not(target_os = "android"))]
    const DESKTOP_WIDTH: f64 = 1220.0;

    #[cfg(not(target_os = "android"))]
    const DESKTOP_HEIGHT: f64 = 820.0;

    #[cfg(not(target_os = "android"))]
    pub fn launch_desktop() -> Result<()> {
        LaunchBuilder::desktop()
            .with_cfg(
                Config::new().with_window(
                    WindowBuilder::new()
                        .with_title(TITLE)
                        .with_inner_size(LogicalSize::new(DESKTOP_WIDTH, DESKTOP_HEIGHT))
                        .with_resizable(false),
                ),
            )
            .launch(app::App);
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub fn launch_mobile() -> Result<()> {
        LaunchBuilder::mobile().launch(app::App);
        Ok(())
    }

    #[cfg(not(target_os = "android"))]
    pub fn launch_current() -> Result<()> {
        launch_desktop()
    }

    #[cfg(target_os = "android")]
    pub fn launch_current() -> Result<()> {
        launch_mobile()
    }
}
