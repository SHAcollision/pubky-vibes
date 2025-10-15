use dioxus::prelude::*;
use pubky::{Keypair, PubkyAuthFlow, PubkySession};

use crate::components::{NetworkToggleOption, TabButton};
use crate::style::APP_STYLE;
use crate::tabs::{
    AuthTabState, HttpTabState, KeysTabState, PkdnsTabState, SessionsTabState, SocialTabState,
    StorageTabState, TokensTabState, render_auth_tab, render_http_tab, render_keys_tab,
    render_pkdns_tab, render_sessions_tab, render_social_tab, render_storage_tab,
    render_tokens_tab,
};
use crate::utils::logging::{ActivityLog, LogEntry};
use crate::utils::mobile::{MobileEnhancementsScript, touch_tooltip};
use crate::utils::pubky::{PubkyFacadeHandle, PubkyFacadeState, PubkyFacadeStatus};

const TESTNET_DEFAULT_SESSION_HOMESERVER: &str =
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

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
    Pkdns,
    Auth,
    Storage,
    Social,
    Http,
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::Keys,
        Tab::Tokens,
        Tab::Sessions,
        Tab::Pkdns,
        Tab::Auth,
        Tab::Storage,
        Tab::Social,
        Tab::Http,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Keys => "Keys",
            Tab::Tokens => "Auth Tokens",
            Tab::Sessions => "Sessions",
            Tab::Pkdns => "PKDNS",
            Tab::Auth => "Auth Flows",
            Tab::Storage => "Storage",
            Tab::Social => "Social",
            Tab::Http => "Raw Requests",
        }
    }

    pub fn icon(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Tab::Keys => (
                "0 0 24 24",
                &[
                    r#"M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"#,
                ],
            ),
            Tab::Tokens => (
                "0 0 24 24",
                &[
                    r#"M6 6.878V6a2.25 2.25 0 0 1 2.25-2.25h7.5A2.25 2.25 0 0 1 18 6v.878m-12 0c.235-.083.487-.128.75-.128h10.5c.263 0 .515.045.75.128m-12 0A2.25 2.25 0 0 0 4.5 9v.878m13.5-3A2.25 2.25 0 0 1 19.5 9v.878m0 0a2.246 2.246 0 0 0-.75-.128H5.25c-.263 0-.515.045-.75.128m15 0A2.25 2.25 0 0 1 21 12v6a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 18v-6c0-.98.626-1.813 1.5-2.122"#,
                ],
            ),
            Tab::Sessions => (
                "0 0 24 24",
                &[
                    r#"M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z"#,
                ],
            ),
            Tab::Pkdns => (
                "0 0 24 24",
                &[
                    r#"M6 7.5h12m-12 0A2.25 2.25 0 0 0 3.75 9.75v1.5A2.25 2.25 0 0 0 6 13.5h12a2.25 2.25 0 0 0 2.25-2.25v-1.5A2.25 2.25 0 0 0 18 7.5H6Zm0 6H18m-12 0a2.25 2.25 0 0 0-2.25 2.25v1.5A2.25 2.25 0 0 0 6 19.5h12a2.25 2.25 0 0 0 2.25-2.25v-1.5A2.25 2.25 0 0 0 18 13.5H6Z"#,
                ],
            ),
            Tab::Auth => (
                "0 0 24 24",
                &[
                    r#"M7.864 4.243A7.5 7.5 0 0 1 19.5 10.5c0 2.92-.556 5.709-1.568 8.268M5.742 6.364A7.465 7.465 0 0 0 4.5 10.5a7.464 7.464 0 0 1-1.15 3.993m1.989 3.559A11.209 11.209 0 0 0 8.25 10.5a3.75 3.75 0 1 1 7.5 0c0 .527-.021 1.049-.064 1.565M12 10.5a14.94 14.94 0 0 1-3.6 9.75m6.633-4.596a18.666 18.666 0 0 1-2.485 5.33"#,
                ],
            ),
            Tab::Storage => (
                "0 0 24 24",
                &[
                    r#"M5.25 14.25h13.5m-13.5 0a3 3 0 0 1-3-3m3 3a3 3 0 1 0 0 6h13.5a3 3 0 1 0 0-6m-16.5-3a3 3 0 0 1 3-3h13.5a3 3 0 0 1 3 3m-19.5 0a4.5 4.5 0 0 1 .9-2.7L5.737 5.1a3.375 3.375 0 0 1 2.7-1.35h7.126c1.062 0 2.062.5 2.7 1.35l2.587 3.45a4.5 4.5 0 0 1 .9 2.7m0 0a3 3 0 0 1-3 3m0 3h.008v.008h-.008v-.008Zm0-6h.008v.008h-.008v-.008Zm-3 6h.008v.008h-.008v-.008Zm0-6h.008v.008h-.008v-.008Z"#,
                ],
            ),
            Tab::Social => (
                "0 0 24 24",
                &[
                    r#"M5.25 8.25H20.25M3.75 15.75H18.75M16.95 2.25L13.05 21.75M10.9503 2.25L7.05029 21.75"#,
                ],
            ),
            Tab::Http => (
                "0 0 24 24",
                &[
                    r#"M12 21a9.004 9.004 0 0 0 8.716-6.747M12 21a9.004 9.004 0 0 1-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 0 1 7.843 4.582M12 3a8.997 8.997 0 0 0-7.843 4.582m15.686 0A11.953 11.953 0 0 1 12 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0 1 21 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0 1 12 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 0 1 3 12c0-1.605.42-3.113 1.157-4.418"#,
                ],
            ),
        }
    }

    pub fn requires_session(self) -> bool {
        matches!(self, Tab::Social)
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

    let pkdns_state = PkdnsTabState {
        keypair: keypair.clone(),
        lookup_input: use_signal(String::new),
        lookup_result: use_signal(String::new),
        host_override: use_signal(String::new),
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

    let social_state = SocialTabState {
        session: session.clone(),
        profile_name: use_signal(String::new),
        profile_bio: use_signal(String::new),
        profile_image: use_signal(String::new),
        profile_status: use_signal(String::new),
        profile_links: use_signal(String::new),
        profile_error: use_signal(String::new),
        profile_response: use_signal(String::new),
        post_content: use_signal(String::new),
        post_kind: use_signal(|| String::from("short")),
        post_parent: use_signal(String::new),
        post_embed_kind: use_signal(String::new),
        post_embed_uri: use_signal(String::new),
        post_attachments: use_signal(String::new),
        post_response: use_signal(String::new),
        tag_uri: use_signal(String::new),
        tag_label: use_signal(String::new),
        tag_response: use_signal(String::new),
    };

    let http_state = HttpTabState {
        method: use_signal(|| String::from("GET")),
        url: use_signal(|| String::from("https://")),
        headers: use_signal(String::new),
        body: use_signal(String::new),
        response: use_signal(String::new),
    };

    let has_session = session.read().is_some();
    if matches!(*active_tab.read(), Tab::Social) && !has_session {
        let mut reset_tab = active_tab.clone();
        reset_tab.set(Tab::Keys);
    }
    let mut session_homeserver_prefill = sessions_state.homeserver.clone();
    let network_signal_for_prefill = network_mode.clone();
    use_effect(move || {
        if *network_signal_for_prefill.read() == NetworkMode::Testnet {
            session_homeserver_prefill.set(String::from(TESTNET_DEFAULT_SESSION_HOMESERVER));
        }
    });

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
    let retry_handle = pubky_facade.clone();
    let retry_signal = network_mode.clone();

    rsx! {
        style { {APP_STYLE} }
        MobileEnhancementsScript {}
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
                    for tab in Tab::ALL
                        .iter()
                        .copied()
                        .filter(|tab| !tab.requires_session() || has_session)
                    {
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
                        Tab::Pkdns => render_pkdns_tab(
                            pubky_facade.clone(),
                            pkdns_state.clone(),
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
                        Tab::Social => render_social_tab(
                            pubky_facade.clone(),
                            social_state.clone(),
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
                                "data-touch-tooltip": touch_tooltip(
                                    "Try to initialize Pubky again with the default settings",
                                ),
                                onclick: move |_| {
                                    queue_pubky_build(
                                        retry_handle.clone(),
                                        retry_signal.clone(),
                                        retry_network,
                                        true,
                                    );
                                },
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
                    "data-touch-tooltip": touch_tooltip(
                        "Show or hide the live log of Pubky activity",
                    ),
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
