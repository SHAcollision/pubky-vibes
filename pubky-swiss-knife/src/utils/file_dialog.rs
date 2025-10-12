use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum FileDialogResult {
    Selected(PathBuf),
    Cancelled,
    Unavailable,
}

pub const MANUAL_ENTRY_HINT: &str =
    "File picker unavailable on this platform. Enter a path manually.";

#[cfg(target_os = "android")]
pub fn pick_file() -> FileDialogResult {
    FileDialogResult::Unavailable
}

#[cfg(not(target_os = "android"))]
pub fn pick_file() -> FileDialogResult {
    rfd::FileDialog::new()
        .pick_file()
        .map(FileDialogResult::Selected)
        .unwrap_or(FileDialogResult::Cancelled)
}

#[cfg(target_os = "android")]
pub fn save_file() -> FileDialogResult {
    FileDialogResult::Unavailable
}

#[cfg(not(target_os = "android"))]
pub fn save_file() -> FileDialogResult {
    rfd::FileDialog::new()
        .save_file()
        .map(FileDialogResult::Selected)
        .unwrap_or(FileDialogResult::Cancelled)
}
