use anyhow::anyhow;
use dioxus::prelude::*;
use pubky::{Keypair, PubkySession, PublicKey};

use crate::tabs::format_session_info;
use crate::utils::logging::{LogEntry, LogLevel, push_log};
use crate::utils::pubky::PubkyFacadeState;

#[allow(clippy::clone_on_copy)]
pub fn render_sessions_tab(
    pubky_state: Signal<PubkyFacadeState>,
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

    let signup_keypair = keypair.clone();
    let signup_homeserver = homeserver_input.clone();
    let signup_code_signal = signup_code_input.clone();
    let signup_session_signal = session.clone();
    let signup_details_signal = session_details.clone();
    let signup_logs = logs.clone();
    let signup_pubky_state = pubky_state.clone();

    let signin_keypair = keypair.clone();
    let signin_session_signal = session.clone();
    let signin_details_signal = session_details.clone();
    let signin_logs = logs.clone();
    let signin_pubky_state = pubky_state.clone();

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
                            let maybe_pubky = { signup_pubky_state.read().facade() };
                            let Some(pubky) = maybe_pubky else {
                                push_log(
                                    signup_logs.clone(),
                                    LogLevel::Info,
                                    "Pubky facade is still starting up. Try again shortly.",
                                );
                                return;
                            };
                            let mut session_signal = signup_session_signal.clone();
                            let mut details_signal = signup_details_signal.clone();
                            let logs_task = signup_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let homeserver_pk = PublicKey::try_from(homeserver.as_str())
                                        .map_err(|e| anyhow!("Invalid homeserver key: {e}"))?;
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
                            let maybe_pubky = { signin_pubky_state.read().facade() };
                            let Some(pubky) = maybe_pubky else {
                                push_log(
                                    signin_logs.clone(),
                                    LogLevel::Info,
                                    "Pubky facade is still starting up. Try again shortly.",
                                );
                                return;
                            };
                            let logs_task = signin_logs.clone();
                            let mut session_signal = signin_session_signal.clone();
                            let mut details_signal = signin_details_signal.clone();
                            spawn(async move {
                                let result = async move {
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
