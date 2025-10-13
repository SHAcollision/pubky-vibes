use dioxus::prelude::*;

#[must_use]
pub fn is_android_touch() -> bool {
    cfg!(target_os = "android") || is_android_touch_runtime()
}

#[cfg(target_os = "android")]
const fn is_android_touch_runtime() -> bool {
    true
}

#[cfg(all(target_arch = "wasm32", not(target_os = "android")))]
fn is_android_touch_runtime() -> bool {
    use std::sync::OnceLock;

    static IS_ANDROID_TOUCH: OnceLock<bool> = OnceLock::new();
    *IS_ANDROID_TOUCH.get_or_init(|| {
        use web_sys::window;

        window()
            .map(|win| win.navigator())
            .and_then(|navigator| navigator.user_agent().ok())
            .map(|ua| ua.to_ascii_lowercase().contains("android"))
            .unwrap_or(false)
    })
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
const fn is_android_touch_runtime() -> bool {
    false
}

pub fn touch_tooltip(value: impl Into<String>) -> Option<String> {
    is_android_touch().then(|| value.into())
}

pub fn touch_copy<T: Into<String>>(value: T) -> Option<String> {
    is_android_touch().then(|| value.into())
}

pub fn touch_copy_option<T: Into<String>>(value: Option<T>) -> Option<String> {
    if is_android_touch() {
        value.map(Into::into)
    } else {
        None
    }
}

#[component]
pub fn MobileEnhancementsScript() -> Element {
    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    {
        rsx! { script { { include_str!("../../assets/mobile.js") } } }
    }

    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        rsx! { Fragment {} }
    }
}
