use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::LaunchBuilder;

pub fn launch() {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(WindowBuilder::new().with_title("Portable Pubky Homeserver")),
        )
        .launch(super::ui::App);
}
