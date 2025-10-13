/// Global stylesheet for the Dioxus desktop shell.
///
/// We keep the CSS in a dedicated asset file so designers can tweak the layout
/// without hunting through Rust string literals. The file is embedded at compile
/// time which keeps the binary self-contained while still being easy to edit
/// and diff.
pub(crate) static STYLE: &str = include_str!("../../assets/app.css");
