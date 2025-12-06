use dioxus::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use mimalloc::MiMalloc;
#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub fn app() -> Element {
    use dioxus::prelude::*;

    rsx! {
        main { class: "app-shell",
            header { class: "hero",
                h1 { "Dioxus template" }
                p { "Cross-platform starter with desktop, web, and Android builds." }
            }
            section { class: "content",
                h2 { "Getting started" }
                ol {
                    li { "Run the desktop app with `cargo run`." }
                    li { "Serve the web build with `trunk serve` or `dx serve --platform web`." }
                    li { "Produce release artifacts through the provided GitHub workflows." }
                }
                h2 { "Why this template" }
                ul {
                    li { "Single codebase that targets desktop, web, and Android." }
                    li { "Opinionated workflows for formatting, linting, and release packaging." }
                    li { "Ready-to-tweak configuration for CI and production builds." }
                }
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn launch_desktop() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    dioxus::LaunchBuilder::desktop().launch(app);
}

#[cfg(target_arch = "wasm32")]
pub fn launch_web() {
    dioxus::LaunchBuilder::web().launch(app);
}

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(target_os = "android")]
pub fn launch_mobile(_android_app: AndroidApp) {
    dioxus::LaunchBuilder::mobile().launch(app);
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn android_main(app: AndroidApp) {
    launch_mobile(app);
}
