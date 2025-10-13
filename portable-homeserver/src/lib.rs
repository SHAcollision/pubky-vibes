use mimalloc::MiMalloc;

pub mod app;

pub use app::App;

#[cfg(not(target_os = "android"))]
pub use app::launch_desktop;

#[cfg(target_os = "android")]
pub use app::launch_mobile;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
