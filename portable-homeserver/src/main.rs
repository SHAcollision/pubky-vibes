#[cfg(not(target_os = "android"))]
fn main() -> anyhow::Result<()> {
    portable_homeserver::launch_desktop()
}

#[cfg(target_os = "android")]
fn main() {
    portable_homeserver::launch_mobile();
}
