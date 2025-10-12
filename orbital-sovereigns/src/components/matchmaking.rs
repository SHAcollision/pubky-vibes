use anyhow::{Context, Result};
use dioxus::prelude::WritableExt;
use dioxus::prelude::*;
use pubky::PubkySession;
use rand::random;
use uuid::Uuid;

use crate::models::{
    MatchMeta, MatchRules, MatchState, MatchStoragePaths, ModuleKind, ShipBlueprint, ShipModule,
};
use crate::services::storage::{read_json, store_match_meta};
use crate::services::{LogEntry, LogLevel, PubkyFacadeState, push_log};

fn default_blueprint() -> ShipBlueprint {
    ShipBlueprint {
        name: "Vanguard".into(),
        modules: vec![
            ShipModule::new(ModuleKind::Hull, [0, 0, 0], [0, 1, 0], 12, 18),
            ShipModule::new(ModuleKind::Reactor, [0, -1, 0], [0, 1, 0], 18, 14),
            ShipModule::new(ModuleKind::Thruster, [0, -2, 0], [0, 1, 0], 16, 12),
            ShipModule::new(ModuleKind::Cannon, [0, 1, 0], [0, 1, 0], 20, 10),
            ShipModule::new(ModuleKind::Shield, [1, 0, 0], [0, 1, 0], 14, 9),
        ],
        crew: 48,
        mass_override: None,
    }
}

fn default_blueprint_json() -> String {
    serde_json::to_string_pretty(&default_blueprint()).unwrap()
}

#[component]
#[allow(clippy::too_many_arguments, non_snake_case)]
pub fn MatchmakingPanel(
    identity: Signal<crate::models::CommanderIdentity>,
    session: Signal<Option<PubkySession>>,
    pubky_state: Signal<PubkyFacadeState>,
    active_match: Signal<Option<MatchState>>,
    storage_paths: Signal<Option<MatchStoragePaths>>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let host_blueprint = use_signal(default_blueprint_json);
    let host_rounds = use_signal(|| String::from("60"));
    let host_radius = use_signal(|| String::from("520"));
    let simultaneous_fire = use_signal(|| true);

    let import_match_id = use_signal(|| String::new());
    let import_owner = use_signal(|| String::new());
    let join_blueprint = use_signal(default_blueprint_json);

    let host_blueprint_value = { host_blueprint.read().clone() };
    let rounds_value = { host_rounds.read().clone() };
    let radius_value = { host_radius.read().clone() };
    let simultaneous_value = *simultaneous_fire.read();

    let import_match_value = { import_match_id.read().clone() };
    let import_owner_value = { import_owner.read().clone() };
    let join_blueprint_value = { join_blueprint.read().clone() };

    let mut host_blueprint_binding = host_blueprint.clone();
    let mut host_rounds_binding = host_rounds.clone();
    let mut host_radius_binding = host_radius.clone();
    let mut simultaneous_binding = simultaneous_fire.clone();

    let mut import_match_binding = import_match_id.clone();
    let mut import_owner_binding = import_owner.clone();
    let mut join_blueprint_binding = join_blueprint.clone();

    let create_identity = identity.clone();
    let create_session = session.clone();
    let create_logs = logs.clone();
    let create_host_blueprint = host_blueprint.clone();
    let create_rounds = host_rounds.clone();
    let create_radius = host_radius.clone();
    let create_simultaneous = simultaneous_fire.clone();
    let create_active_match = active_match.clone();
    let create_storage_paths = storage_paths.clone();

    let import_pubky = pubky_state.clone();
    let import_logs = logs.clone();
    let import_match_signal = import_match_id.clone();
    let import_owner_signal = import_owner.clone();
    let import_active_match = active_match.clone();
    let import_paths = storage_paths.clone();

    let join_identity = identity.clone();
    let join_logs = logs.clone();
    let mut join_match = active_match.clone();
    let join_blueprint_signal = join_blueprint.clone();

    let current_match_id = active_match
        .read()
        .as_ref()
        .map(|state| state.meta.match_id.to_string())
        .unwrap_or_else(|| "—".into());

    rsx! {
        div { class: "panel",
            h2 { "Match orchestration" }
            p { "Host new matches or import an existing lobby from a rival commander." }
            section {
                h3 { "Host a new match" }
                div { class: "field-grid",
                    label {
                        "Max rounds"
                        input {
                            value: rounds_value,
                            oninput: move |evt| host_rounds_binding.set(evt.value()),
                        }
                    }
                    label {
                        "Arena radius"
                        input {
                            value: radius_value,
                            oninput: move |evt| host_radius_binding.set(evt.value()),
                        }
                    }
                    label {
                        "Simultaneous fire"
                        input {
                            r#type: "checkbox",
                            checked: simultaneous_value,
                            onchange: move |_| {
                                let current = *simultaneous_binding.read();
                                simultaneous_binding.set(!current);
                            },
                        }
                    }
                }
                label {
                    "Ship blueprint"
                    textarea {
                        value: host_blueprint_value,
                        oninput: move |evt| host_blueprint_binding.set(evt.value()),
                        class: "tall",
                    }
                }
                button {
                    onclick: move |_| {
                        let Some(session) = create_session.read().as_ref().cloned() else {
                            push_log(create_logs.clone(), LogLevel::Warning, "Sign into a homeserver first");
                            return;
                        };
                        let Some(commander_profile) = create_identity.read().public_profile(None) else {
                            push_log(create_logs.clone(), LogLevel::Warning, "Generate or import a key first");
                            return;
                        };
                        let blueprint_text = create_host_blueprint.read().clone();
                        let rounds_text = create_rounds.read().clone();
                        let radius_text = create_radius.read().clone();
                        let simultaneous = *create_simultaneous.read();
                        let mut match_signal = create_active_match.clone();
                        let mut paths_signal = create_storage_paths.clone();
                        let logs_signal = create_logs.clone();
                        spawn(async move {
                            let result: Result<()> = async move {
                                let blueprint: ShipBlueprint = serde_json::from_str(&blueprint_text)
                                    .context("Blueprint must be valid JSON")?;
                                let rules = MatchRules {
                                    max_rounds: rounds_text.parse().unwrap_or(60),
                                    arena_radius: radius_text.parse().unwrap_or(520.0),
                                    simultaneous_fire: simultaneous,
                                };
                                let meta = MatchMeta::new(commander_profile.clone(), blueprint, rules, random());
                                let paths = MatchStoragePaths::new(meta.match_id);
                                store_match_meta(&session, &paths, &meta).await?;
                                let arena = meta.arena_state()?;
                                match_signal.set(Some(MatchState::new(meta.clone(), arena)));
                                paths_signal.set(Some(paths));
                                Ok(())
                            }
                            .await;
                            match result {
                                Ok(()) => push_log(logs_signal.clone(), LogLevel::Success, "Match metadata stored"),
                                Err(err) => push_log(logs_signal.clone(), LogLevel::Error, format!("Match creation failed: {err}")),
                            }
                        });
                    },
                    "Create match"
                }
            }
            section {
                h3 { "Import remote match" }
                div { class: "field-grid",
                    label {
                        "Opponent public key"
                        input {
                            value: import_owner_value,
                            oninput: move |evt| import_owner_binding.set(evt.value()),
                            placeholder: "Base32 pubky key",
                        }
                    }
                    label {
                        "Match ID"
                        input {
                            value: import_match_value,
                            oninput: move |evt| import_match_binding.set(evt.value()),
                            placeholder: "UUID",
                        }
                    }
                }
                button {
                    onclick: move |_| {
                        let Some(pubky) = import_pubky.read().facade() else {
                            push_log(import_logs.clone(), LogLevel::Info, "Pubky is still starting up");
                            return;
                        };
                        let match_id_text = import_match_signal.read().clone();
                        let owner_text = import_owner_signal.read().clone();
                        if match_id_text.trim().is_empty() || owner_text.trim().is_empty() {
                            push_log(import_logs.clone(), LogLevel::Warning, "Provide opponent key and match ID");
                            return;
                        }
                        let mut match_signal = import_active_match.clone();
                        let mut paths_signal = import_paths.clone();
                        let logs_signal = import_logs.clone();
                        spawn(async move {
                            let result: Result<()> = async move {
                                let id = Uuid::parse_str(&match_id_text)?;
                                let paths = MatchStoragePaths::new(id);
                                let meta_url = format!("pubky{owner_text}{}/meta.json", paths.namespace);
                                let response = pubky.public_storage().get(meta_url).await?;
                                let meta: MatchMeta = read_json(response).await?;
                                let arena = meta.arena_state()?;
                                match_signal.set(Some(MatchState::new(meta.clone(), arena)));
                                paths_signal.set(Some(paths));
                                Ok(())
                            }
                            .await;
                            match result {
                                Ok(()) => push_log(logs_signal.clone(), LogLevel::Success, "Imported match metadata"),
                                Err(err) => push_log(logs_signal.clone(), LogLevel::Error, format!("Import failed: {err}")),
                            }
                        });
                    },
                    "Load meta"
                }
                label {
                    "Join with blueprint"
                    textarea {
                        value: join_blueprint_value,
                        oninput: move |evt| join_blueprint_binding.set(evt.value()),
                        class: "tall",
                    }
                }
                button { class: "secondary",
                    onclick: move |_| {
                        let mut binding = join_match.write();
                        let Some(state) = binding.as_mut() else {
                            push_log(join_logs.clone(), LogLevel::Warning, "Load a match first");
                            return;
                        };
                        let Some(profile) = join_identity.read().public_profile(None) else {
                            push_log(join_logs.clone(), LogLevel::Warning, "Generate a key first");
                            return;
                        };
                        let blueprint_text = join_blueprint_signal.read().clone();
                        match serde_json::from_str::<ShipBlueprint>(&blueprint_text) {
                            Ok(blueprint) => {
                                let updated_meta = state.meta.clone().with_guest(profile, blueprint);
                                state.meta = updated_meta.clone();
                                if let Err(err) = state.rebuild_arena() {
                                    push_log(join_logs.clone(), LogLevel::Error, format!("Failed to rebuild arena: {err}"));
                                } else {
                                    push_log(join_logs.clone(), LogLevel::Success, "Joined match as guest" );
                                }
                            }
                            Err(err) => {
                                push_log(join_logs.clone(), LogLevel::Error, format!("Blueprint invalid: {err}"));
                            }
                        }
                    },
                    "Join match"
                }
            }
            p { class: "helper", "Current match ID: {current_match_id}" }
        }
    }
}
