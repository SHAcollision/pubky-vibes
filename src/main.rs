#![cfg_attr(target_os = "android", no_main)]
#![cfg_attr(target_os = "android", allow(non_snake_case))]

/// Platform dispatcher.
///
/// Desktop builds use the standard `main` entrypoint, while Android uses the
/// `ndk_glue::main` macro to wire up the native activity for `cargo-apk`
/// produced `cdylib` artifacts.
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn main() {
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        dioxus_template::launch_desktop();
    }

    #[cfg(target_os = "android")]
    {
        dioxus_template::launch_mobile();
    }

    #[cfg(target_arch = "wasm32")]
    {
        dioxus_template::launch_web();
    }
}
