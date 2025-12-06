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
