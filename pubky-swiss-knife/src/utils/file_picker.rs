use std::path::PathBuf;

#[cfg(not(target_os = "android"))]
pub fn pick_file() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_file()
}

#[cfg(target_os = "android")]
pub fn pick_file() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "android"))]
pub fn save_file() -> Option<PathBuf> {
    rfd::FileDialog::new().save_file()
}

#[cfg(target_os = "android")]
pub fn save_file() -> Option<PathBuf> {
    None
}
