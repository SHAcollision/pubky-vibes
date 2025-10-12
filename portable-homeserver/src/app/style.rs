use std::sync::LazyLock;

use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Global stylesheet for the Dioxus desktop shell.
///
/// We keep the CSS in a dedicated asset file so designers can tweak the layout
/// without hunting through Rust string literals. The file is embedded at compile
/// time which keeps the binary self-contained while still being easy to edit
/// and diff.
pub(crate) static STYLE: &str = include_str!("../../assets/app.css");

/// Embedded Pubky logo exposed as a data URI so the desktop shell can render it
/// without touching the filesystem at runtime.
pub(crate) static LOGO_DATA_URI: LazyLock<String> = LazyLock::new(|| {
    let encoded = STANDARD.encode(include_bytes!("../../assets/pubky-core-logo.svg"));
    format!("data:image/svg+xml;base64,{}", encoded)
});
