use anyhow::Result;
use dioxus::LaunchBuilder;
use dioxus_desktop::Config;
use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::tao::window::WindowBuilder;
use mimalloc::MiMalloc;

mod app;
mod components;
mod style;
mod tabs;
mod utils;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Pubky Swiss Knife")
                    .with_inner_size(LogicalSize::new(1220.0, 820.0))
                    .with_resizable(false),
            ),
        )
        .launch(app::App);
    Ok(())
}
