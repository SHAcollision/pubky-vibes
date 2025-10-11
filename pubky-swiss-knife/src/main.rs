use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::LaunchBuilder;
use dioxus::prelude::*;
use dioxus_desktop::Config;
use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::tao::window::WindowBuilder;
use mimalloc::MiMalloc;
use pubky::recovery_file;
use pubky::{
    AuthToken, Capabilities, Keypair, Method, Pubky, PubkyAuthFlow, PubkyHttpClient, PubkySession,
    PublicKey,
};
use qrcode::{QrCode, render::svg};
use reqwest::header::HeaderName;
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Pubky Swiss Knife")
                    .with_inner_size(LogicalSize::new(1220.0, 820.0))
                    .with_resizable(false),
            ),
        )
        .launch(App);
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NetworkMode {
    Mainnet,
    Testnet,
}

impl NetworkMode {
    const ALL: [NetworkMode; 2] = [NetworkMode::Mainnet, NetworkMode::Testnet];

    fn label(self) -> &'static str {
        match self {
            NetworkMode::Mainnet => "Mainnet",
            NetworkMode::Testnet => "Testnet",
        }
    }
}

#[component]
fn NetworkToggleOption(network_mode: Signal<NetworkMode>, mode: NetworkMode) -> Element {
    let is_selected = *network_mode.read() == mode;
    let mut setter = network_mode;
    rsx! {
        label {
            input {
                r#type: "radio",
                name: "network-mode",
                checked: is_selected,
                onchange: move |_| setter.set(mode),
            }
            span { "{mode.label()}" }
        }
    }
}

#[component]
fn TabButton(tab: Tab, active_tab: Signal<Tab>) -> Element {
    let is_active = *active_tab.read() == tab;
    let mut setter = active_tab;
    let class_name = if is_active { "action active" } else { "action" };
    rsx! {
        button {
            class: class_name,
            onclick: move |_| setter.set(tab),
            "{tab.label()}"
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Keys,
    Tokens,
    Sessions,
    Auth,
    Storage,
    Http,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Keys,
        Tab::Tokens,
        Tab::Sessions,
        Tab::Auth,
        Tab::Storage,
        Tab::Http,
    ];

    fn label(self) -> &'static str {
        match self {
            Tab::Keys => "Keys",
            Tab::Tokens => "Auth Tokens",
            Tab::Sessions => "Sessions",
            Tab::Auth => "Auth Flows",
            Tab::Storage => "Storage",
            Tab::Http => "Raw Requests",
        }
    }
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Success,
    Error,
}

struct LogEntry {
    level: LogLevel,
    message: String,
}

impl LogEntry {
    fn class(&self) -> &'static str {
        match self.level {
            LogLevel::Info => "log-info",
            LogLevel::Success => "log-success",
            LogLevel::Error => "log-error",
        }
    }
}

const STYLES: &str = r#"
:root {
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    color: #e2e8f0;
    background: #020617;
}

*, *::before, *::after {
    box-sizing: border-box;
}

html, body {
    width: 100%;
    height: 100%;
    margin: 0;
    background: radial-gradient(circle at 15% 10%, rgba(59, 130, 246, 0.15), transparent 55%),
        radial-gradient(circle at 85% 0%, rgba(124, 58, 237, 0.18), transparent 45%),
        #020617;
    color: inherit;
    overflow: hidden;
}

body {
    font-size: 16px;
}

.app {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    gap: 1.5rem;
    padding: 1.75rem 2.25rem 2.25rem;
}

header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.18);
}

.title-block {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.brand-row {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.brandmark {
    height: 38px;
    width: auto;
    filter: drop-shadow(0 6px 18px rgba(15, 23, 42, 0.85));
}

header h1 {
    margin: 0;
    font-size: 2rem;
    font-weight: 600;
    letter-spacing: -0.01em;
}

.subtitle {
    margin: 0;
    font-size: 0.95rem;
    color: rgba(226, 232, 240, 0.76);
}

.header-controls {
    display: flex;
    align-items: flex-start;
    gap: 1.25rem;
}

.network-toggle {
    display: flex;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.2);
    background: rgba(15, 23, 42, 0.7);
}

.network-toggle label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    cursor: pointer;
    font-size: 0.9rem;
    color: rgba(226, 232, 240, 0.85);
}

.network-toggle input[type="radio"] {
    accent-color: #60a5fa;
}

main {
    flex: 1;
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 1.5rem;
    min-height: 0;
}

.tabs {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
}

.tabs button {
    padding: 0.75rem 1rem;
    border: 1px solid transparent;
    border-radius: 0.85rem;
    background: rgba(30, 41, 59, 0.6);
    color: #e2e8f0;
    font-size: 0.95rem;
    cursor: pointer;
    text-align: left;
    transition: transform 0.15s ease, border 0.2s ease, background 0.25s ease;
}

.tabs button:hover {
    transform: translateX(4px);
    border-color: rgba(94, 234, 212, 0.3);
}

.tabs button.active {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.95), rgba(124, 58, 237, 0.9));
    border-color: rgba(148, 163, 184, 0.32);
    box-shadow: 0 12px 32px rgba(30, 64, 175, 0.35);
    transform: translateX(8px);
}

.panel {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    background: rgba(8, 11, 23, 0.78);
    border-radius: 1.15rem;
    padding: 1.6rem;
    box-shadow: 0 18px 40px rgba(2, 6, 23, 0.65);
    border: 1px solid rgba(148, 163, 184, 0.22);
    min-width: 0;
    overflow-y: auto;
}

.tab-body {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: minmax(0, 1fr);
    gap: 1.25rem;
    align-content: start;
    min-height: 0;
}

.tab-body.single-column {
    grid-template-columns: minmax(0, 1fr);
}

.tab-body.tight {
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
}

.tab-body > .card {
    height: 100%;
}

.card {
    background: rgba(15, 23, 42, 0.72);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 1rem;
    padding: 1.25rem 1.35rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    box-shadow: inset 0 0 0 1px rgba(94, 234, 212, 0.03);
}

.card h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: #f8fafc;
}

.helper-text {
    margin: 0;
    font-size: 0.85rem;
    color: rgba(226, 232, 240, 0.7);
}

.form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 0.85rem 1rem;
}

label {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    font-size: 0.9rem;
}

input[type="text"],
input[type="password"],
textarea,
select {
    background: rgba(10, 14, 26, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.28);
    border-radius: 0.75rem;
    padding: 0.6rem 0.75rem;
    color: #e2e8f0;
    font-size: 0.95rem;
    transition: border 0.2s ease, box-shadow 0.2s ease;
    width: 100%;
}

input::placeholder,
textarea::placeholder {
    color: rgba(148, 163, 184, 0.6);
}

input:focus,
textarea:focus,
select:focus {
    outline: none;
    border-color: rgba(59, 130, 246, 0.65);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.25);
}

textarea {
    width: 100%;
    min-height: 5.75rem;
    max-height: 9rem;
    resize: none;
}

textarea.tall {
    min-height: 8.5rem;
}

.small-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
}

button.action {
    padding: 0.65rem 1.15rem;
    border: none;
    border-radius: 0.75rem;
    background: linear-gradient(135deg, rgba(37, 99, 235, 0.92), rgba(14, 165, 233, 0.9));
    color: #f8fafc;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.12s ease, box-shadow 0.2s ease;
}

button.action:hover {
    transform: translateY(-2px);
    box-shadow: 0 14px 28px rgba(37, 99, 235, 0.35);
}

button.secondary {
    background: rgba(30, 41, 59, 0.85);
    color: rgba(226, 232, 240, 0.9);
}

.outputs {
    background: rgba(8, 11, 23, 0.85);
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 0.85rem;
    padding: 1rem;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, SFMono-Regular, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    font-size: 0.9rem;
}

.qr-container {
    display: flex;
    align-items: flex-start;
    gap: 1.5rem;
    flex-wrap: wrap;
}

.qr-container img {
    background: #f8fafc;
    padding: 0.75rem;
    border-radius: 1.1rem;
    box-shadow: 0 12px 30px rgba(15, 23, 42, 0.4);
    max-width: 220px;
}

.qr-container textarea {
    width: min(420px, 100%);
}

.auth-status {
    font-size: 0.9rem;
    color: rgba(148, 163, 184, 0.9);
}

.activity-drawer {
    position: absolute;
    right: 2.25rem;
    bottom: 2.25rem;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.75rem;
}

.activity-button {
    padding: 0.55rem 0.9rem;
    border-radius: 999px;
    border: 1px solid rgba(59, 130, 246, 0.6);
    background: rgba(15, 23, 42, 0.75);
    color: rgba(191, 219, 254, 0.95);
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s ease, transform 0.12s ease;
}

.activity-button:hover {
    transform: translateY(-1px);
    background: rgba(30, 41, 59, 0.9);
}

.logs-panel {
    width: 340px;
    max-height: 280px;
    background: rgba(15, 23, 42, 0.9);
    border-radius: 1rem;
    padding: 1.1rem;
    border: 1px solid rgba(148, 163, 184, 0.22);
    box-shadow: 0 18px 32px rgba(2, 6, 23, 0.55);
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
}

.logs-panel h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #f8fafc;
}

.log-scroll {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
}

.log-entry {
    padding: 0.6rem 0.75rem;
    border-radius: 0.7rem;
    background: rgba(30, 41, 59, 0.78);
    border: 1px solid transparent;
    font-size: 0.9rem;
}

.log-info {
    border-color: rgba(148, 163, 184, 0.38);
}

.log-success {
    border-color: rgba(34, 197, 94, 0.55);
    background: rgba(22, 163, 74, 0.2);
}

.log-error {
    border-color: rgba(248, 113, 113, 0.6);
    background: rgba(248, 113, 113, 0.22);
}

.span-2 {
    grid-column: span 2;
}

.status-chip {
    padding: 0.35rem 0.6rem;
    border-radius: 0.65rem;
    background: rgba(34, 197, 94, 0.18);
    color: rgba(187, 247, 208, 0.95);
    font-size: 0.8rem;
    align-self: flex-start;
}
"#;

#[allow(non_snake_case)]
fn App() -> Element {
    let active_tab = use_signal(|| Tab::Keys);
    let network_mode = use_signal(|| NetworkMode::Mainnet);
    let logs = use_signal(|| Vec::<LogEntry>::new());
    let show_logs = use_signal(|| false);

    let keypair = use_signal(|| Option::<Keypair>::None);
    let secret_input = use_signal(String::new);
    let recovery_path = use_signal(String::new);
    let recovery_passphrase = use_signal(String::new);

    let token_caps_input = use_signal(|| String::from("/:rw"));
    let token_output = use_signal(String::new);

    let session = use_signal(|| Option::<PubkySession>::None);
    let session_details = use_signal(String::new);
    let homeserver_input = use_signal(String::new);
    let signup_code_input = use_signal(String::new);

    let auth_caps_input = use_signal(|| String::from("/:rw"));
    let auth_relay_input = use_signal(String::new);
    let auth_url_output = use_signal(String::new);
    let auth_qr_data = use_signal(|| Option::<String>::None);
    let auth_status = use_signal(String::new);
    let auth_flow = use_signal(|| Option::<PubkyAuthFlow>::None);
    let auth_request_input = use_signal(String::new);

    let storage_path = use_signal(|| String::from("/pub/"));
    let storage_body = use_signal(String::new);
    let storage_response = use_signal(String::new);

    let public_resource = use_signal(String::new);
    let public_response = use_signal(String::new);

    let http_method = use_signal(|| String::from("GET"));
    let http_url = use_signal(|| String::from("https://"));
    let http_headers = use_signal(String::new);
    let http_body = use_signal(String::new);
    let http_response = use_signal(String::new);

    let show_logs_value = *show_logs.read();
    let show_logs_label = if show_logs_value {
        "Hide activity"
    } else {
        "Show activity"
    };
    let has_logs = !logs.read().is_empty();
    let mut toggle_logs_signal = show_logs.clone();

    rsx! {
        style { {STYLES} }
        div { class: "app",
            header {
                div { class: "title-block",
                    div { class: "brand-row",
                        img {
                            class: "brandmark",
                            src: "https://pubky.org/pubky-core-logo.svg",
                            alt: "Pubky Core logotype",
                        }
                        h1 { "Swiss Knife" }
                    }
                    p { class: "subtitle", "A tidy cockpit for every Pubky homeserver workflow." }
                }
                div { class: "header-controls",
                    div { class: "network-toggle",
                        for mode in NetworkMode::ALL {
                            NetworkToggleOption { network_mode: network_mode.clone(), mode }
                        }
                    }
                }
            }
            main {
                nav { class: "tabs",
                    for tab in Tab::ALL {
                        TabButton { tab, active_tab: active_tab.clone() }
                    }
                }
                div { class: "panel",
                    match *active_tab.read() {
                        Tab::Keys => render_keys_tab(
                            keypair,
                            secret_input,
                            recovery_path,
                            recovery_passphrase,
                            logs,
                        ),
                        Tab::Tokens => render_tokens_tab(keypair, token_caps_input, token_output, logs),
                        Tab::Sessions => render_sessions_tab(
                            network_mode,
                            keypair,
                            session,
                            session_details,
                            homeserver_input,
                            signup_code_input,
                            logs,
                        ),
                        Tab::Auth => render_auth_tab(
                            network_mode,
                            keypair,
                            session,
                            session_details,
                            auth_caps_input,
                            auth_relay_input,
                            auth_url_output,
                            auth_qr_data,
                            auth_status,
                            auth_flow,
                            auth_request_input,
                            logs,
                        ),
                        Tab::Storage => render_storage_tab(
                            network_mode,
                            session,
                            storage_path,
                            storage_body,
                            storage_response,
                            public_resource,
                            public_response,
                            logs,
                        ),
                        Tab::Http => render_http_tab(
                            network_mode,
                            http_method,
                            http_url,
                            http_headers,
                            http_body,
                            http_response,
                            logs,
                        ),
                    }
                }
            }
            div { class: "activity-drawer",
                button {
                    class: "activity-button",
                    onclick: move |_| {
                        let next = !*toggle_logs_signal.read();
                        toggle_logs_signal.set(next);
                    },
                    "{show_logs_label}"
                }
                if show_logs_value {
                    div { class: "logs-panel",
                        h3 { "Activity" }
                        div {
                            class: "log-scroll",
                            role: "log",
                            aria_live: "polite",
                            if has_logs {
                                for entry in logs.read().iter() {
                                    div { class: format_args!("log-entry {}", entry.class()), "{entry.message}" }
                                }
                            } else {
                                div { class: "log-entry log-info", "No activity yet. Trigger any action to see logs here." }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_keys_tab(
    keypair: Signal<Option<Keypair>>,
    secret_input: Signal<String>,
    recovery_path: Signal<String>,
    recovery_passphrase: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let current_public = {
        let guard = keypair.read();
        guard
            .as_ref()
            .map(|kp| kp.public_key().to_string())
            .unwrap_or_else(|| "–".to_string())
    };
    let secret_value = { secret_input.read().clone() };
    let recovery_path_value = { recovery_path.read().clone() };
    let recovery_pass_value = { recovery_passphrase.read().clone() };
    let recovery_path_display = if recovery_path_value.trim().is_empty() {
        "No file selected".to_string()
    } else {
        recovery_path_value.clone()
    };

    let mut generate_secret_input = secret_input.clone();
    let mut generate_keypair = keypair.clone();
    let generate_logs = logs.clone();

    let mut export_secret_input = secret_input.clone();
    let export_keypair = keypair.clone();
    let export_logs = logs.clone();

    let mut import_keypair_signal = keypair.clone();
    let import_secret_signal = secret_input.clone();
    let import_logs = logs.clone();

    let load_path_signal = recovery_path.clone();
    let load_pass_signal = recovery_passphrase.clone();
    let load_keypair_signal = keypair.clone();
    let load_secret_signal = secret_input.clone();
    let load_logs = logs.clone();

    let save_path_signal = recovery_path.clone();
    let save_pass_signal = recovery_passphrase.clone();
    let save_keypair_signal = keypair.clone();
    let save_logs = logs.clone();

    let mut secret_input_binding = secret_input.clone();
    let mut recovery_pass_binding = recovery_passphrase.clone();
    let mut choose_recovery_path_signal = recovery_path.clone();

    rsx! {
        div { class: "tab-body tight",
            section { class: "card",
                h2 { "Key material" }
                p { class: "helper-text", "Generate or import keys. Current public key: {current_public}." }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let kp = Keypair::random();
                            generate_secret_input.set(STANDARD.encode(kp.secret_key()));
                            generate_keypair.set(Some(kp.clone()));
                            push_log(generate_logs, LogLevel::Success, format!("Generated signer {}", kp.public_key()));
                        },
                        "Generate random key"
                    }
                    button { class: "action secondary", onclick: move |_| {
                            if let Some(kp) = export_keypair.read().as_ref() {
                                export_secret_input.set(STANDARD.encode(kp.secret_key()));
                                push_log(export_logs, LogLevel::Info, "Secret key exported to editor");
                            } else {
                                push_log(export_logs, LogLevel::Error, "No key loaded");
                            }
                        },
                        "Show secret key"
                    }
                }
                div { class: "form-grid",
                    label {
                        "Secret key (base64)"
                        textarea {
                            class: "tall",
                            value: secret_value,
                            oninput: move |evt| secret_input_binding.set(evt.value()),
                            placeholder: "Base64 encoded 32-byte secret key",
                        }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let secret = import_secret_signal.read().clone();
                            match decode_secret_key(&secret) {
                                Ok(kp) => {
                                    import_keypair_signal.set(Some(kp.clone()));
                                    push_log(import_logs.clone(), LogLevel::Success, format!("Loaded key for {}", kp.public_key()));
                                }
                                Err(err) => push_log(import_logs, LogLevel::Error, format!("Invalid secret key: {err}")),
                            }
                        },
                        "Import secret"
                    }
                }
            }
            section { class: "card",
                h2 { "Recovery files" }
                div { class: "form-grid",
                    label {
                        "Recovery file path"
                        div { class: "file-picker-row",
                            span { class: "file-path-display", "{recovery_path_display}" }
                            button {
                                class: "action secondary",
                                onclick: move |_| {
                                    if let Some(path) = FileDialog::new().pick_file() {
                                        choose_recovery_path_signal.set(path.display().to_string());
                                    }
                                },
                                "Choose file"
                            }
                        }
                    }
                    label {
                        "Passphrase"
                        input { r#type: "password", value: recovery_pass_value.clone(), oninput: move |evt| recovery_pass_binding.set(evt.value()) }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let raw_path = load_path_signal.read().clone();
                            let passphrase = load_pass_signal.read().clone();
                            let mut immediate_path_signal = load_path_signal.clone();
                            let chosen_path = if raw_path.trim().is_empty() {
                                FileDialog::new().pick_file().map(|path| {
                                    let display = path.display().to_string();
                                    immediate_path_signal.set(display.clone());
                                    display
                                })
                            } else {
                                Some(raw_path.clone())
                            };
                            if let Some(selected_path) = chosen_path {
                                let mut keypair_signal = load_keypair_signal.clone();
                                let mut secret_signal = load_secret_signal.clone();
                                let mut path_signal = load_path_signal.clone();
                                let logs_task = load_logs.clone();
                                let passphrase_for_task = passphrase.clone();
                                spawn(async move {
                                    let outcome = (|| -> Result<(Keypair, PathBuf)> {
                                        let normalized = normalize_pkarr_path(&selected_path)?;
                                        let kp = load_keypair_from_recovery(&normalized, &passphrase_for_task)?;
                                        Ok((kp, normalized))
                                    })();
                                    match outcome {
                                        Ok((kp, normalized)) => {
                                            secret_signal.set(STANDARD.encode(kp.secret_key()));
                                            keypair_signal.set(Some(kp.clone()));
                                            path_signal.set(normalized.display().to_string());
                                            push_log(
                                                logs_task,
                                                LogLevel::Success,
                                                format!(
                                                    "Decrypted recovery file {} for {}",
                                                    normalized.display(),
                                                    kp.public_key()
                                                ),
                                            );
                                        }
                                        Err(err) => push_log(
                                            logs_task,
                                            LogLevel::Error,
                                            format!("Failed to load recovery file: {err}"),
                                        ),
                                    }
                                });
                            }
                        },
                        "Load from recovery file"
                    }
                    button { class: "action secondary", onclick: move |_| {
                            if let Some(kp) = save_keypair_signal.read().as_ref().cloned() {
                                let raw_path = save_path_signal.read().clone();
                                let mut immediate_path_signal = save_path_signal.clone();
                                let chosen_path = if raw_path.trim().is_empty() {
                                    FileDialog::new().save_file().map(|path| {
                                        let display = path.display().to_string();
                                        immediate_path_signal.set(display.clone());
                                        display
                                    })
                                } else {
                                    Some(raw_path.clone())
                                };
                                if let Some(selected_path) = chosen_path {
                                    let passphrase = save_pass_signal.read().clone();
                                    let mut path_signal = save_path_signal.clone();
                                    let logs_task = save_logs.clone();
                                    spawn(async move {
                                        match save_keypair_to_recovery_file(&kp, &selected_path, &passphrase) {
                                            Ok(path) => {
                                                path_signal.set(path.display().to_string());
                                                push_log(
                                                    logs_task,
                                                    LogLevel::Success,
                                                    format!("Recovery file saved to {}", path.display()),
                                                );
                                            }
                                            Err(err) => push_log(
                                                logs_task,
                                                LogLevel::Error,
                                                format!("Failed to save recovery file: {err}"),
                                            ),
                                        }
                                    });
                                }
                            } else {
                                push_log(save_logs.clone(), LogLevel::Error, "Generate or import a key first");
                            }
                        },
                        "Save recovery file"
                    }
                }
            }
        }
    }
}

fn render_tokens_tab(
    keypair: Signal<Option<Keypair>>,
    token_caps_input: Signal<String>,
    token_output: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let caps_value = { token_caps_input.read().clone() };
    let token_value = { token_output.read().clone() };

    let mut token_caps_binding = token_caps_input.clone();

    let sign_keypair = keypair.clone();
    let sign_caps = token_caps_input.clone();
    let mut sign_token = token_output.clone();
    let sign_logs = logs.clone();

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Sign capability tokens" }
                p { class: "helper-text", "Compose a capability string (e.g. '/:rw,/pub/app/:r') and sign using the active key." }
                div { class: "form-grid",
                label {
                    "Capabilities"
                    input {
                        value: caps_value,
                        oninput: move |evt| token_caps_binding.set(evt.value()),
                        placeholder: "Comma-separated scopes"
                    }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        let caps = sign_caps.read().clone();
                        if let Some(kp) = sign_keypair.read().as_ref() {
                            match Capabilities::try_from(caps.as_str()) {
                                Ok(capabilities) => {
                                    let token = AuthToken::sign(kp, capabilities.clone());
                                    sign_token.set(STANDARD.encode(token.serialize()));
                                    push_log(sign_logs.clone(), LogLevel::Success, format!(
                                        "Signed token for {} with caps {capabilities}",
                                        kp.public_key()
                                    ));
                                }
                                Err(err) => push_log(sign_logs, LogLevel::Error, format!("Invalid capabilities: {err}")),
                            }
                        } else {
                            push_log(sign_logs, LogLevel::Error, "Load a key before signing");
                        }
                    },
                    "Sign token"
                }
            }
            if !token_value.is_empty() {
                div { class: "outputs", {token_value} }
            }
        }
        }
    }
}

fn render_sessions_tab(
    network_mode: Signal<NetworkMode>,
    keypair: Signal<Option<Keypair>>,
    session: Signal<Option<PubkySession>>,
    session_details: Signal<String>,
    homeserver_input: Signal<String>,
    signup_code_input: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let homeserver_value = { homeserver_input.read().clone() };
    let signup_value = { signup_code_input.read().clone() };
    let details_value = { session_details.read().clone() };

    let mut homeserver_binding = homeserver_input.clone();
    let mut signup_binding = signup_code_input.clone();

    let signup_network = network_mode.clone();
    let signup_keypair = keypair.clone();
    let signup_homeserver = homeserver_input.clone();
    let signup_code_signal = signup_code_input.clone();
    let signup_session_signal = session.clone();
    let signup_details_signal = session_details.clone();
    let signup_logs = logs.clone();

    let signin_network = network_mode.clone();
    let signin_keypair = keypair.clone();
    let signin_session_signal = session.clone();
    let signin_details_signal = session_details.clone();
    let signin_logs = logs.clone();

    let revalidate_session_signal = session.clone();
    let revalidate_details_signal = session_details.clone();
    let revalidate_logs = logs.clone();

    let signout_session_signal = session.clone();
    let signout_details_signal = session_details.clone();
    let signout_logs = logs.clone();

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
            h2 { "Session lifecycle" }
            div { class: "form-grid",
                label {
                    "Homeserver public key"
                    input { value: homeserver_value, oninput: move |evt| homeserver_binding.set(evt.value()) }
                }
                label {
                    "Signup code (optional)"
                    input { value: signup_value, oninput: move |evt| signup_binding.set(evt.value()) }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        if let Some(kp) = signup_keypair.read().as_ref().cloned() {
                            let homeserver = signup_homeserver.read().clone();
                            if homeserver.trim().is_empty() {
                                push_log(signup_logs.clone(), LogLevel::Error, "Homeserver public key is required");
                                return;
                            }
                            let signup_code = signup_code_signal.read().clone();
                            let network = *signup_network.read();
                            let mut session_signal = signup_session_signal.clone();
                            let mut details_signal = signup_details_signal.clone();
                            let logs_task = signup_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let homeserver_pk = PublicKey::try_from(homeserver.as_str())
                                        .map_err(|e| anyhow!("Invalid homeserver key: {e}"))?;
                                    let pubky = build_pubky(network)?;
                                    let signer = pubky.signer(kp.clone());
                                    let session = signer
                                        .signup(&homeserver_pk, if signup_code.is_empty() { None } else { Some(signup_code.as_str()) })
                                        .await?;
                                    session_signal.set(Some(session.clone()));
                                    details_signal.set(format_session_info(session.info()));
                                    Ok::<_, anyhow::Error>(format!("Signed up as {}", session.info().public_key()))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("Signup failed: {err}")),
                                }
                            });
                        } else {
                            push_log(signup_logs, LogLevel::Error, "Load or generate a key first");
                        }
                    },
                    "Sign up"
                }
                button { class: "action secondary", onclick: move |_| {
                        if let Some(kp) = signin_keypair.read().as_ref().cloned() {
                            let network = *signin_network.read();
                            let logs_task = signin_logs.clone();
                            let mut session_signal = signin_session_signal.clone();
                            let mut details_signal = signin_details_signal.clone();
                            spawn(async move {
                                let result = async move {
                                    let pubky = build_pubky(network)?;
                                    let signer = pubky.signer(kp.clone());
                                    let session = signer.signin().await?;
                                    session_signal.set(Some(session.clone()));
                                    details_signal.set(format_session_info(session.info()));
                                    Ok::<_, anyhow::Error>(format!(
                                        "Signed in (root) as {}",
                                        session.info().public_key()
                                    ))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(
                                        logs_task,
                                        LogLevel::Error,
                                        format!("Signin (root) failed: {err}"),
                                    ),
                                }
                            });
                        } else {
                            push_log(signin_logs, LogLevel::Error, "Load or generate a key first");
                        }
                    },
                    "Sign in (root)"
                }
                button { class: "action secondary", onclick: move |_| {
                        if let Some(session) = revalidate_session_signal.read().as_ref().cloned() {
                            let mut session_signal = revalidate_session_signal.clone();
                            let mut details_signal = revalidate_details_signal.clone();
                            let logs_task = revalidate_logs.clone();
                            spawn(async move {
                                match session.revalidate().await {
                                    Ok(Some(info)) => {
                                        details_signal.set(format_session_info(&info));
                                        push_log(logs_task, LogLevel::Success, "Session still valid");
                                    }
                                    Ok(None) => {
                                        session_signal.set(None);
                                        details_signal.set(String::new());
                                        push_log(logs_task, LogLevel::Error, "Session expired or missing");
                                    }
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("Revalidate failed: {err}")),
                                }
                            });
                        } else {
                            push_log(revalidate_logs, LogLevel::Error, "No active session");
                        }
                    },
                    "Revalidate"
                }
                button { class: "action secondary", onclick: move |_| {
                        let mut session_signal = signout_session_signal.clone();
                        let maybe_session = {
                            let mut guard = session_signal.write();
                            guard.take()
                        };
                        if let Some(session) = maybe_session {
                            let mut details_signal = signout_details_signal.clone();
                            let logs_task = signout_logs.clone();
                            spawn(async move {
                                match session.signout().await {
                                    Ok(()) => {
                                        details_signal.set(String::new());
                                        push_log(logs_task, LogLevel::Success, "Signed out successfully");
                                    }
                                    Err((err, session_back)) => {
                                        session_signal.set(Some(session_back));
                                        push_log(logs_task, LogLevel::Error, format!("Signout failed: {err}"));
                                    }
                                }
                            });
                        } else {
                            push_log(signout_logs, LogLevel::Error, "No active session");
                        }
                    },
                    "Sign out"
                }
            }
            if !details_value.is_empty() {
                div { class: "outputs", {details_value} }
            }
        }
        }
    }
}

fn render_auth_tab(
    network_mode: Signal<NetworkMode>,
    keypair: Signal<Option<Keypair>>,
    session: Signal<Option<PubkySession>>,
    session_details: Signal<String>,
    auth_caps_input: Signal<String>,
    auth_relay_input: Signal<String>,
    auth_url_output: Signal<String>,
    auth_qr_data: Signal<Option<String>>,
    auth_status: Signal<String>,
    auth_flow: Signal<Option<PubkyAuthFlow>>,
    auth_request_input: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let caps_value = { auth_caps_input.read().clone() };
    let relay_value = { auth_relay_input.read().clone() };
    let url_value = { auth_url_output.read().clone() };
    let status_value = { auth_status.read().clone() };
    let qr_value = { auth_qr_data.read().clone() };
    let request_value = { auth_request_input.read().clone() };

    let mut caps_binding = auth_caps_input.clone();
    let mut relay_binding = auth_relay_input.clone();
    let mut request_binding = auth_request_input.clone();

    let start_network = network_mode.clone();
    let start_caps_signal = auth_caps_input.clone();
    let start_relay_signal = auth_relay_input.clone();
    let start_flow_signal = auth_flow.clone();
    let start_url_signal = auth_url_output.clone();
    let start_qr_signal = auth_qr_data.clone();
    let start_status_signal = auth_status.clone();
    let start_logs = logs.clone();

    let mut await_flow_signal = auth_flow.clone();
    let mut await_status_signal = auth_status.clone();
    let await_url_signal = auth_url_output.clone();
    let await_qr_signal = auth_qr_data.clone();
    let await_session_signal = session.clone();
    let await_details_signal = session_details.clone();
    let await_logs = logs.clone();

    let mut cancel_flow_signal = auth_flow.clone();
    let mut cancel_status_signal = auth_status.clone();
    let mut cancel_url_signal = auth_url_output.clone();
    let mut cancel_qr_signal = auth_qr_data.clone();
    let cancel_logs = logs.clone();

    let approve_network = network_mode.clone();
    let approve_keypair = keypair.clone();
    let approve_request_signal = auth_request_input.clone();
    let approve_logs = logs.clone();

    rsx! {
        div { class: "tab-body",
            section { class: "card span-2",
            h2 { "Request third-party authentication" }
            p { class: "helper-text", "Generate a pubkyauth:// link and QR code that another user can approve with their Pubky signer." }
            div { class: "form-grid",
                label {
                    "Requested capabilities"
                    input {
                        value: caps_value,
                        oninput: move |evt| caps_binding.set(evt.value()),
                        placeholder: "Example: /pub/app/:rw"
                    }
                }
                label {
                    "Relay override (optional)"
                    input {
                        value: relay_value,
                        oninput: move |evt| relay_binding.set(evt.value()),
                        placeholder: "https://your-relay.example/link/"
                    }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        let caps_text = start_caps_signal.read().clone();
                        if caps_text.trim().is_empty() {
                            push_log(start_logs.clone(), LogLevel::Error, "Provide capabilities for the request");
                            return;
                        }
                        let relay_text = start_relay_signal.read().clone();
                        let network = *start_network.read();
                        let mut flow_slot = start_flow_signal.clone();
                        let mut url_slot = start_url_signal.clone();
                        let mut qr_slot = start_qr_signal.clone();
                        let mut status_slot = start_status_signal.clone();
                        let logs_task = start_logs.clone();
                        spawn(async move {
                            let result = async move {
                                let capabilities = Capabilities::try_from(caps_text.trim())
                                    .map_err(|e| anyhow!("Invalid capabilities: {e}"))?;
                                let pubky = build_pubky(network)?;
                                let flow = if relay_text.trim().is_empty() {
                                    pubky.start_auth_flow(&capabilities)?
                                } else {
                                    let relay = Url::parse(relay_text.trim())
                                        .context("Relay URL must be valid")?;
                                    PubkyAuthFlow::builder(&capabilities)
                                        .client(pubky.client().clone())
                                        .relay(relay)
                                        .start()?
                                };
                                let auth_url = flow.authorization_url().to_string();
                                let data_url = generate_qr_data_url(&auth_url)?;
                                flow_slot.set(Some(flow));
                                url_slot.set(auth_url.clone());
                                qr_slot.set(Some(data_url));
                                status_slot.set(String::from("Awaiting remote approval..."));
                                Ok::<_, anyhow::Error>(format!("Auth flow ready: {auth_url}"))
                            };
                            match result.await {
                                Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                Err(err) => {
                                    flow_slot.set(None);
                                    url_slot.set(String::new());
                                    qr_slot.set(None);
                                    status_slot.set(String::new());
                                    push_log(logs_task, LogLevel::Error, format!("Failed to start auth flow: {err}"));
                                }
                            }
                        });
                    },
                    "Start auth flow",
                }
                button { class: "action", onclick: move |_| {
                        let maybe_flow = {
                            let mut guard = await_flow_signal.write();
                            guard.take()
                        };
                        if let Some(flow) = maybe_flow {
                            await_status_signal.set(String::from("Waiting for remote approval..."));
                            let mut url_slot = await_url_signal.clone();
                            let mut qr_slot = await_qr_signal.clone();
                            let mut status_slot = await_status_signal.clone();
                            let mut session_slot = await_session_signal.clone();
                            let mut details_slot = await_details_signal.clone();
                            let logs_task = await_logs.clone();
                            spawn(async move {
                                match flow.await_approval().await {
                                    Ok(new_session) => {
                                        let info = new_session.info().clone();
                                        details_slot.set(format_session_info(&info));
                                        session_slot.set(Some(new_session));
                                        status_slot.set(format!("Approved by {}", info.public_key()));
                                        url_slot.set(String::new());
                                        qr_slot.set(None);
                                        push_log(
                                            logs_task,
                                            LogLevel::Success,
                                            format!("Auth flow approved by {}", info.public_key()),
                                        );
                                    }
                                    Err(err) => {
                                        status_slot.set(String::from("Auth approval failed"));
                                        push_log(
                                            logs_task,
                                            LogLevel::Error,
                                            format!("Auth approval failed: {err}"),
                                        );
                                    }
                                }
                            });
                        } else {
                            push_log(await_logs, LogLevel::Error, "Start an auth flow first");
                        }
                    },
                    "Await approval",
                }
                button { class: "action secondary", onclick: move |_| {
                        let had_flow = {
                            let mut guard = cancel_flow_signal.write();
                            guard.take().is_some()
                        };
                        cancel_status_signal.set(String::new());
                        cancel_url_signal.set(String::new());
                        cancel_qr_signal.set(None);
                        if had_flow {
                            push_log(cancel_logs.clone(), LogLevel::Info, "Auth flow cancelled");
                        } else {
                            push_log(cancel_logs, LogLevel::Error, "No auth flow to cancel");
                        }
                    },
                    "Cancel",
                }
            }
            if !status_value.is_empty() {
                p { class: "auth-status", {status_value} }
            }
            if qr_value.is_some() || !url_value.trim().is_empty() {
                div { class: "qr-container",
                    if let Some(data_url) = qr_value {
                        img { src: data_url, alt: "pubkyauth QR code" }
                    }
                    textarea {
                        class: "tall",
                        readonly: true,
                        value: url_value,
                        placeholder: "Generated pubkyauth:// link"
                    }
                }
            }
        }
            section { class: "card span-2",
            h2 { "Approve a pubkyauth:// request" }
            p { class: "helper-text", "Paste a request URL and approve it using the active keypair." }
            div { class: "form-grid",
                label {
                    "pubkyauth:// URL"
                    textarea {
                        class: "tall",
                        value: request_value,
                        oninput: move |evt| request_binding.set(evt.value()),
                        placeholder: "pubkyauth:///?caps=..."
                    }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        let url = approve_request_signal.read().clone();
                        if url.trim().is_empty() {
                            push_log(approve_logs.clone(), LogLevel::Error, "Paste a pubkyauth:// URL to approve");
                            return;
                        }
                        if let Some(kp) = approve_keypair.read().as_ref().cloned() {
                            let network = *approve_network.read();
                            let url_string = url.trim().to_string();
                            let logs_task = approve_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let pubky = build_pubky(network)?;
                                    let signer = pubky.signer(kp.clone());
                                    signer.approve_auth(&url_string).await?;
                                    Ok::<_, anyhow::Error>(format!(
                                        "Approved auth request with {}",
                                        kp.public_key()
                                    ))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(
                                        logs_task,
                                        LogLevel::Error,
                                        format!("Failed to approve auth request: {err}"),
                                    ),
                                }
                            });
                        } else {
                            push_log(approve_logs, LogLevel::Error, "Load or generate a keypair first");
                        }
                    },
                    "Approve request",
                }
            }
        }
        }
    }
}

fn render_storage_tab(
    network_mode: Signal<NetworkMode>,
    session: Signal<Option<PubkySession>>,
    storage_path: Signal<String>,
    storage_body: Signal<String>,
    storage_response: Signal<String>,
    public_resource: Signal<String>,
    public_response: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let path_value = { storage_path.read().clone() };
    let body_value = { storage_body.read().clone() };
    let session_response = { storage_response.read().clone() };
    let public_value = { public_resource.read().clone() };
    let public_resp = { public_response.read().clone() };

    let mut storage_path_binding = storage_path.clone();
    let mut storage_body_binding = storage_body.clone();

    let storage_session_get = session.clone();
    let storage_path_get = storage_path.clone();
    let storage_response_get = storage_response.clone();
    let storage_logs_get = logs.clone();

    let storage_session_put = session.clone();
    let storage_path_put = storage_path.clone();
    let storage_body_put = storage_body.clone();
    let storage_response_put = storage_response.clone();
    let storage_logs_put = logs.clone();

    let storage_session_delete = session.clone();
    let storage_path_delete = storage_path.clone();
    let storage_response_delete = storage_response.clone();
    let storage_logs_delete = logs.clone();

    let mut public_resource_binding = public_resource.clone();
    let public_resource_signal = public_resource.clone();
    let public_response_signal = public_response.clone();
    let public_logs = logs.clone();
    let public_network = network_mode.clone();

    rsx! {
        div { class: "tab-body",
            section { class: "card",
            h2 { "Session storage" }
            p { class: "helper-text", "Operate on authenticated storage using the active session." }
            div { class: "form-grid",
                label {
                    "Absolute path"
                    input { value: path_value.clone(), oninput: move |evt| storage_path_binding.set(evt.value()) }
                }
                label {
                    "Body"
                    textarea { class: "tall", value: body_value.clone(), oninput: move |evt| storage_body_binding.set(evt.value()) }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        if let Some(session) = storage_session_get.read().as_ref().cloned() {
                            let path = storage_path_get.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_get.clone(), LogLevel::Error, "Provide a path to GET");
                                return;
                            }
                            let mut response_signal = storage_response_get.clone();
                            let logs_task = storage_logs_get.clone();
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().get(path.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Fetched {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("GET failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_get, LogLevel::Error, "No active session");
                        }
                    },
                    "GET"
                }
                button { class: "action secondary", onclick: move |_| {
                        if let Some(session) = storage_session_put.read().as_ref().cloned() {
                            let path = storage_path_put.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_put.clone(), LogLevel::Error, "Provide a path to PUT");
                                return;
                            }
                            let body = storage_body_put.read().clone();
                            let mut response_signal = storage_response_put.clone();
                            let logs_task = storage_logs_put.clone();
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().put(path.clone(), body.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Stored {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("PUT failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_put, LogLevel::Error, "No active session");
                        }
                    },
                    "PUT"
                }
                button { class: "action secondary", onclick: move |_| {
                        if let Some(session) = storage_session_delete.read().as_ref().cloned() {
                            let path = storage_path_delete.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_delete.clone(), LogLevel::Error, "Provide a path to DELETE");
                                return;
                            }
                            let mut response_signal = storage_response_delete.clone();
                            let logs_task = storage_logs_delete.clone();
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().delete(path.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Deleted {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("DELETE failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_delete, LogLevel::Error, "No active session");
                        }
                    },
                    "DELETE"
                }
            }
            if !session_response.is_empty() {
                div { class: "outputs", {session_response} }
            }
        }
            section { class: "card",
            h2 { "Public storage" }
            p { class: "helper-text", "Fetch any public resource (pubky<pk>/path or pubky://...)." }
            div { class: "form-grid",
                label {
                    "Resource"
                    input { value: public_value.clone(), oninput: move |evt| public_resource_binding.set(evt.value()) }
                }
            }
            div { class: "small-buttons",
                button { class: "action", onclick: move |_| {
                        let resource = public_resource_signal.read().clone();
                        if resource.trim().is_empty() {
                            push_log(public_logs.clone(), LogLevel::Error, "Provide a resource to fetch");
                            return;
                        }
                        let mut response_signal = public_response_signal.clone();
                        let logs_task = public_logs.clone();
                        let network = *public_network.read();
                        spawn(async move {
                            let result = async move {
                                let pubky = build_pubky(network)?;
                                let resp = pubky.public_storage().get(resource.clone()).await?;
                                let formatted = format_response(resp).await?;
                                response_signal.set(formatted.clone());
                                Ok::<_, anyhow::Error>(format!("Fetched public resource {resource}"))
                            };
                            match result.await {
                                Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                Err(err) => push_log(logs_task, LogLevel::Error, format!("Public GET failed: {err}")),
                            }
                        });
                    },
                    "GET"
                }
            }
            if !public_resp.is_empty() {
                div { class: "outputs", {public_resp} }
            }
        }
        }
    }
}

fn render_http_tab(
    network_mode: Signal<NetworkMode>,
    http_method: Signal<String>,
    http_url: Signal<String>,
    http_headers: Signal<String>,
    http_body: Signal<String>,
    http_response: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let method_value = { http_method.read().clone() };
    let url_value = { http_url.read().clone() };
    let headers_value = { http_headers.read().clone() };
    let body_value = { http_body.read().clone() };
    let response_value = { http_response.read().clone() };

    let mut method_binding = http_method.clone();
    let mut url_binding = http_url.clone();
    let mut headers_binding = http_headers.clone();
    let mut body_binding = http_body.clone();

    let request_method_signal = http_method.clone();
    let request_url_signal = http_url.clone();
    let request_headers_signal = http_headers.clone();
    let request_body_signal = http_body.clone();
    let request_response_signal = http_response.clone();
    let request_logs = logs.clone();
    let request_network = network_mode.clone();

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Raw Pubky/HTTPS request" }
                div { class: "form-grid",
                    label {
                        "Method"
                        select {
                            value: method_value.clone(),
                            oninput: move |evt| method_binding.set(evt.value()),
                            for option in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                                option { value: option, selected: method_value == option, "{option}" }
                            }
                        }
                    }
                    label {
                        "URL"
                        input {
                            value: url_value.clone(),
                            oninput: move |evt| url_binding.set(evt.value()),
                            placeholder: "https:// or pubky://",
                        }
                    }
                }
                div { class: "form-grid",
                    label {
                        "Headers (one per line, Name: Value)"
                        textarea {
                            class: "tall",
                            value: headers_value.clone(),
                            oninput: move |evt| headers_binding.set(evt.value()),
                            placeholder: "Header-Name: value",
                        }
                    }
                    label {
                        "Body"
                        textarea {
                            class: "tall",
                            value: body_value.clone(),
                            oninput: move |evt| body_binding.set(evt.value()),
                            placeholder: "Request body (optional)",
                        }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let method = request_method_signal.read().clone();
                            let url = request_url_signal.read().clone();
                            if url.trim().is_empty() {
                                push_log(request_logs.clone(), LogLevel::Error, "Provide a URL");
                                return;
                            }
                            let headers = request_headers_signal.read().clone();
                            let body = request_body_signal.read().clone();
                            let mut response_signal = request_response_signal.clone();
                            let logs_task = request_logs.clone();
                            let network = *request_network.read();
                            spawn(async move {
                                let result = async move {
                                    let method_parsed = Method::from_bytes(method.as_bytes())
                                        .map_err(|e| anyhow!("Invalid HTTP method: {e}"))?;
                                    let parsed_url = Url::parse(&url)?;
                                    let url_display = parsed_url.to_string();
                                    let client = match network {
                                        NetworkMode::Mainnet => PubkyHttpClient::new()?,
                                        NetworkMode::Testnet => PubkyHttpClient::testnet()?,
                                    };
                                    let mut request = client.request(method_parsed.clone(), parsed_url);
                                    for line in headers.lines() {
                                        if line.trim().is_empty() {
                                            continue;
                                        }
                                        let (name, value) = line
                                            .split_once(':')
                                            .ok_or_else(|| anyhow!("Header must use Name: Value format"))?;
                                        let header_name: HeaderName = name.trim().parse()?;
                                        request = request.header(header_name, value.trim());
                                    }
                                    if !body.is_empty() {
                                        request = request.body(body.clone());
                                    }
                                    let response = request.send().await?;
                                    let formatted = format_response(response).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("{method_parsed} {url_display}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, format!("Request completed: {msg}")),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("Request failed: {err}")),
                                }
                            });
                        },
                        "Send"
                    }
                }
                if !response_value.is_empty() {
                    div { class: "outputs", {response_value} }
                }
            }
        }
    }
}

fn push_log(mut logs: Signal<Vec<LogEntry>>, level: LogLevel, message: impl Into<String>) {
    let mut entries = logs.write();
    entries.push(LogEntry {
        level,
        message: message.into(),
    });
    if entries.len() > 200 {
        let drop = entries.len() - 200;
        entries.drain(0..drop);
    }
}

fn decode_secret_key(value: &str) -> Result<Keypair> {
    let bytes = STANDARD
        .decode(value.trim())
        .context("secret key must be valid base64")?;
    let secret: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("secret key must be 32 bytes"))?;
    Ok(Keypair::from_secret_key(&secret))
}

fn load_keypair_from_recovery(path: impl AsRef<Path>, passphrase: &str) -> Result<Keypair> {
    let bytes = fs::read(path.as_ref())
        .with_context(|| format!("failed to read {}", path.as_ref().display()))?;
    let kp = recovery_file::decrypt_recovery_file(&bytes, passphrase)?;
    Ok(kp)
}

fn save_keypair_to_recovery_file(
    keypair: &Keypair,
    path: &str,
    passphrase: &str,
) -> Result<PathBuf> {
    let normalized = normalize_pkarr_path(path)?;
    if let Some(parent) = normalized.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
    }
    let bytes = recovery_file::create_recovery_file(keypair, passphrase);
    fs::write(&normalized, bytes)
        .with_context(|| format!("failed to write {}", normalized.display()))?;
    Ok(normalized)
}

fn normalize_pkarr_path(input: &str) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("path cannot be empty"));
    }

    let mut expanded = if let Some(stripped) = trimmed.strip_prefix('~') {
        let home = resolve_home_dir().context("unable to resolve home directory")?;
        if stripped.starts_with('/') || stripped.starts_with('\\') {
            home.join(&stripped[1..])
        } else if stripped.is_empty() {
            home
        } else {
            home.join(stripped)
        }
    } else {
        PathBuf::from(trimmed)
    };

    let needs_extension = expanded
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase() != "pkarr")
        .unwrap_or(true);
    if needs_extension {
        expanded.set_extension("pkarr");
    }

    Ok(expanded)
}

fn resolve_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn generate_qr_data_url(content: &str) -> Result<String> {
    let code = QrCode::new(content.as_bytes()).context("failed to encode QR code")?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(280, 280)
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#f8fafc"))
        .build();
    let encoded = STANDARD.encode(svg.as_bytes());
    Ok(format!("data:image/svg+xml;base64,{encoded}"))
}

fn build_pubky(mode: NetworkMode) -> Result<Pubky> {
    match mode {
        NetworkMode::Mainnet => Ok(Pubky::new()?),
        NetworkMode::Testnet => Ok(Pubky::testnet()?),
    }
}

async fn format_response(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let version = response.version();
    let mut headers = Vec::new();
    for (name, value) in response.headers().iter() {
        if let Ok(text) = value.to_str() {
            headers.push(format!("{}: {}", name, text));
        }
    }
    let bytes = response.bytes().await?;
    let body = match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => format!("<binary {} bytes>", bytes.len()),
    };
    Ok(format!(
        "< {:?} {}\n{}\n\n{}",
        version,
        status,
        headers.join("\n"),
        body
    ))
}

fn format_session_info(info: &impl std::fmt::Debug) -> String {
    format!("{info:#?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use base64::engine::general_purpose::STANDARD;
    use std::ffi::OsString;
    use std::path::Path;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set_path(key: &'static str, value: &Path) -> Self {
            let original = std::env::var_os(key);
            // `std::env::set_var` is `unsafe` on the 2024 edition surface while
            // the standard library finalises its strictly-checked contract, so
            // keep the unsafety contained to this helper.
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(ref value) = self.original {
                unsafe {
                    std::env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn decode_secret_key_roundtrip() -> Result<()> {
        let secret = [0x42u8; 32];
        let encoded = STANDARD.encode(secret);
        let decoded = decode_secret_key(&encoded)?;
        assert_eq!(decoded.secret_key(), secret);
        Ok(())
    }

    #[test]
    fn decode_secret_key_rejects_invalid_base64() {
        let err = decode_secret_key("not-base64").unwrap_err();
        assert!(err.to_string().contains("base64"));
    }

    #[test]
    fn normalize_pkarr_path_adds_extension_and_expands_home() -> Result<()> {
        let home = TempDir::new()?;
        let _guard_home = EnvGuard::set_path("HOME", home.path());
        let _guard_profile = EnvGuard::remove("USERPROFILE");

        let normalized = normalize_pkarr_path("~/keys/my-key")?;
        assert!(normalized.starts_with(home.path()));
        assert_eq!(
            normalized.extension().and_then(|ext| ext.to_str()),
            Some("pkarr")
        );
        assert_eq!(
            normalized.file_name().and_then(|name| name.to_str()),
            Some("my-key.pkarr")
        );
        Ok(())
    }

    #[test]
    fn normalize_pkarr_path_keeps_existing_extension() -> Result<()> {
        let path = normalize_pkarr_path("/tmp/example.PKARR")?;
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("PKARR"));
        Ok(())
    }

    #[test]
    fn normalize_pkarr_path_rejects_empty_input() {
        let err = normalize_pkarr_path("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn generate_qr_data_url_encodes_svg() -> Result<()> {
        let qr = generate_qr_data_url("pubkyauth://example")?;
        assert!(qr.starts_with("data:image/svg+xml;base64,"));
        let encoded = qr.trim_start_matches("data:image/svg+xml;base64,");
        let svg_bytes = STANDARD.decode(encoded)?;
        let svg = String::from_utf8(svg_bytes).expect("qr svg should be utf8");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#0f172a"), "dark color should be embedded");
        assert!(svg.contains("#f8fafc"), "light color should be embedded");
        Ok(())
    }

    #[test]
    fn save_and_load_keypair_through_recovery_file() -> Result<()> {
        let keypair = Keypair::from_secret_key(&[7u8; 32]);
        let dir = TempDir::new()?;
        let target = dir.path().join("nested/subdir/key");
        let target_str = target.to_string_lossy();
        let saved = save_keypair_to_recovery_file(&keypair, &target_str, "passphrase")?;
        assert!(saved.exists());
        assert_eq!(
            saved.extension().and_then(|ext| ext.to_str()),
            Some("pkarr")
        );

        let restored = load_keypair_from_recovery(&saved, "passphrase")?;
        assert_eq!(restored.secret_key(), keypair.secret_key());
        Ok(())
    }
}
