use dioxus::prelude::*;

// Mimalloc performs well on desktop but can be unreliable on some Android
// devices. Keep the system allocator for Android while retaining mimalloc for
// other native targets.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use mimalloc::MiMalloc;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub fn app() -> Element {
    use dioxus::prelude::*;

    rsx! {
        div {
            style: "min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; \
                    background: linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #334155 100%); color: #e2e8f0;",
            div {
                style: "max-width: 420px; width: 100%; border-radius: 18px; padding: 20px; background: rgba(15, 23, 42, 0.8); \
                        box-shadow: 0 16px 50px rgba(0,0,0,0.45); border: 1px solid rgba(148, 163, 184, 0.25);",
                h1 { style: "margin: 0 0 8px; font-size: 24px;", "Pubky Vibes" }
                p { style: "margin: 0 0 16px; line-height: 1.5;", "Cross-platform starter powered by Dioxus — runs on desktop, web, and Android." }
                ul { style: "margin: 0; padding-left: 18px; line-height: 1.6;",
                    li { "Run locally with `cargo run`." }
                    li { "Preview in the browser via `trunk serve` or `dx serve --platform web`." }
                    li { "Build an Android APK with `cargo apk build --release --target aarch64-linux-android`." }
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
