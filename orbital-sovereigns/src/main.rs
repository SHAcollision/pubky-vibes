#[cfg(not(target_os = "android"))]
fn main() -> anyhow::Result<()> {
    orbital_sovereigns::launch_desktop()
}

#[cfg(target_os = "android")]
fn main() {
    orbital_sovereigns::launch_mobile();
}
