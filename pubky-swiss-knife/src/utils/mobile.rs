use dioxus::prelude::*;

pub const IS_ANDROID: bool = cfg!(target_os = "android");

pub fn touch_tooltip(value: impl Into<String>) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        Some(value.into())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = value;
        None
    }
}

pub fn touch_copy<T: Into<String>>(value: T) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        Some(value.into())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = value;
        None
    }
}

pub fn touch_copy_option<T: Into<String>>(value: Option<T>) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        value.map(|inner| inner.into())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = value;
        None
    }
}

#[component]
pub fn MobileEnhancementsScript() -> Element {
    #[cfg(target_os = "android")]
    {
        return rsx! { script { { include_str!("../../assets/mobile.js") } } };
    }

    #[cfg(not(target_os = "android"))]
    {
        rsx! { Fragment {} }
    }
}
