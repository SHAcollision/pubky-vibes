#[cfg(target_os = "android")]
mod android;
mod admin;
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

#[cfg(target_os = "android")]
pub(crate) use android::{
    android_default_data_dir as android_data_dir, ensure_android_environment,
};

pub use ui::App;
