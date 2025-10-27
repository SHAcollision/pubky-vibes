use dioxus::prelude::*;
use pubky::PubkySession;

use crate::components::{ActivityLog, BattlePanel, IdentityPanel, MatchmakingPanel, SessionPanel};
use crate::models::{CommanderIdentity, MatchState, MatchStoragePaths};
use crate::services::{
    LogEntry, LogLevel, NetworkMode, PubkyFacadeState, build_pubky_facade, push_log,
};
use crate::style::APP_STYLE;

#[component]
#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut network_mode = use_signal(|| NetworkMode::Mainnet);
    let pubky_state = use_signal(|| PubkyFacadeState::loading(NetworkMode::Mainnet));
    let mut pubky_bootstrapped = use_signal(|| false);

    let logs = use_signal(Vec::<LogEntry>::new);
    let identity = use_signal(CommanderIdentity::default);
    let session = use_signal(|| Option::<PubkySession>::None);
    let homeserver_input = use_signal(String::new);
    let signup_code_input = use_signal(String::new);
    let session_details = use_signal(String::new);

    let active_match = use_signal(|| Option::<MatchState>::None);
    let storage_paths = use_signal(|| Option::<MatchStoragePaths>::None);

    if !*pubky_bootstrapped.read() {
        pubky_bootstrapped.set(true);
        let initial = *network_mode.read();
        queue_pubky_build(pubky_state.clone(), initial, logs.clone());
    }

    let current_network = *network_mode.read();
    rsx! {
        style { {APP_STYLE} }
        div { class: "app",
            header {
                div { class: "branding",
                    div { class: "title", "Orbital Sovereigns" }
                    div { class: "subtitle", "Asynchronous tactics on sovereign storage." }
                }
                div { class: "network-toggle",
                    for mode in NetworkMode::ALL {
                        button {
                            class: if mode == current_network { "" } else { "secondary" },
                            onclick: move |_| {
                                network_mode.set(mode);
                                queue_pubky_build(pubky_state.clone(), mode, logs.clone());
                            },
                            {mode.label()}
                        }
                    }
                }
            }
            main {
                div { class: "sidebar",
                    IdentityPanel { identity: identity.clone(), logs: logs.clone() }
                    SessionPanel {
                        pubky_state: pubky_state.clone(),
                        identity: identity.clone(),
                        session: session.clone(),
                        homeserver_input: homeserver_input.clone(),
                        signup_code_input: signup_code_input.clone(),
                        session_details: session_details.clone(),
                        logs: logs.clone(),
                    }
                    ActivityLog { logs: logs.clone() }
                }
                div { class: "content",
                    if let Some(message) = pubky_state.read().error_message() {
                        div { class: "panel", p { class: "helper", "Pubky error: {message}" } }
                    }
                    MatchmakingPanel {
                        identity: identity.clone(),
                        session: session.clone(),
                        pubky_state: pubky_state.clone(),
                        active_match: active_match.clone(),
                        storage_paths: storage_paths.clone(),
                        logs: logs.clone(),
                    }
                    BattlePanel {
                        identity: identity.clone(),
                        session: session.clone(),
                        pubky_state: pubky_state.clone(),
                        active_match: active_match.clone(),
                        storage_paths: storage_paths.clone(),
                        logs: logs.clone(),
                    }
                }
            }
        }
    }
}

fn queue_pubky_build(
    mut state: Signal<PubkyFacadeState>,
    mode: NetworkMode,
    logs: Signal<Vec<LogEntry>>,
) {
    state.set(PubkyFacadeState::loading(mode));
    spawn(async move {
        match build_pubky_facade(mode).await {
            Ok(facade) => {
                state.set(PubkyFacadeState::ready(mode, facade));
                push_log(
                    logs.clone(),
                    LogLevel::Success,
                    format!("Connected to {:?}", mode),
                );
            }
            Err(err) => {
                state.set(PubkyFacadeState::error(mode, err.to_string()));
                push_log(
                    logs.clone(),
                    LogLevel::Error,
                    format!("Failed to bootstrap Pubky: {err}"),
                );
            }
        }
    });
}
