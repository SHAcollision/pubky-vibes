use std::{
    env, fmt, fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, LazyLock},
};

use anyhow::{Context, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use dioxus::signals::{SignalData, Storage};
use directories::ProjectDirs;
use pubky_homeserver::{ConfigToml, Domain, HomeserverSuite, LoggingToml, SignupMode};
use pubky_testnet::StaticTestnet;

const STYLE: &str = r#"
:root {
    color-scheme: dark light;
    font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
    background-color: #030b14;
}

body {
    margin: 0;
    min-height: 100vh;
    background:
        radial-gradient(120% 100% at 20% 0%, rgba(0, 163, 255, 0.25), transparent 60%),
        radial-gradient(80% 120% at 85% 10%, rgba(0, 255, 200, 0.15), transparent 70%),
        linear-gradient(180deg, #020712 0%, #041b2d 45%, #02060d 100%);
    color: #f0f6ff;
}

main.app {
    max-width: 960px;
    margin: 0 auto;
    padding: 48px 40px 96px;
    display: flex;
    flex-direction: column;
    gap: 36px;
}

.hero {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 32px;
    padding: 40px 44px;
    border-radius: 28px;
    background: linear-gradient(135deg, rgba(3, 24, 40, 0.92), rgba(6, 46, 68, 0.78));
    border: 1px solid rgba(0, 194, 255, 0.28);
    box-shadow: 0 32px 60px rgba(6, 32, 56, 0.35);
    overflow: hidden;
}

.hero::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 15% 20%, rgba(0, 194, 255, 0.28), transparent 55%);
    mix-blend-mode: screen;
    pointer-events: none;
}

.hero h1 {
    font-size: clamp(2.75rem, 4vw, 3.4rem);
    margin: 0 0 12px;
    letter-spacing: -0.01em;
}

.hero img {
    width: clamp(200px, 26vw, 260px);
    height: clamp(200px, 26vw, 260px);
    filter: drop-shadow(0 16px 48px rgba(0, 205, 255, 0.55));
    z-index: 1;
}

.hero-content {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 480px;
    z-index: 1;
}

.hero p {
    margin: 0;
    font-size: 1.2rem;
    color: rgba(237, 246, 255, 0.78);
    line-height: 1.6;
}

.controls {
    background: linear-gradient(160deg, rgba(6, 30, 48, 0.85), rgba(3, 19, 32, 0.85));
    border: 1px solid rgba(0, 194, 255, 0.28);
    border-radius: 24px;
    padding: 32px 36px;
    display: flex;
    flex-direction: column;
    gap: 28px;
    box-shadow: 0 28px 60px rgba(3, 18, 32, 0.4);
    backdrop-filter: blur(22px);
}

.network-selector {
    display: flex;
    flex-direction: column;
    gap: 18px;
}

.network-options {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
}

.network-option {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(5, 24, 38, 0.85);
    border: 1px solid rgba(0, 194, 255, 0.35);
    border-radius: 999px;
    padding: 12px 18px;
    transition: border-color 180ms ease, transform 180ms ease, box-shadow 180ms ease;
}

.network-option:hover {
    border-color: rgba(0, 255, 200, 0.6);
    transform: translateY(-1px);
    box-shadow: 0 12px 24px rgba(0, 200, 255, 0.25);
}

.network-option input[type="radio"] {
    width: 18px;
    height: 18px;
    accent-color: #10ffd7;
}

.controls label {
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    font-size: 0.85rem;
    color: rgba(192, 227, 255, 0.85);
}

.controls input[type="text"],
.config-field input[type="text"] {
    background: rgba(4, 18, 30, 0.95);
    border: 1px solid rgba(0, 194, 255, 0.32);
    border-radius: 16px;
    padding: 16px 18px;
    font-size: 1rem;
    color: inherit;
    transition: border-color 160ms ease, box-shadow 160ms ease;
    box-shadow: inset 0 0 0 1px rgba(0, 194, 255, 0.18);
}

.data-dir-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    margin-top: 8px;
}

.data-dir-row input[type="text"] {
    flex: 1;
    min-width: min(360px, 100%);
}

.controls input[type="text"]:focus,
.config-field input[type="text"]:focus {
    outline: none;
    border-color: rgba(16, 255, 215, 0.8);
    box-shadow: 0 0 0 2px rgba(0, 209, 255, 0.25);
}

.button-row {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
}

button.action {
    background: linear-gradient(140deg, #00e0ff, #1dd6a6);
    border: none;
    border-radius: 14px;
    padding: 14px 22px;
    font-weight: 700;
    letter-spacing: 0.01em;
    color: #042134;
    cursor: pointer;
    transition: transform 160ms ease, box-shadow 160ms ease, filter 160ms ease;
    box-shadow: 0 18px 30px rgba(0, 211, 255, 0.28);
}

button.action:hover:not([disabled]) {
    transform: translateY(-2px);
    filter: brightness(1.05);
    box-shadow: 0 20px 40px rgba(0, 211, 255, 0.35);
}

button.action[disabled] {
    opacity: 0.55;
    cursor: not-allowed;
    box-shadow: none;
}

button.secondary {
    background: rgba(5, 24, 38, 0.85);
    border: 1px solid rgba(0, 194, 255, 0.4);
    border-radius: 14px;
    padding: 12px 20px;
    font-weight: 600;
    color: #edf6ff;
    cursor: pointer;
    transition: border-color 160ms ease, transform 160ms ease, box-shadow 160ms ease;
}

button.secondary:hover:not([disabled]) {
    border-color: rgba(16, 255, 215, 0.75);
    transform: translateY(-2px);
    box-shadow: 0 12px 26px rgba(0, 200, 255, 0.25);
}

button.secondary[disabled] {
    opacity: 0.45;
    cursor: not-allowed;
}

.status-card {
    background: linear-gradient(150deg, rgba(4, 20, 34, 0.92), rgba(2, 10, 18, 0.92));
    border-radius: 22px;
    padding: 28px 30px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    gap: 18px;
    box-shadow: 0 24px 45px rgba(2, 12, 20, 0.5);
    backdrop-filter: blur(20px);
}

.status-card.running {
    border-color: rgba(0, 230, 173, 0.55);
    background: linear-gradient(150deg, rgba(5, 38, 28, 0.92), rgba(2, 18, 12, 0.92));
}

.status-card.error {
    border-color: rgba(255, 118, 118, 0.5);
    background: linear-gradient(150deg, rgba(35, 3, 8, 0.9), rgba(10, 0, 4, 0.9));
}

.status-card h2 {
    margin: 0;
    font-size: 1.5rem;
}

.status-card p {
    margin: 0;
    line-height: 1.7;
    color: rgba(230, 244, 255, 0.82);
}

.status-details ul {
    margin: 0;
    padding-left: 22px;
    display: grid;
    gap: 10px;
}

.status-details a {
    color: #5be4ff;
    text-decoration: none;
}

.status-details a:hover {
    text-decoration: underline;
}

pre.public-key {
    background: rgba(1, 6, 12, 0.6);
    border-radius: 14px;
    padding: 14px 18px;
    overflow-x: auto;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.9rem;
    border: 1px solid rgba(0, 194, 255, 0.25);
}

.config-editor {
    display: flex;
    flex-direction: column;
    gap: 24px;
    background: rgba(3, 16, 27, 0.85);
    border: 1px solid rgba(0, 194, 255, 0.24);
    border-radius: 20px;
    padding: 28px 30px;
    box-shadow: 0 24px 50px rgba(3, 20, 32, 0.42);
    backdrop-filter: blur(20px);
}

.config-grid {
    display: grid;
    gap: 20px 28px;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
}

.config-editor-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
}

.config-field {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.config-feedback {
    border-radius: 12px;
    padding: 14px 16px;
    font-size: 0.95rem;
}

.config-feedback.success {
    background: rgba(0, 230, 173, 0.18);
    color: #8cffe5;
    border: 1px solid rgba(0, 230, 173, 0.4);
}

.config-feedback.error {
    background: rgba(255, 118, 118, 0.16);
    color: #ffc2c2;
    border: 1px solid rgba(255, 118, 118, 0.45);
}

.signup-mode-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.signup-mode-options {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
}

.signup-mode-option {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(5, 24, 38, 0.85);
    border: 1px solid rgba(0, 194, 255, 0.35);
    border-radius: 14px;
    padding: 12px 16px;
    transition: border-color 160ms ease, transform 160ms ease;
}

.signup-mode-option:hover {
    border-color: rgba(16, 255, 215, 0.75);
    transform: translateY(-1px);
}

.footnote {
    font-size: 0.9rem;
    color: rgba(237, 246, 255, 0.7);
    line-height: 1.6;
}

.footnote code {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    background: rgba(0, 0, 0, 0.4);
    padding: 2px 8px;
    border-radius: 8px;
}
"#;

static LOGO_DATA_URI: LazyLock<String> = LazyLock::new(|| {
    let encoded = STANDARD.encode(include_bytes!("../assets/pubky-core-logo.svg"));
    format!("data:image/svg+xml;base64,{}", encoded)
});

#[derive(Clone, Debug, PartialEq)]
enum ServerStatus {
    Idle,
    Starting,
    Running(ServerInfo),
    Stopping,
    Error(String),
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ServerInfo {
    public_key: String,
    admin_url: String,
    icann_http_url: String,
    pubky_url: String,
    network: NetworkProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkProfile {
    Mainnet,
    Testnet,
}

impl NetworkProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Mainnet => "Mainnet",
            Self::Testnet => "Static Testnet",
        }
    }
}

impl fmt::Display for NetworkProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
enum RunningServer {
    Mainnet(Arc<HomeserverSuite>),
    Testnet(Arc<StaticTestnet>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StartSpec {
    Mainnet { data_dir: PathBuf },
    Testnet,
}

#[derive(Debug, PartialEq, Eq)]
enum StartValidationError {
    MissingDataDir,
}

impl fmt::Display for StartValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartValidationError::MissingDataDir => f.write_str(
                "Please provide a directory where the homeserver can persist its state.",
            ),
        }
    }
}

fn resolve_start_spec(
    network: NetworkProfile,
    data_dir: &str,
) -> Result<StartSpec, StartValidationError> {
    match network {
        NetworkProfile::Mainnet => {
            let trimmed = data_dir.trim();
            if trimmed.is_empty() {
                return Err(StartValidationError::MissingDataDir);
            }

            Ok(StartSpec::Mainnet {
                data_dir: PathBuf::from(trimmed),
            })
        }
        NetworkProfile::Testnet => Ok(StartSpec::Testnet),
    }
}

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(WindowBuilder::new().with_title("Portable Pubky Homeserver")),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let initial_data_dir = default_data_dir();
    let initial_config_state = config_state_from_dir(&initial_data_dir);

    let mut data_dir = use_signal_sync(|| initial_data_dir.clone());
    let status = use_signal_sync(ServerStatus::default);
    let suite_handle = use_signal_sync(|| Option::<RunningServer>::None);
    let mut network = use_signal_sync(|| NetworkProfile::Mainnet);
    let config_state = use_signal_sync(|| initial_config_state.clone());

    let start_disabled = matches!(
        *status.peek(),
        ServerStatus::Starting | ServerStatus::Running(_) | ServerStatus::Stopping
    );
    let stop_disabled = matches!(
        *status.peek(),
        ServerStatus::Idle | ServerStatus::Starting | ServerStatus::Stopping
    );
    let restart_blocked = matches!(
        *status.peek(),
        ServerStatus::Starting | ServerStatus::Stopping
    );

    let start_server = {
        let data_dir_signal = data_dir;
        let mut status_signal = status;
        let mut suite_signal = suite_handle;
        let network_signal = network;

        move |_| {
            let selection = *network_signal.read();
            let data_dir_value = data_dir_signal.read().to_string();
            let start_spec = match resolve_start_spec(selection, &data_dir_value) {
                Ok(spec) => spec,
                Err(err) => {
                    *status_signal.write() = ServerStatus::Error(err.to_string());
                    return;
                }
            };

            suite_signal.write().take();
            spawn_start_task(start_spec, status_signal, suite_signal);
        }
    };

    let stop_server = {
        let status_signal = status;
        let suite_signal = suite_handle;

        move |_| {
            stop_current_server(status_signal, suite_signal);
        }
    };

    let load_config = {
        let data_dir_signal = data_dir;
        let mut config_signal = config_state;

        move |_| {
            let dir = data_dir_signal.read().to_string();
            match load_config_form_from_dir(&dir) {
                Ok(form) => {
                    let mut state = config_signal.write();
                    state.form = form;
                    state.dirty = false;
                    state.feedback = None;
                }
                Err(err) => {
                    let mut state = config_signal.write();
                    state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                }
            }
        }
    };

    let save_and_restart = {
        let mut config_signal = config_state;
        let data_dir_signal = data_dir;
        let status_signal = status;
        let suite_signal = suite_handle;
        let network_signal = network;

        move |_| {
            let form_snapshot = {
                let state = config_signal.read();
                state.form.clone()
            };
            let dir = data_dir_signal.read().to_string();

            match persist_config_form(&dir, &form_snapshot) {
                Ok(_) => {
                    let selection = *network_signal.read();
                    let start_spec = match resolve_start_spec(selection, &dir) {
                        Ok(spec) => spec,
                        Err(err) => {
                            let mut state = config_signal.write();
                            state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                            return;
                        }
                    };

                    {
                        let mut state = config_signal.write();
                        state.dirty = false;
                        state.feedback = Some(ConfigFeedback::Saved);
                    }

                    stop_current_server(status_signal, suite_signal);
                    spawn_start_task(start_spec, status_signal, suite_signal);
                }
                Err(err) => {
                    let mut state = config_signal.write();
                    state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                }
            }
        }
    };

    let data_dir_value = data_dir.read().to_string();
    let status_snapshot = status.read().clone();
    let selected_network = *network.read();
    let config_state_snapshot = config_state.read().clone();
    let ConfigForm {
        signup_mode,
        drive_pubky_listen_socket,
        drive_icann_listen_socket,
        admin_listen_socket,
        admin_password,
        pkdns_public_ip,
        pkdns_public_pubky_tls_port,
        pkdns_public_icann_http_port,
        pkdns_icann_domain,
        logging_level,
    } = config_state_snapshot.form.clone();
    let config_feedback = config_state_snapshot.feedback.clone();
    let save_disabled = restart_blocked || !config_state_snapshot.dirty;

    let config_state_signup_token = config_state;
    let config_state_signup_open = config_state;
    let config_state_pubky = config_state;
    let config_state_icann = config_state;
    let config_state_admin_socket = config_state;
    let config_state_admin_password = config_state;
    let config_state_public_ip = config_state;
    let config_state_tls_port = config_state;
    let config_state_http_port = config_state;
    let config_state_icann_domain = config_state;
    let config_state_logging = config_state;

    rsx! {
        style { "{STYLE}" }
        main { class: "app",
            div { class: "hero",
                img {
                    src: LazyLock::force(&LOGO_DATA_URI).as_str(),
                    alt: "Pubky logo",
                }
                div { class: "hero-content",
                    h1 { "Portable Pubky Homeserver" }
                    p { "It's your data, bring it with you." }
                }
            }

            section { class: "controls",
                div { class: "network-selector",
                    label { "Select network" }
                    div { class: "network-options",
                        label { class: "network-option",
                            input {
                                r#type: "radio",
                                name: "network",
                                value: "mainnet",
                                checked: matches!(selected_network, NetworkProfile::Mainnet),
                                onchange: move |_| {
                                    *network.write() = NetworkProfile::Mainnet;
                                },
                            }
                            span { "Mainnet" }
                        }
                        label { class: "network-option",
                            input {
                                r#type: "radio",
                                name: "network",
                                value: "testnet",
                                checked: matches!(selected_network, NetworkProfile::Testnet),
                                onchange: move |_| {
                                    *network.write() = NetworkProfile::Testnet;
                                },
                            }
                            span { "Static Testnet" }
                        }
                    }
                    p { class: "footnote",
                        "Testnet runs a local DHT, relays, and homeserver with fixed ports using pubky-testnet."
                    }
                }

                div {
                    label { r#"Data directory"# }
                    div { class: "data-dir-row",
                        input {
                            r#type: "text",
                            value: "{data_dir_value}",
                            placeholder: r#"~/Library/Application Support/Pubky"#,
                            oninput: move |evt| {
                                let value = evt.value();
                                *data_dir.write() = value;
                            }
                        }
                    }
                    p { class: "footnote",
                        "Config, logs, and keys live inside this folder. The homeserver will create missing files automatically."
                    }
                }

                div { class: "config-editor",
                    div { class: "config-editor-header",
                        label { "Homeserver configuration" }
                        button { class: "secondary", onclick: load_config, "Reload from disk" }
                    }

                    div { class: "signup-mode-group",
                        span { "Signup mode" }
                        div { class: "signup-mode-options",
                            label { class: "signup-mode-option",
                                input {
                                    r#type: "radio",
                                    name: "signup-mode",
                                    value: "token_required",
                                    checked: matches!(signup_mode, SignupMode::TokenRequired),
                                    onchange: move |_| {
                                        modify_config_form(config_state_signup_token, |form| {
                                            form.signup_mode = SignupMode::TokenRequired;
                                        });
                                    }
                                }
                                span { "Token required" }
                            }
                            label { class: "signup-mode-option",
                                input {
                                    r#type: "radio",
                                    name: "signup-mode",
                                    value: "open",
                                    checked: matches!(signup_mode, SignupMode::Open),
                                    onchange: move |_| {
                                        modify_config_form(config_state_signup_open, |form| {
                                            form.signup_mode = SignupMode::Open;
                                        });
                                    }
                                }
                                span { "Open signup" }
                            }
                        }
                    }

                    div { class: "config-grid",
                        div { class: "config-field",
                            label { "Pubky TLS listen socket" }
                            input {
                                r#type: "text",
                                value: "{drive_pubky_listen_socket}",
                                placeholder: "127.0.0.1:6287",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_pubky, |form| {
                                        form.drive_pubky_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "ICANN HTTP listen socket" }
                            input {
                                r#type: "text",
                                value: "{drive_icann_listen_socket}",
                                placeholder: "127.0.0.1:6286",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_icann, |form| {
                                        form.drive_icann_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Admin listen socket" }
                            input {
                                r#type: "text",
                                value: "{admin_listen_socket}",
                                placeholder: "127.0.0.1:6288",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_admin_socket, |form| {
                                        form.admin_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Admin password" }
                            input {
                                r#type: "text",
                                value: "{admin_password}",
                                placeholder: "admin",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_admin_password, |form| {
                                        form.admin_password = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public IP address" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_ip}",
                                placeholder: "127.0.0.1",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_public_ip, |form| {
                                        form.pkdns_public_ip = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public Pubky TLS port" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_pubky_tls_port}",
                                placeholder: "6287",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_tls_port, |form| {
                                        form.pkdns_public_pubky_tls_port = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public ICANN HTTP port" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_icann_http_port}",
                                placeholder: "80",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_http_port, |form| {
                                        form.pkdns_public_icann_http_port = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "ICANN domain" }
                            input {
                                r#type: "text",
                                value: "{pkdns_icann_domain}",
                                placeholder: "example.com",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_icann_domain, |form| {
                                        form.pkdns_icann_domain = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Logging level override" }
                            input {
                                r#type: "text",
                                value: "{logging_level}",
                                placeholder: "info",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_logging, |form| {
                                        form.logging_level = value;
                                    });
                                }
                            }
                        }
                    }

                    if let Some(feedback) = config_feedback.clone() {
                        match feedback {
                            ConfigFeedback::Saved => rsx! {
                                div { class: "config-feedback success",
                                    p { "Configuration saved. Restarting homeserver..." }
                                }
                            },
                            ConfigFeedback::Error(message) => rsx! {
                                div { class: "config-feedback error", "{message}" }
                            },
                        }
                    }

                    div { class: "button-row",
                        button {
                            class: "action",
                            disabled: save_disabled,
                            onclick: save_and_restart,
                            "Save & Restart"
                        }
                    }
                }

                div { class: "button-row",
                    button {
                        class: "action",
                        disabled: start_disabled,
                        onclick: start_server,
                        "Start server"
                    }
                    button {
                        class: "action",
                        disabled: stop_disabled,
                        onclick: stop_server,
                        "Stop server"
                    }
                }
            }

            StatusPanel { status: status_snapshot.clone() }

            div { class: "footnote",
                "Tip: keep this window open while the homeserver is running. Close it to gracefully stop Pubky." }
            div { class: "footnote",
                "Power users can tweak advanced settings in ",
                code { "{data_dir_value}/config.toml" },
                "."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigForm {
    signup_mode: SignupMode,
    drive_pubky_listen_socket: String,
    drive_icann_listen_socket: String,
    admin_listen_socket: String,
    admin_password: String,
    pkdns_public_ip: String,
    pkdns_public_pubky_tls_port: String,
    pkdns_public_icann_http_port: String,
    pkdns_icann_domain: String,
    logging_level: String,
}

impl ConfigForm {
    fn from_config(config: &ConfigToml) -> Self {
        Self {
            signup_mode: config.general.signup_mode.clone(),
            drive_pubky_listen_socket: config.drive.pubky_listen_socket.to_string(),
            drive_icann_listen_socket: config.drive.icann_listen_socket.to_string(),
            admin_listen_socket: config.admin.listen_socket.to_string(),
            admin_password: config.admin.admin_password.clone(),
            pkdns_public_ip: config.pkdns.public_ip.to_string(),
            pkdns_public_pubky_tls_port: config
                .pkdns
                .public_pubky_tls_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
            pkdns_public_icann_http_port: config
                .pkdns
                .public_icann_http_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
            pkdns_icann_domain: config
                .pkdns
                .icann_domain
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default(),
            logging_level: config
                .logging
                .as_ref()
                .map(|logging| logging.level.to_string())
                .unwrap_or_default(),
        }
    }

    fn default() -> Self {
        Self::from_config(&ConfigToml::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigState {
    form: ConfigForm,
    dirty: bool,
    feedback: Option<ConfigFeedback>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConfigFeedback {
    Saved,
    Error(String),
}

fn load_config_form_from_dir(data_dir: &str) -> anyhow::Result<ConfigForm> {
    if data_dir.trim().is_empty() {
        return Ok(ConfigForm::default());
    }

    let config_path = Path::new(data_dir).join("config.toml");
    if config_path.is_file() {
        let config = ConfigToml::from_file(&config_path)
            .map_err(|err| anyhow!("Failed to read {}: {}", config_path.display(), err))?;
        Ok(ConfigForm::from_config(&config))
    } else {
        Ok(ConfigForm::default())
    }
}

fn config_state_from_dir(data_dir: &str) -> ConfigState {
    match load_config_form_from_dir(data_dir) {
        Ok(form) => ConfigState {
            form,
            dirty: false,
            feedback: None,
        },
        Err(err) => ConfigState {
            form: ConfigForm::default(),
            dirty: false,
            feedback: Some(ConfigFeedback::Error(err.to_string())),
        },
    }
}

fn persist_config_form(data_dir: &str, form: &ConfigForm) -> anyhow::Result<()> {
    let trimmed = data_dir.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "Please provide a directory where we can write config.toml."
        ));
    }

    let dir_path = PathBuf::from(trimmed);
    let config_path = dir_path.join("config.toml");

    let mut config = if config_path.is_file() {
        ConfigToml::from_file(&config_path)
            .map_err(|err| anyhow!("Failed to parse {}: {}", config_path.display(), err))?
    } else {
        ConfigToml::default()
    };

    apply_config_form(form, &mut config)?;

    fs::create_dir_all(&dir_path)
        .with_context(|| format!("Failed to create data directory at {}", dir_path.display()))?;

    let rendered =
        toml::to_string_pretty(&config).context("Failed to render config as TOML text")?;
    fs::write(&config_path, rendered)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(())
}

fn apply_config_form(form: &ConfigForm, config: &mut ConfigToml) -> anyhow::Result<()> {
    config.general.signup_mode = form.signup_mode.clone();

    config.drive.pubky_listen_socket =
        parse_socket("Pubky TLS listen socket", &form.drive_pubky_listen_socket)?;
    config.drive.icann_listen_socket =
        parse_socket("ICANN HTTP listen socket", &form.drive_icann_listen_socket)?;

    config.admin.listen_socket = parse_socket("Admin listen socket", &form.admin_listen_socket)?;
    config.admin.admin_password = form.admin_password.clone();

    config.pkdns.public_ip = parse_ip("Public IP", &form.pkdns_public_ip)?;
    config.pkdns.public_pubky_tls_port =
        parse_optional_port("Public Pubky TLS port", &form.pkdns_public_pubky_tls_port)?;
    config.pkdns.public_icann_http_port =
        parse_optional_port("Public ICANN HTTP port", &form.pkdns_public_icann_http_port)?;
    config.pkdns.icann_domain = parse_optional_domain(&form.pkdns_icann_domain)?;

    let logging = parse_logging_level(&form.logging_level, config.logging.clone())?;
    config.logging = logging;

    Ok(())
}

fn parse_socket(label: &str, raw: &str) -> anyhow::Result<SocketAddr> {
    raw.trim()
        .parse()
        .map_err(|err| anyhow!("{} must be in host:port format ({}).", label, err))
}

fn parse_ip(label: &str, raw: &str) -> anyhow::Result<IpAddr> {
    raw.trim()
        .parse()
        .map_err(|err| anyhow!("{} is not a valid IP address ({}).", label, err))
}

fn parse_optional_port(label: &str, raw: &str) -> anyhow::Result<Option<u16>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse()
        .map(Some)
        .map_err(|err| anyhow!("{} must be a port number ({}).", label, err))
}

fn parse_optional_domain(raw: &str) -> anyhow::Result<Option<Domain>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Domain::from_str(trimmed)
        .map(Some)
        .map_err(|err| anyhow!("Invalid domain '{}': {}", trimmed, err))
}

fn parse_logging_level(
    raw: &str,
    existing: Option<LoggingToml>,
) -> anyhow::Result<Option<LoggingToml>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(existing.map(|mut logging| {
            logging.level = LoggingToml::default().level;
            logging
        }));
    }

    let parsed = trimmed.parse().map_err(|err| {
        anyhow!(
            "Invalid logging level '{}': {}. Use trace, debug, info, warn, or error.",
            trimmed,
            err
        )
    })?;

    let mut logging = existing.unwrap_or_default();
    logging.level = parsed;
    Ok(Some(logging))
}

fn modify_config_form<F, S>(mut state: Signal<ConfigState, S>, update: F)
where
    F: FnOnce(&mut ConfigForm),
    S: Storage<SignalData<ConfigState>> + 'static,
{
    let mut guard = state.write();
    update(&mut guard.form);
    guard.dirty = true;
    guard.feedback = None;
}

fn stop_current_server<S1, S2>(
    mut status_signal: Signal<ServerStatus, S1>,
    mut suite_signal: Signal<Option<RunningServer>, S2>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    let was_active = matches!(
        *status_signal.peek(),
        ServerStatus::Running(_) | ServerStatus::Starting
    );

    if was_active {
        *status_signal.write() = ServerStatus::Stopping;
    }

    suite_signal.write().take();
    *status_signal.write() = ServerStatus::Idle;
}

fn spawn_start_task<S1, S2>(
    start_spec: StartSpec,
    mut status_signal: Signal<ServerStatus, S1>,
    suite_signal: Signal<Option<RunningServer>, S2>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    *status_signal.write() = ServerStatus::Starting;

    let mut status_for_task = status_signal;
    let mut suite_for_task = suite_signal;

    spawn(async move {
        let result: anyhow::Result<(RunningServer, ServerInfo)> = async {
            match start_spec {
                StartSpec::Mainnet { data_dir } => {
                    tokio::fs::create_dir_all(&data_dir)
                        .await
                        .with_context(|| {
                            format!("Failed to create data directory at {}", data_dir.display())
                        })?;

                    let server =
                        HomeserverSuite::start_with_persistent_data_dir_path(data_dir.clone())
                            .await
                            .with_context(|| {
                                format!(
                                    "HomeserverSuite::start_with_persistent_data_dir_path({})",
                                    data_dir.display()
                                )
                            })?;

                    let info = ServerInfo {
                        public_key: server.public_key().to_string(),
                        admin_url: format!("http://{}", server.admin().listen_socket()),
                        icann_http_url: server.icann_http_url().to_string(),
                        pubky_url: server.pubky_url().to_string(),
                        network: NetworkProfile::Mainnet,
                    };

                    Ok((RunningServer::Mainnet(Arc::new(server)), info))
                }
                StartSpec::Testnet => {
                    let static_net = StaticTestnet::start()
                        .await
                        .context("StaticTestnet::start()")?;
                    let static_net = Arc::new(static_net);
                    let homeserver = static_net.homeserver();
                    let info = ServerInfo {
                        public_key: homeserver.public_key().to_string(),
                        admin_url: format!("http://{}", homeserver.admin().listen_socket()),
                        icann_http_url: homeserver.icann_http_url().to_string(),
                        pubky_url: homeserver.pubky_url().to_string(),
                        network: NetworkProfile::Testnet,
                    };

                    Ok((RunningServer::Testnet(static_net), info))
                }
            }
        }
        .await;

        match result {
            Ok((suite, info)) => {
                *suite_for_task.write() = Some(suite);
                *status_for_task.write() = ServerStatus::Running(info);
            }
            Err(err) => {
                *status_for_task.write() =
                    ServerStatus::Error(format!("Failed to start the homeserver: {err:?}"));
            }
        }
    });
}

#[component]
fn StatusPanel(status: ServerStatus) -> Element {
    let StatusCopy {
        class_name,
        heading,
        summary,
    } = status_copy(&status);

    let details_section: Option<Element> = match status_details(&status) {
        StatusDetails::Running {
            network_label,
            network_hint,
            admin_url,
            icann_url,
            pubky_url,
            public_key,
        } => Some(rsx! {
            div { class: "status-details",
                p {
                    strong { "Network:" }
                    " {network_label}"
                }
                if let Some(hint) = network_hint {
                    p { "{hint}" }
                }
                p { "Share these endpoints or bookmark them for later:" }
                ul {
                    li {
                        strong { "Admin API:" }
                        " "
                        a { href: "{admin_url}", target: "_blank", rel: "noreferrer", "{admin_url}" }
                    }
                    li {
                        strong { "ICANN HTTP:" }
                        " "
                        a { href: "{icann_url}", target: "_blank", rel: "noreferrer", "{icann_url}" }
                    }
                    li {
                        strong { "Pubky TLS:" }
                        " "
                        a { href: "{pubky_url}", target: "_blank", rel: "noreferrer", "{pubky_url}" }
                    }
                }
                p { "Public key:" }
                pre { class: "public-key", "{public_key}" }
                p { "Anyone can reach your agent with the public key above." }
            }
        }),
        StatusDetails::Error { message } => Some(rsx! {
            div { class: "status-details",
                p { "Check that the directory is writable and the config is valid." }
                pre { class: "public-key", "{message}" }
            }
        }),
        StatusDetails::Message(copy) => Some(rsx! {
            div { class: "status-details",
                p { "{copy}" }
            }
        }),
        StatusDetails::None => None,
    };

    let details_section = details_section.unwrap_or_else(|| rsx! { Fragment {} });

    rsx! {
        div { class: "status-card {class_name}",
            h2 { "{heading}" }
            p { "{summary}" }
            {details_section}
        }
    }
}

fn default_data_dir() -> String {
    if let Some(project_dirs) = ProjectDirs::from("io", "Pubky", "PortableHomeserver") {
        project_dirs.data_dir().to_string_lossy().into_owned()
    } else {
        let mut fallback = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        fallback.push(".pubky");
        fallback.to_string_lossy().into_owned()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StatusCopy {
    class_name: &'static str,
    heading: &'static str,
    summary: &'static str,
}

fn status_copy(status: &ServerStatus) -> StatusCopy {
    match status {
        ServerStatus::Idle => StatusCopy {
            class_name: "idle",
            heading: "Homeserver is idle",
            summary: "Select a storage directory and click start to bring your node online.",
        },
        ServerStatus::Starting => StatusCopy {
            class_name: "starting",
            heading: "Starting homeserver",
            summary: "Loading configuration, generating keys, and opening network ports…",
        },
        ServerStatus::Running(info) => StatusCopy {
            class_name: "running",
            heading: "Homeserver is running",
            summary: match info.network {
                NetworkProfile::Mainnet => {
                    "Your Pubky agent is online and sharing data for your communities."
                }
                NetworkProfile::Testnet => {
                    "Static testnet services are online with fixed ports and credentials."
                }
            },
        },
        ServerStatus::Stopping => StatusCopy {
            class_name: "stopping",
            heading: "Stopping homeserver",
            summary: "Shutting down services and closing sockets…",
        },
        ServerStatus::Error(_) => StatusCopy {
            class_name: "error",
            heading: "Something went wrong",
            summary: "We couldn't boot the homeserver with the current settings.",
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
enum StatusDetails {
    None,
    Message(&'static str),
    Error {
        message: String,
    },
    Running {
        network_label: String,
        network_hint: Option<&'static str>,
        admin_url: String,
        icann_url: String,
        pubky_url: String,
        public_key: String,
    },
}

fn status_details(status: &ServerStatus) -> StatusDetails {
    match status {
        ServerStatus::Idle => StatusDetails::None,
        ServerStatus::Starting => StatusDetails::Message(
            "This usually takes a few seconds – we wait for the admin and TLS endpoints to come online.",
        ),
        ServerStatus::Stopping => StatusDetails::Message(
            "Hold tight while we close the node. You can start it again once this completes.",
        ),
        ServerStatus::Error(message) => StatusDetails::Error {
            message: message.clone(),
        },
        ServerStatus::Running(info) => {
            let NetworkDisplay { label, hint } = network_display(info);
            StatusDetails::Running {
                network_label: label,
                network_hint: hint,
                admin_url: info.admin_url.clone(),
                icann_url: info.icann_http_url.clone(),
                pubky_url: info.pubky_url.clone(),
                public_key: info.public_key.clone(),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct NetworkDisplay {
    label: String,
    hint: Option<&'static str>,
}

fn network_display(info: &ServerInfo) -> NetworkDisplay {
    let label = match info.network {
        NetworkProfile::Mainnet => info.network.label().to_string(),
        NetworkProfile::Testnet => {
            format!("{} · local relays & bootstrap", info.network.label())
        }
    };

    let hint = match info.network {
        NetworkProfile::Mainnet => None,
        NetworkProfile::Testnet => {
            Some("Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.")
        }
    };

    NetworkDisplay { label, hint }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info(network: NetworkProfile) -> ServerInfo {
        ServerInfo {
            public_key: "pk_test".into(),
            admin_url: "http://localhost:6288".into(),
            icann_http_url: "http://localhost:15412".into(),
            pubky_url: "https://example.pubky".into(),
            network,
        }
    }

    #[test]
    fn config_form_roundtrip_preserves_updates() {
        let mut config = ConfigToml::default();
        config.general.signup_mode = SignupMode::Open;
        config.admin.admin_password = "changed".into();
        config.pkdns.public_ip = "192.168.1.1".parse::<IpAddr>().unwrap();

        let form = ConfigForm::from_config(&config);
        let mut applied = ConfigToml::default();
        apply_config_form(&form, &mut applied).expect("form should apply cleanly");

        assert_eq!(applied.general.signup_mode, SignupMode::Open);
        assert_eq!(applied.admin.admin_password, "changed");
        assert_eq!(
            applied.pkdns.public_ip,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn apply_config_form_rejects_invalid_port() {
        let mut form = ConfigForm::default();
        form.pkdns_public_pubky_tls_port = "invalid".into();

        let mut config = ConfigToml::default();
        let err = apply_config_form(&form, &mut config)
            .expect_err("invalid port should produce an error");

        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn persist_config_form_writes_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut form = ConfigForm::default();
        form.admin_password = "super-secure".into();

        persist_config_form(temp_dir.path().to_str().unwrap(), &form)
            .expect("config should persist");

        let saved = ConfigToml::from_file(temp_dir.path().join("config.toml"))
            .expect("config should parse");
        assert_eq!(saved.admin.admin_password, "super-secure");
    }

    #[test]
    fn resolves_mainnet_start_spec_with_trimmed_path() {
        let spec = resolve_start_spec(NetworkProfile::Mainnet, "  /tmp/pubky  ");

        assert_eq!(
            spec,
            Ok(StartSpec::Mainnet {
                data_dir: PathBuf::from("/tmp/pubky"),
            })
        );
    }

    #[test]
    fn rejects_mainnet_start_without_path() {
        let err = resolve_start_spec(NetworkProfile::Mainnet, "   ")
            .expect_err("missing directories must error");

        assert_eq!(err, StartValidationError::MissingDataDir);
        assert_eq!(
            err.to_string(),
            "Please provide a directory where the homeserver can persist its state."
        );
    }

    #[test]
    fn resolves_testnet_start_spec() {
        let spec = resolve_start_spec(NetworkProfile::Testnet, "ignored");
        assert_eq!(spec, Ok(StartSpec::Testnet));
    }

    #[test]
    fn status_copy_reflects_idle_state() {
        let copy = status_copy(&ServerStatus::Idle);

        assert_eq!(
            copy,
            StatusCopy {
                class_name: "idle",
                heading: "Homeserver is idle",
                summary: "Select a storage directory and click start to bring your node online.",
            }
        );
    }

    #[test]
    fn status_copy_reflects_running_profiles() {
        let mainnet_copy =
            status_copy(&ServerStatus::Running(sample_info(NetworkProfile::Mainnet)));
        assert_eq!(
            mainnet_copy,
            StatusCopy {
                class_name: "running",
                heading: "Homeserver is running",
                summary: "Your Pubky agent is online and sharing data for your communities.",
            }
        );

        let testnet_copy =
            status_copy(&ServerStatus::Running(sample_info(NetworkProfile::Testnet)));
        assert_eq!(
            testnet_copy,
            StatusCopy {
                class_name: "running",
                heading: "Homeserver is running",
                summary: "Static testnet services are online with fixed ports and credentials.",
            }
        );
    }

    #[test]
    fn network_display_describes_profiles() {
        let mainnet = network_display(&sample_info(NetworkProfile::Mainnet));
        assert_eq!(mainnet.label, "Mainnet");
        assert_eq!(mainnet.hint, None);

        let testnet = network_display(&sample_info(NetworkProfile::Testnet));
        assert_eq!(testnet.label, "Static Testnet · local relays & bootstrap");
        assert_eq!(
            testnet.hint,
            Some("Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.")
        );
    }

    #[test]
    fn status_details_returns_none_for_idle() {
        assert_eq!(status_details(&ServerStatus::Idle), StatusDetails::None);
    }

    #[test]
    fn status_details_returns_message_states() {
        assert_eq!(
            status_details(&ServerStatus::Starting),
            StatusDetails::Message(
                "This usually takes a few seconds – we wait for the admin and TLS endpoints to come online.",
            )
        );

        assert_eq!(
            status_details(&ServerStatus::Stopping),
            StatusDetails::Message(
                "Hold tight while we close the node. You can start it again once this completes.",
            )
        );
    }

    #[test]
    fn status_details_describes_errors() {
        let err = StatusDetails::Error {
            message: "boom".into(),
        };
        assert_eq!(status_details(&ServerStatus::Error("boom".into())), err);
    }

    #[test]
    fn status_details_summarises_running_info() {
        let info = sample_info(NetworkProfile::Testnet);
        let details = status_details(&ServerStatus::Running(info.clone()));

        assert_eq!(
            details,
            StatusDetails::Running {
                network_label: "Static Testnet · local relays & bootstrap".into(),
                network_hint: Some(
                    "Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.",
                ),
                admin_url: info.admin_url,
                icann_url: info.icann_http_url,
                pubky_url: info.pubky_url,
                public_key: info.public_key,
            }
        );
    }

    #[test]
    fn start_validation_error_formats_helpfully() {
        assert_eq!(
            StartValidationError::MissingDataDir.to_string(),
            "Please provide a directory where the homeserver can persist its state."
        );
    }
}
