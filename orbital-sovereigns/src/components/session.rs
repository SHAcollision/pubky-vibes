use anyhow::anyhow;
use dioxus::prelude::WritableExt;
use dioxus::prelude::*;
use pubky::{PubkySession, PublicKey};

use crate::models::CommanderIdentity;
use crate::services::{LogEntry, LogLevel, PubkyFacadeState, push_log};

#[component]
#[allow(clippy::too_many_arguments, non_snake_case)]
pub fn SessionPanel(
    pubky_state: Signal<PubkyFacadeState>,
    identity: Signal<CommanderIdentity>,
    session: Signal<Option<PubkySession>>,
    homeserver_input: Signal<String>,
    signup_code_input: Signal<String>,
    session_details: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let homeserver_value = { homeserver_input.read().clone() };
    let signup_code_value = { signup_code_input.read().clone() };
    let details_value = { session_details.read().clone() };
    let current_public = identity
        .read()
        .keypair()
        .map(|kp| kp.public_key().to_string())
        .unwrap_or_else(|| "—".into());

    let mut homeserver_binding = homeserver_input.clone();
    let mut signup_binding = signup_code_input.clone();

    let signup_identity = identity.clone();
    let signup_pubky = pubky_state.clone();
    let signup_homeserver = homeserver_input.clone();
    let signup_code = signup_code_input.clone();
    let signup_session = session.clone();
    let signup_details = session_details.clone();
    let signup_logs = logs.clone();

    let signin_identity = identity.clone();
    let signin_pubky = pubky_state.clone();
    let signin_session = session.clone();
    let signin_details = session_details.clone();
    let signin_logs = logs.clone();

    let revalidate_session = session.clone();
    let revalidate_details = session_details.clone();
    let revalidate_logs = logs.clone();

    let signout_session = session.clone();
    let signout_details = session_details.clone();
    let signout_logs = logs.clone();

    rsx! {
        div { class: "panel",
            h2 { "Homeserver session" }
            p { "Authenticate with a Pubky homeserver using the active commander signer." }
            p { class: "helper", "Active signer: {current_public}" }
            div { class: "field-grid",
                label {
                    "Homeserver public key"
                    input {
                        value: homeserver_value,
                        oninput: move |evt| homeserver_binding.set(evt.value()),
                        placeholder: "Base32 homeserver public key",
                    }
                }
                label {
                    "Signup code (optional)"
                    input {
                        value: signup_code_value,
                        oninput: move |evt| signup_binding.set(evt.value()),
                        placeholder: "Invitation code if required",
                    }
                }
            }
            div { class: "field-grid",
                button {
                    onclick: move |_| {
                        if let Some(kp) = signup_identity.read().keypair().cloned() {
                            let Some(pubky) = signup_pubky.read().facade() else {
                                push_log(signup_logs.clone(), LogLevel::Info, "Pubky is still starting up");
                                return;
                            };
                            let homeserver = signup_homeserver.read().clone();
                            if homeserver.trim().is_empty() {
                                push_log(signup_logs.clone(), LogLevel::Error, "Homeserver public key is required");
                                return;
                            }
                            let signup_code_value = signup_code.read().clone();
                            let mut session_signal = signup_session.clone();
                            let mut details_signal = signup_details.clone();
                            let logs_signal = signup_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let homeserver_pk = PublicKey::try_from(homeserver.as_str())
                                        .map_err(|e| anyhow!("Invalid homeserver key: {e}"))?;
                                    let signer = pubky.signer(kp.clone());
                                    let session = signer
                                        .signup(&homeserver_pk, if signup_code_value.trim().is_empty() {
                                            None
                                        } else {
                                            Some(signup_code_value.trim())
                                        })
                                        .await?;
                                    let info_line = format!(
                                        "Signed in as {}",
                                        session.info().public_key()
                                    );
                                    session_signal.set(Some(session));
                                    details_signal.set(info_line);
                                    Ok::<_, anyhow::Error>(format!("Signed up with homeserver {homeserver}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_signal.clone(), LogLevel::Success, msg),
                                    Err(err) => push_log(logs_signal.clone(), LogLevel::Error, format!("Signup failed: {err}")),
                                }
                            });
                        } else {
                            push_log(signup_logs, LogLevel::Warning, "Generate or import a key first");
                        }
                    },
                    "Sign up"
                }
                button { class: "secondary",
                    onclick: move |_| {
                        if let Some(kp) = signin_identity.read().keypair().cloned() {
                            let Some(pubky) = signin_pubky.read().facade() else {
                                push_log(signin_logs.clone(), LogLevel::Info, "Pubky is still starting up");
                                return;
                            };
                            let mut session_signal = signin_session.clone();
                            let mut details_signal = signin_details.clone();
                            let logs_signal = signin_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let session = pubky.signer(kp.clone()).signin().await?;
                                    let info_line = format!(
                                        "Signed in as {}",
                                        session.info().public_key()
                                    );
                                    session_signal.set(Some(session));
                                    details_signal.set(info_line);
                                    Ok::<_, anyhow::Error>("Signed in".to_string())
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_signal.clone(), LogLevel::Success, msg),
                                    Err(err) => push_log(logs_signal.clone(), LogLevel::Error, format!("Signin failed: {err}")),
                                }
                            });
                        } else {
                            push_log(signin_logs, LogLevel::Warning, "Generate or import a key first");
                        }
                    },
                    "Sign in"
                }
                button { class: "secondary",
                    onclick: move |_| {
                        if let Some(session) = revalidate_session.read().as_ref().cloned() {
                            let mut session_signal = revalidate_session.clone();
                            let mut details_signal = revalidate_details.clone();
                            let logs_signal = revalidate_logs.clone();
                            spawn(async move {
                                match session.revalidate().await {
                                    Ok(Some(info)) => {
                                        details_signal.set(format!("{info:#?}"));
                                        push_log(logs_signal.clone(), LogLevel::Success, "Session still valid");
                                        session_signal.set(Some(session));
                                    }
                                    Ok(None) => {
                                        session_signal.set(None);
                                        details_signal.set(String::new());
                                        push_log(logs_signal.clone(), LogLevel::Error, "Session expired or missing");
                                    }
                                    Err(err) => {
                                        session_signal.set(Some(session));
                                        push_log(
                                            logs_signal.clone(),
                                            LogLevel::Error,
                                            format!("Revalidate failed: {err}"),
                                        );
                                    }
                                }
                            });
                        } else {
                            push_log(revalidate_logs, LogLevel::Warning, "No active session to revalidate");
                        }
                    },
                    "Revalidate"
                }
                button { class: "secondary",
                    onclick: move |_| {
                        let mut session_signal = signout_session.clone();
                        let maybe_session = {
                            let mut guard = session_signal.write();
                            guard.take()
                        };
                        if let Some(session) = maybe_session {
                            let mut details_signal = signout_details.clone();
                            let logs_signal = signout_logs.clone();
                            spawn(async move {
                                match session.signout().await {
                                    Ok(()) => {
                                        details_signal.set(String::new());
                                        push_log(logs_signal.clone(), LogLevel::Success, "Signed out" );
                                    }
                                    Err((err, session_back)) => {
                                        session_signal.set(Some(session_back));
                                        push_log(logs_signal.clone(), LogLevel::Error, format!("Signout failed: {err}"));
                                    }
                                }
                            });
                        } else {
                            push_log(signout_logs, LogLevel::Warning, "No active session to sign out");
                        }
                    },
                    "Sign out"
                }
            }
            if !details_value.is_empty() {
                p { class: "helper", {details_value} }
            }
        }
    }
}
