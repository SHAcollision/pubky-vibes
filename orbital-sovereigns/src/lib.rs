use dioxus::LaunchBuilder;
use mimalloc::MiMalloc;

#[cfg(not(target_os = "android"))]
use anyhow::Result;

pub mod app;
pub mod components;
pub mod models;
pub mod services;
pub mod style;

pub use app::App;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(not(target_os = "android"))]
use dioxus_desktop::Config;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::tao::dpi::LogicalSize;
#[cfg(not(target_os = "android"))]
use dioxus_desktop::tao::window::WindowBuilder;

#[cfg(not(target_os = "android"))]
pub fn launch_desktop() -> Result<()> {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Orbital Sovereigns")
                    .with_inner_size(LogicalSize::new(1320.0, 860.0))
                    .with_resizable(true),
            ),
        )
        .launch(App);
    Ok(())
}

#[cfg(target_os = "android")]
pub fn launch_mobile() {
    LaunchBuilder::mobile().launch(App);
}
