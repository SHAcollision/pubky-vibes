#[cfg(not(target_os = "android"))]
fn main() -> anyhow::Result<()> {
    pubky_swiss_knife::launch_desktop()
}

#[cfg(target_os = "android")]
fn main() {
    pubky_swiss_knife::launch_mobile();
}
