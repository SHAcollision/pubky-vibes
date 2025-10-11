use dioxus::prelude::*;
use pubky::{Keypair, PubkyAuthFlow, PubkySession};

use crate::components::{NetworkToggleOption, TabButton};
use crate::style::APP_STYLE;
use crate::tabs::{
    render_auth_tab, render_http_tab, render_keys_tab, render_sessions_tab, render_storage_tab,
    render_tokens_tab,
};
use crate::utils::logging::LogEntry;
use crate::utils::pubky::{PubkyFacadeState, PubkyFacadeStatus};

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

#[allow(non_snake_case)]
pub fn App() -> Element {
    let active_tab = use_signal(|| Tab::Keys);
    let network_mode = use_signal(|| NetworkMode::Mainnet);
    let logs = use_signal(|| Vec::<LogEntry>::new());
    let show_logs = use_signal(|| false);

    let pubky_facade = use_signal(|| PubkyFacadeState::loading(NetworkMode::Mainnet));
    let mut pubky_bootstrapped = use_signal(|| false);

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

    if !*pubky_bootstrapped.read() {
        pubky_bootstrapped.set(true);
        let initial_network = *network_mode.read();
        queue_pubky_build(pubky_facade, network_mode, initial_network, true);
    }

    let pubky_state_snapshot = { pubky_facade.read().clone() };
    let retry_network = pubky_state_snapshot.network;

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
                            NetworkToggleOption {
                                network_mode: network_mode.clone(),
                                mode,
                                on_select: move |selected| {
                                    queue_pubky_build(pubky_facade, network_mode, selected, false);
                                }
                            }
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
                            pubky_facade,
                            keypair,
                            session,
                            session_details,
                            homeserver_input,
                            signup_code_input,
                            logs,
                        ),
                        Tab::Auth => render_auth_tab(
                            pubky_facade,
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
                            pubky_facade,
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
            if pubky_state_snapshot.is_loading() {
                div { class: "pubky-overlay",
                    div { class: "pubky-spinner" }
                    p {
                        class: "pubky-overlay-text",
                        {format!(
                            "Initializing Pubky facade for {}...",
                            pubky_state_snapshot.network.label()
                        )}
                    }
                }
            } else if let Some(message) = pubky_state_snapshot.error_message() {
                div { class: "pubky-overlay pubky-overlay-error",
                    div { class: "pubky-overlay-panel",
                        h3 { "Failed to initialize Pubky" }
                        p { class: "pubky-overlay-text", "{message}" }
                        div { class: "small-buttons",
                            button {
                                class: "action",
                                onclick: move |_| queue_pubky_build(pubky_facade, network_mode, retry_network, true),
                                "Retry"
                            }
                        }
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

fn queue_pubky_build(
    pubky_state: Signal<PubkyFacadeState>,
    network_signal: Signal<NetworkMode>,
    target: NetworkMode,
    force: bool,
) {
    if !force {
        let current = { pubky_state.read().clone() };
        if current.network == target {
            match current.status {
                PubkyFacadeStatus::Ready(_) | PubkyFacadeStatus::Loading => return,
                PubkyFacadeStatus::Error(_) => {}
            }
        }
    }

    {
        let mut setter = pubky_state;
        setter.set(PubkyFacadeState::loading(target));
    }

    let mut setter = pubky_state;
    spawn({
        let network_signal = network_signal;
        async move {
            match crate::utils::pubky::build_pubky_facade(target).await {
                Ok(pubky) => {
                    if *network_signal.read() == target {
                        setter.set(PubkyFacadeState::ready(target, pubky));
                    }
                }
                Err(err) => {
                    if *network_signal.read() == target {
                        setter.set(PubkyFacadeState::error(target, err.to_string()));
                    }
                }
            }
        }
    });
}
