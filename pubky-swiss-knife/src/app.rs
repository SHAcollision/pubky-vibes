use dioxus::prelude::*;
use pubky::{Keypair, PubkyAuthFlow, PubkySession};

use crate::components::{NetworkToggleOption, TabButton};
use crate::style::APP_STYLE;
use crate::tabs::{
    AuthTabState, HttpTabState, KeysTabState, SessionsTabState, StorageTabState, TokensTabState,
    render_auth_tab, render_http_tab, render_keys_tab, render_sessions_tab, render_storage_tab,
    render_tokens_tab,
};
use crate::utils::logging::{ActivityLog, LogEntry};
use crate::utils::pubky::{PubkyFacadeHandle, PubkyFacadeState, PubkyFacadeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    let logs_signal = use_signal(Vec::<LogEntry>::new);
    let activity_log = ActivityLog::new(logs_signal.clone());
    let show_logs = use_signal(|| false);

    let pubky_state = use_signal(|| PubkyFacadeState::loading(NetworkMode::Mainnet));
    let pubky_facade = PubkyFacadeHandle::new(pubky_state.clone());
    let mut pubky_bootstrapped = use_signal(|| false);

    let keypair = use_signal(|| Option::<Keypair>::None);
    let session = use_signal(|| Option::<PubkySession>::None);
    let session_details = use_signal(String::new);

    let keys_state = KeysTabState {
        keypair: keypair.clone(),
        secret_input: use_signal(String::new),
        recovery_path: use_signal(String::new),
        recovery_passphrase: use_signal(String::new),
    };

    let tokens_state = TokensTabState {
        keypair: keypair.clone(),
        capabilities: use_signal(|| String::from("/:rw")),
        output: use_signal(String::new),
    };

    let sessions_state = SessionsTabState {
        keypair: keypair.clone(),
        session: session.clone(),
        details: session_details.clone(),
        homeserver: use_signal(String::new),
        signup_code: use_signal(String::new),
    };

    let auth_state = AuthTabState {
        keypair: keypair.clone(),
        session: session.clone(),
        details: session_details.clone(),
        capabilities: use_signal(|| String::from("/:rw")),
        relay: use_signal(String::new),
        url_output: use_signal(String::new),
        qr_data: use_signal(|| Option::<String>::None),
        status: use_signal(String::new),
        flow: use_signal(|| Option::<PubkyAuthFlow>::None),
        request_body: use_signal(String::new),
    };

    let storage_state = StorageTabState {
        session: session.clone(),
        path: use_signal(|| String::from("/pub/")),
        body: use_signal(String::new),
        response: use_signal(String::new),
        public_resource: use_signal(String::new),
        public_response: use_signal(String::new),
    };

    let http_state = HttpTabState {
        method: use_signal(|| String::from("GET")),
        url: use_signal(|| String::from("https://")),
        headers: use_signal(String::new),
        body: use_signal(String::new),
        response: use_signal(String::new),
    };

    if !*pubky_bootstrapped.read() {
        pubky_bootstrapped.set(true);
        let initial_network = *network_mode.read();
        queue_pubky_build(
            pubky_facade.clone(),
            network_mode.clone(),
            initial_network,
            true,
        );
    }

    let pubky_state_snapshot = pubky_facade.snapshot();
    let retry_network = pubky_state_snapshot.network;

    let show_logs_value = *show_logs.read();
    let show_logs_label = if show_logs_value {
        "Hide activity"
    } else {
        "Show activity"
    };
    let has_logs = !logs_signal.read().is_empty();
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
                                on_select: {
                                    let toggle_handle = pubky_facade.clone();
                                    let toggle_network = network_mode.clone();
                                    move |selected| {
                                        queue_pubky_build(
                                            toggle_handle.clone(),
                                            toggle_network.clone(),
                                            selected,
                                            false,
                                        );
                                    }
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
                        Tab::Keys => render_keys_tab(keys_state.clone(), activity_log.clone()),
                        Tab::Tokens => render_tokens_tab(tokens_state.clone(), activity_log.clone()),
                        Tab::Sessions => render_sessions_tab(
                            pubky_facade.clone(),
                            sessions_state.clone(),
                            activity_log.clone(),
                        ),
                        Tab::Auth => render_auth_tab(
                            pubky_facade.clone(),
                            auth_state.clone(),
                            activity_log.clone(),
                        ),
                        Tab::Storage => render_storage_tab(
                            pubky_facade.clone(),
                            storage_state.clone(),
                            activity_log.clone(),
                        ),
                        Tab::Http => render_http_tab(
                            network_mode.clone(),
                            http_state.clone(),
                            activity_log.clone(),
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
                                title: "Try to initialize Pubky again with the default settings",
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
                    title: "Show or hide the live log of Pubky activity",
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
                                for entry in logs_signal.read().iter() {
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
    pubky_handle: PubkyFacadeHandle,
    network_signal: Signal<NetworkMode>,
    target: NetworkMode,
    force: bool,
) {
    if !force {
        let current = pubky_handle.snapshot();
        if current.network == target {
            match current.status {
                PubkyFacadeStatus::Ready(_) | PubkyFacadeStatus::Loading => return,
                PubkyFacadeStatus::Error(_) => {}
            }
        }
    }

    pubky_handle.set(PubkyFacadeState::loading(target));

    let handle = pubky_handle.clone();
    let network_signal = network_signal.clone();
    spawn(async move {
        match crate::utils::pubky::build_pubky_facade(target).await {
            Ok(pubky) => {
                if *network_signal.read() == target {
                    handle.set(PubkyFacadeState::ready(target, pubky));
                }
            }
            Err(err) => {
                if *network_signal.read() == target {
                    handle.set(PubkyFacadeState::error(target, err.to_string()));
                }
            }
        }
    });
}
