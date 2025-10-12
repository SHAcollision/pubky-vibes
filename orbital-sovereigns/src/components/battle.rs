use anyhow::Result;
use dioxus::prelude::WritableExt;
use dioxus::prelude::*;
use pubky::PubkySession;

use crate::models::{BattleAction, MatchState, MatchStoragePaths, TurnSnapshot, Vector3};
use crate::services::storage::store_turn;
use crate::services::sync::discover_new_turn;
use crate::services::{LogEntry, LogLevel, PubkyFacadeState, push_log};

fn parse_vector3(raw: &str) -> Result<Vector3> {
    let parts: Vec<_> = raw.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 {
        anyhow::bail!("Vector must have three comma-separated components");
    }
    let x: f32 = parts[0].parse()?;
    let y: f32 = parts[1].parse()?;
    let z: f32 = parts[2].parse()?;
    Ok(Vector3 { x, y, z })
}

fn current_actor(
    state: &MatchState,
    identity: &crate::models::CommanderIdentity,
) -> Option<String> {
    let pub_key = identity.keypair()?.public_key().to_string();
    if state.meta.host.public_key == pub_key {
        Some(state.meta.host.label.clone())
    } else if let Some(guest) = &state.meta.guest {
        if guest.public_key == pub_key {
            Some(guest.label.clone())
        } else {
            None
        }
    } else {
        None
    }
}

async fn persist_turn(
    session: Option<PubkySession>,
    paths: MatchStoragePaths,
    snapshot: TurnSnapshot,
    logs: Signal<Vec<LogEntry>>,
) {
    let Some(session) = session else {
        push_log(
            logs,
            LogLevel::Warning,
            "Session expired before storing turn",
        );
        return;
    };
    match store_turn(&session, &paths, &snapshot).await {
        Ok(pointer) => {
            push_log(
                logs,
                LogLevel::Success,
                format!("Turn {} stored (hash {})", snapshot.turn, pointer.hash),
            );
        }
        Err(err) => {
            push_log(
                logs,
                LogLevel::Error,
                format!("Failed to persist turn: {err}"),
            );
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments, non_snake_case)]
pub fn BattlePanel(
    identity: Signal<crate::models::CommanderIdentity>,
    session: Signal<Option<PubkySession>>,
    pubky_state: Signal<PubkyFacadeState>,
    active_match: Signal<Option<MatchState>>,
    storage_paths: Signal<Option<MatchStoragePaths>>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let thrust_input = use_signal(|| String::from("0,0,0"));
    let target_input = use_signal(String::new);
    let power_input = use_signal(|| String::from("24"));
    let opponent_key = use_signal(String::new);

    let thrust_value = { thrust_input.read().clone() };
    let target_value = { target_input.read().clone() };
    let power_value = { power_input.read().clone() };
    let opponent_value = { opponent_key.read().clone() };

    let match_snapshot = active_match.read().clone();
    let scene_digest = match match_snapshot.as_ref() {
        Some(state) => state.arena.digest(),
        None => String::from("No match loaded"),
    };
    let turns = match_snapshot
        .as_ref()
        .map(|state| state.turns.clone())
        .unwrap_or_default();
    let actor_name = match match_snapshot.as_ref() {
        Some(state) => current_actor(state, &identity.read()),
        None => None,
    };

    let identity_for_actions = identity.clone();
    let session_for_actions = session.clone();
    let mut match_for_actions = active_match.clone();
    let paths_for_actions = storage_paths.clone();
    let logs_for_actions = logs.clone();
    let thrust_for_actions = thrust_input.clone();
    let target_for_actions = target_input.clone();
    let power_for_actions = power_input.clone();

    let poll_pubky = pubky_state.clone();
    let poll_paths = storage_paths.clone();
    let poll_match = active_match.clone();
    let poll_logs = logs.clone();
    let poll_key = opponent_key.clone();

    let mut thrust_binding = thrust_input.clone();
    let mut target_binding = target_input.clone();
    let mut power_binding = power_input.clone();
    let mut opponent_binding = opponent_key.clone();

    rsx! {
        div { class: "panel",
            h2 { "Battle console" }
            p { "Pilot your fleet, publish turns, and ingest opponent updates." }
            div { class: "scene-preview", "Scene hash: {scene_digest}" }
            if let Some(label) = actor_name {
                p { class: "helper", "Acting as {label}" }
            } else {
                p { class: "helper", "Load a match and ensure your key matches a participant." }
            }
            section {
                h3 { "Local orders" }
                div { class: "field-grid",
                    label {
                        "Thrust vector"
                        input {
                            value: thrust_value,
                            oninput: move |evt| thrust_binding.set(evt.value()),
                            placeholder: "dx, dy, dz",
                        }
                    }
                    label {
                        "Fire target"
                        input {
                            value: target_value,
                            oninput: move |evt| target_binding.set(evt.value()),
                            placeholder: "Opponent callsign",
                        }
                    }
                    label {
                        "Fire power"
                        input {
                            value: power_value,
                            oninput: move |evt| power_binding.set(evt.value()),
                        }
                    }
                }
                div { class: "field-grid",
                    button {
                        onclick: move |_| {
                            let Some(paths) = paths_for_actions.read().clone() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Match storage paths unavailable");
                                return;
                            };
                            let mut guard = match_for_actions.write();
                            let Some(state) = guard.as_mut() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Load a match first");
                                return;
                            };
                            let Some(actor) = current_actor(state, &identity_for_actions.read()) else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Your key is not part of this match");
                                return;
                            };
                            let thrust_raw = thrust_for_actions.read().clone();
                            let vector = match parse_vector3(&thrust_raw) {
                                Ok(v) => v,
                                Err(err) => {
                                    push_log(logs_for_actions.clone(), LogLevel::Error, format!("Invalid vector: {err}"));
                                    return;
                                }
                            };
                            match state.apply_action(&actor, BattleAction::Maneuver { thrust: vector }) {
                                Ok(snapshot) => {
                                    push_log(
                                        logs_for_actions.clone(),
                                        LogLevel::Info,
                                        format!("Simulated thrust for turn {}", snapshot.turn),
                                    );
                                    let session_clone = session_for_actions.read().as_ref().cloned();
                                    let logs_clone = logs_for_actions.clone();
                                    let paths_clone = paths.clone();
                                    spawn(persist_turn(session_clone, paths_clone, snapshot, logs_clone));
                                }
                                Err(err) => {
                                    push_log(logs_for_actions.clone(), LogLevel::Error, format!("Action failed: {err}"));
                                }
                            }
                        },
                        "Apply thrust"
                    }
                    button {
                        onclick: move |_| {
                            let Some(paths) = paths_for_actions.read().clone() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Match storage paths unavailable");
                                return;
                            };
                            let mut guard = match_for_actions.write();
                            let Some(state) = guard.as_mut() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Load a match first");
                                return;
                            };
                            let Some(actor) = current_actor(state, &identity_for_actions.read()) else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Your key is not part of this match");
                                return;
                            };
                            let target = target_for_actions.read().clone();
                            if target.trim().is_empty() {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Provide a target callsign");
                                return;
                            }
                            let power = power_for_actions
                                .read()
                                .parse::<u16>()
                                .unwrap_or(20);
                            match state.apply_action(&actor, BattleAction::Fire { target: target.clone(), power }) {
                                Ok(snapshot) => {
                                    push_log(
                                        logs_for_actions.clone(),
                                        LogLevel::Info,
                                        format!("Simulated fire for turn {}", snapshot.turn),
                                    );
                                    let session_clone = session_for_actions.read().as_ref().cloned();
                                    let logs_clone = logs_for_actions.clone();
                                    let paths_clone = paths.clone();
                                    spawn(persist_turn(session_clone, paths_clone, snapshot, logs_clone));
                                }
                                Err(err) => {
                                    push_log(logs_for_actions.clone(), LogLevel::Error, format!("Action failed: {err}"));
                                }
                            }
                        },
                        "Fire"
                    }
                    button { class: "secondary",
                        onclick: move |_| {
                            let Some(paths) = paths_for_actions.read().clone() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Match storage paths unavailable");
                                return;
                            };
                            let mut guard = match_for_actions.write();
                            let Some(state) = guard.as_mut() else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Load a match first");
                                return;
                            };
                            let Some(actor) = current_actor(state, &identity_for_actions.read()) else {
                                push_log(logs_for_actions.clone(), LogLevel::Warning, "Your key is not part of this match");
                                return;
                            };
                            match state.apply_action(&actor, BattleAction::Brace) {
                                Ok(snapshot) => {
                                    push_log(
                                        logs_for_actions.clone(),
                                        LogLevel::Info,
                                        format!("Simulated brace for turn {}", snapshot.turn),
                                    );
                                    let session_clone = session_for_actions.read().as_ref().cloned();
                                    let logs_clone = logs_for_actions.clone();
                                    let paths_clone = paths.clone();
                                    spawn(persist_turn(session_clone, paths_clone, snapshot, logs_clone));
                                }
                                Err(err) => {
                                    push_log(logs_for_actions.clone(), LogLevel::Error, format!("Action failed: {err}"));
                                }
                            }
                        },
                        "Brace"
                    }
                }
            }
            section {
                h3 { "Opponent sync" }
                label {
                    "Opponent public key"
                    input {
                        value: opponent_value,
                        oninput: move |evt| opponent_binding.set(evt.value()),
                        placeholder: "pubky...",
                    }
                }
                button { class: "secondary",
                    onclick: move |_| {
                        let Some(pubky) = poll_pubky.read().facade() else {
                            push_log(poll_logs.clone(), LogLevel::Info, "Pubky is still starting up");
                            return;
                        };
                        let Some(paths) = poll_paths.read().clone() else {
                            push_log(poll_logs.clone(), LogLevel::Warning, "Load a match first");
                            return;
                        };
                        let opponent = poll_key.read().clone();
                        if opponent.trim().is_empty() {
                            push_log(poll_logs.clone(), LogLevel::Warning, "Provide the opponent key");
                            return;
                        }
                        let mut match_signal = poll_match.clone();
                        let logs_signal = poll_logs.clone();
                        spawn(async move {
                            let latest_turn = match_signal
                                .read()
                                .as_ref()
                                .and_then(|state| state.latest_turn().map(|turn| turn.turn));
                            match discover_new_turn(pubky.clone(), opponent.trim(), &paths, latest_turn).await {
                                Ok(Some(snapshot)) => {
                                    match_signal.write().as_mut().map(|state| state.turns.push(snapshot.clone()));
                                    push_log(logs_signal, LogLevel::Success, format!("Fetched opponent turn {}", snapshot.turn));
                                }
                                Ok(None) => push_log(logs_signal, LogLevel::Info, "No new turns available"),
                                Err(err) => push_log(logs_signal, LogLevel::Error, format!("Failed to sync: {err}")),
                            }
                        });
                    },
                    "Poll opponent"
                }
            }
            section {
                h3 { "Turn history" }
                div { class: "turn-list",
                    for turn in turns {
                        div { class: "turn-card",
                            h4 { "Turn {turn.turn} – {turn.actor}" }
                            pre { "{serde_json::to_string_pretty(&turn).unwrap_or_default()}" }
                        }
                    }
                }
            }
        }
    }
}
