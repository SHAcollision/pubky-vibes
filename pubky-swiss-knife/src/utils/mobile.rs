use dioxus::prelude::*;

pub const IS_ANDROID: bool = cfg!(target_os = "android");

#[cfg(target_os = "android")]
pub fn touch_tooltip(value: impl Into<String>) -> Option<String> {
    Some(value.into())
}

#[cfg(not(target_os = "android"))]
pub fn touch_tooltip(value: impl Into<String>) -> Option<String> {
    let _ = value;
    None
}

#[cfg(target_os = "android")]
pub fn touch_copy<T: Into<String>>(value: T) -> Option<String> {
    Some(value.into())
}

#[cfg(not(target_os = "android"))]
pub fn touch_copy<T: Into<String>>(value: T) -> Option<String> {
    let _ = value;
    None
}

#[cfg(target_os = "android")]
pub fn touch_copy_option<T: Into<String>>(value: Option<T>) -> Option<String> {
    value.map(Into::into)
}

#[cfg(not(target_os = "android"))]
pub fn touch_copy_option<T: Into<String>>(value: Option<T>) -> Option<String> {
    let _ = value;
    None
}

#[cfg(target_os = "android")]
#[component]
pub fn MobileEnhancementsScript() -> Element {
    rsx! { script { { include_str!("../../assets/mobile.js") } } }
}

#[cfg(not(target_os = "android"))]
#[component]
pub fn MobileEnhancementsScript() -> Element {
    rsx! { Fragment {} }
}
