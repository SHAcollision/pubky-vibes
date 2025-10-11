use anyhow::anyhow;
use dioxus::prelude::*;
use pubky::PublicKey;

use crate::tabs::{SessionsTabState, format_session_info};
use crate::utils::logging::ActivityLog;
use crate::utils::pubky::PubkyFacadeHandle;

#[allow(clippy::clone_on_copy)]
pub fn render_sessions_tab(
    pubky: PubkyFacadeHandle,
    state: SessionsTabState,
    logs: ActivityLog,
) -> Element {
    let SessionsTabState {
        keypair,
        session,
        details,
        homeserver,
        signup_code,
    } = state;

    let homeserver_value = { homeserver.read().clone() };
    let signup_value = { signup_code.read().clone() };
    let details_value = { details.read().clone() };

    let mut homeserver_binding = homeserver.clone();
    let mut signup_binding = signup_code.clone();

    let signup_keypair = keypair.clone();
    let signup_homeserver = homeserver.clone();
    let signup_code_signal = signup_code.clone();
    let signup_session_signal = session.clone();
    let signup_details_signal = details.clone();
    let signup_logs = logs.clone();
    let signup_pubky = pubky.clone();

    let signin_keypair = keypair.clone();
    let signin_session_signal = session.clone();
    let signin_details_signal = details.clone();
    let signin_logs = logs.clone();
    let signin_pubky = pubky.clone();

    let revalidate_session_signal = session.clone();
    let revalidate_details_signal = details.clone();
    let revalidate_logs = logs.clone();

    let signout_session_signal = session.clone();
    let signout_details_signal = details.clone();
    let signout_logs = logs.clone();

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Session lifecycle" }
                div { class: "form-grid",
                    label {
                        "Homeserver public key"
                        input {
                            value: homeserver_value,
                            oninput: move |evt| homeserver_binding.set(evt.value()),
                            title: "Base32 public key of the homeserver used by PubkySession::signup",
                        }
                    }
                    label {
                        "Signup code (optional)"
                        input {
                            value: signup_value,
                            oninput: move |evt| signup_binding.set(evt.value()),
                            title: "Optional invitation code passed to PubkySession::signup",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Call signer.signup to create a session bound to the homeserver",
                        onclick: move |_| {
                            if let Some(kp) = signup_keypair.read().as_ref().cloned() {
                                let homeserver = signup_homeserver.read().clone();
                                if homeserver.trim().is_empty() {
                                    signup_logs.error("Homeserver public key is required");
                                    return;
                                }
                                let signup_code_value = signup_code_signal.read().clone();
                                let Some(pubky) = signup_pubky.ready_or_log(&signup_logs) else {
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
                                        let code = if signup_code_value.trim().is_empty() {
                                            None
                                        } else {
                                            Some(signup_code_value.as_str())
                                        };
                                        let session = signer.signup(&homeserver_pk, code).await?;
                                        session_signal.set(Some(session.clone()));
                                        details_signal.set(format_session_info(session.info()));
                                        Ok::<_, anyhow::Error>(format!("Signed up as {}", session.info().public_key()))
                                    };
                                    match result.await {
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!("Signup failed: {err}")),
                                    }
                                });
                            } else {
                                signup_logs.error("Load or generate a key first");
                            }
                        },
                        "Sign up"
                    }
                    button {
                        class: "action secondary",
                        title: "Invoke signer.signin to obtain a root Pubky session",
                        onclick: move |_| {
                            if let Some(kp) = signin_keypair.read().as_ref().cloned() {
                                let Some(pubky) = signin_pubky.ready_or_log(&signin_logs) else {
                                    return;
                                };
                                let mut session_signal = signin_session_signal.clone();
                                let mut details_signal = signin_details_signal.clone();
                                let logs_task = signin_logs.clone();
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
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!("Signin (root) failed: {err}")),
                                    }
                                });
                            } else {
                                signin_logs.error("Load or generate a key first");
                            }
                        },
                        "Sign in (root)"
                    }
                    button {
                        class: "action secondary",
                        title: "Use session.revalidate to confirm the access token is still accepted",
                        onclick: move |_| {
                            if let Some(session) = revalidate_session_signal.read().as_ref().cloned() {
                                let mut session_signal = revalidate_session_signal.clone();
                                let mut details_signal = revalidate_details_signal.clone();
                                let logs_task = revalidate_logs.clone();
                                spawn(async move {
                                    match session.revalidate().await {
                                        Ok(Some(info)) => {
                                            details_signal.set(format_session_info(&info));
                                            logs_task.success("Session still valid");
                                        }
                                        Ok(None) => {
                                            session_signal.set(None);
                                            details_signal.set(String::new());
                                            logs_task.error("Session expired or missing");
                                        }
                                        Err(err) => logs_task.error(format!("Revalidate failed: {err}")),
                                    }
                                });
                            } else {
                                revalidate_logs.error("No active session");
                            }
                        },
                        "Revalidate"
                    }
                    button {
                        class: "action secondary",
                        title: "Call PubkySession::signout to revoke the session token",
                        onclick: move |_| {
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
                                            logs_task.success("Signed out successfully");
                                        }
                                        Err((err, session_back)) => {
                                            session_signal.set(Some(session_back));
                                            logs_task.error(format!("Signout failed: {err}"));
                                        }
                                    }
                                });
                            } else {
                                signout_logs.error("No active session");
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
