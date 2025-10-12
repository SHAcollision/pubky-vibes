use dioxus::LaunchBuilder;
use mimalloc::MiMalloc;

#[cfg(not(target_os = "android"))]
use anyhow::Result;

pub mod app;
pub mod components;
pub mod style;
pub mod tabs;
pub mod utils;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(not(target_os = "android"))]
use dioxus_desktop::Config;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::tao::dpi::LogicalSize;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::tao::window::WindowBuilder;

pub use app::App;

#[cfg(not(target_os = "android"))]
pub fn launch_desktop() -> Result<()> {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Pubky Swiss Knife")
                    .with_inner_size(LogicalSize::new(1220.0, 820.0))
                    .with_resizable(false),
            ),
        )
        .launch(App);
    Ok(())
}

#[cfg(target_os = "android")]
pub fn launch_mobile() {
    LaunchBuilder::mobile().launch(App);
}
