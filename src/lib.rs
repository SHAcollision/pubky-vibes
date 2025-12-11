use dioxus::prelude::*;

// Mimalloc performs well on desktop but can be unreliable on some Android
// devices. Keep the system allocator for Android while retaining mimalloc for
// other native targets.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use mimalloc::MiMalloc;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(target_os = "android")]
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn android_main() {
    launch_mobile();
}

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
pub fn launch_mobile() {
    dioxus_mobile::launch(app);
}
