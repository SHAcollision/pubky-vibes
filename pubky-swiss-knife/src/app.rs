use dioxus::prelude::*;
use pubky::{Keypair, PubkyAuthFlow, PubkySession};

use crate::components::{NetworkToggleOption, TabButton};
use crate::style::APP_STYLE;
use crate::tabs::{
    render_auth_tab, render_http_tab, render_keys_tab, render_sessions_tab, render_storage_tab,
    render_tokens_tab,
};
use crate::utils::logging::LogEntry;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Mainnet,
    Testnet,
}

impl NetworkMode {
    pub const ALL: [NetworkMode; 2] = [NetworkMode::Mainnet, NetworkMode::Testnet];

    pub fn label(self) -> &'static str {
        match self {
            NetworkMode::Mainnet => "Mainnet",
            NetworkMode::Testnet => "Testnet",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Keys,
    Tokens,
    Sessions,
    Auth,
    Storage,
    Http,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Keys,
        Tab::Tokens,
        Tab::Sessions,
        Tab::Auth,
        Tab::Storage,
        Tab::Http,
    ];

    pub fn label(self) -> &'static str {
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

#[allow(non_snake_case, clippy::clone_on_copy)]
pub fn App() -> Element {
    let active_tab = use_signal(|| Tab::Keys);
    let network_mode = use_signal(|| NetworkMode::Mainnet);
    let logs = use_signal(Vec::<LogEntry>::new);
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
        style { {APP_STYLE} }
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
                TabPanel {
                    active_tab: active_tab.clone(),
                    network_mode: network_mode.clone(),
                    keypair: keypair.clone(),
                    secret_input: secret_input.clone(),
                    recovery_path: recovery_path.clone(),
                    recovery_passphrase: recovery_passphrase.clone(),
                    token_caps_input: token_caps_input.clone(),
                    token_output: token_output.clone(),
                    session: session.clone(),
                    session_details: session_details.clone(),
                    homeserver_input: homeserver_input.clone(),
                    signup_code_input: signup_code_input.clone(),
                    auth_caps_input: auth_caps_input.clone(),
                    auth_relay_input: auth_relay_input.clone(),
                    auth_url_output: auth_url_output.clone(),
                    auth_qr_data: auth_qr_data.clone(),
                    auth_status: auth_status.clone(),
                    auth_flow: auth_flow.clone(),
                    auth_request_input: auth_request_input.clone(),
                    storage_path: storage_path.clone(),
                    storage_body: storage_body.clone(),
                    storage_response: storage_response.clone(),
                    public_resource: public_resource.clone(),
                    public_response: public_response.clone(),
                    http_method: http_method.clone(),
                    http_url: http_url.clone(),
                    http_headers: http_headers.clone(),
                    http_body: http_body.clone(),
                    http_response: http_response.clone(),
                    logs: logs.clone(),
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
                                    div { class: format_args!("log-entry {}", entry.class()), "{entry.message()}" }
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

#[allow(clippy::too_many_arguments)]
#[component]
fn TabPanel(
    active_tab: Signal<Tab>,
    network_mode: Signal<NetworkMode>,
    keypair: Signal<Option<Keypair>>,
    secret_input: Signal<String>,
    recovery_path: Signal<String>,
    recovery_passphrase: Signal<String>,
    token_caps_input: Signal<String>,
    token_output: Signal<String>,
    session: Signal<Option<PubkySession>>,
    session_details: Signal<String>,
    homeserver_input: Signal<String>,
    signup_code_input: Signal<String>,
    auth_caps_input: Signal<String>,
    auth_relay_input: Signal<String>,
    auth_url_output: Signal<String>,
    auth_qr_data: Signal<Option<String>>,
    auth_status: Signal<String>,
    auth_flow: Signal<Option<PubkyAuthFlow>>,
    auth_request_input: Signal<String>,
    storage_path: Signal<String>,
    storage_body: Signal<String>,
    storage_response: Signal<String>,
    public_resource: Signal<String>,
    public_response: Signal<String>,
    http_method: Signal<String>,
    http_url: Signal<String>,
    http_headers: Signal<String>,
    http_body: Signal<String>,
    http_response: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let selection = *active_tab.read();

    let panel = match selection {
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
    };

    rsx! {
        div { class: "panel", {panel} }
    }
}
