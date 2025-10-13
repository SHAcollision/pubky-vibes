mod bootstrap;
mod config;
mod state;
mod status;
mod style;
mod tasks;
mod ui;

#[cfg(not(target_os = "android"))]
pub use bootstrap::launch_desktop;

#[cfg(target_os = "android")]
pub use bootstrap::launch_mobile;

pub use ui::App;
