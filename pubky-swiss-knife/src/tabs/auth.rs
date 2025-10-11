use anyhow::{Context, anyhow};
use dioxus::prelude::*;
use pubky::{Capabilities, Keypair, PubkyAuthFlow, PubkySession};
use url::Url;

use crate::tabs::format_session_info;
use crate::utils::logging::{LogEntry, LogLevel, push_log};
use crate::utils::pubky::PubkyFacadeState;
use crate::utils::qr::generate_qr_data_url;

#[allow(clippy::too_many_arguments, clippy::clone_on_copy)]
pub fn render_auth_tab(
    pubky_state: Signal<PubkyFacadeState>,
    keypair: Signal<Option<Keypair>>,
    session: Signal<Option<PubkySession>>,
    session_details: Signal<String>,
    auth_caps_input: Signal<String>,
    auth_relay_input: Signal<String>,
    auth_url_output: Signal<String>,
    auth_qr_data: Signal<Option<String>>,
    auth_status: Signal<String>,
    auth_flow: Signal<Option<PubkyAuthFlow>>,
    auth_request_input: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let caps_value = { auth_caps_input.read().clone() };
    let relay_value = { auth_relay_input.read().clone() };
    let url_value = { auth_url_output.read().clone() };
    let status_value = { auth_status.read().clone() };
    let qr_value = { auth_qr_data.read().clone() };
    let request_value = { auth_request_input.read().clone() };

    let mut caps_binding = auth_caps_input.clone();
    let mut relay_binding = auth_relay_input.clone();
    let mut request_binding = auth_request_input.clone();

    let start_pubky_state = pubky_state.clone();
    let start_caps_signal = auth_caps_input.clone();
    let start_relay_signal = auth_relay_input.clone();
    let start_flow_signal = auth_flow.clone();
    let start_url_signal = auth_url_output.clone();
    let start_qr_signal = auth_qr_data.clone();
    let start_status_signal = auth_status.clone();
    let start_logs = logs.clone();

    let mut await_flow_signal = auth_flow.clone();
    let mut await_status_signal = auth_status.clone();
    let await_url_signal = auth_url_output.clone();
    let await_qr_signal = auth_qr_data.clone();
    let await_session_signal = session.clone();
    let await_details_signal = session_details.clone();
    let await_logs = logs.clone();

    let mut cancel_flow_signal = auth_flow.clone();
    let mut cancel_status_signal = auth_status.clone();
    let mut cancel_url_signal = auth_url_output.clone();
    let mut cancel_qr_signal = auth_qr_data.clone();
    let cancel_logs = logs.clone();

    let approve_pubky_state = pubky_state.clone();
    let approve_keypair = keypair.clone();
    let approve_request_signal = auth_request_input.clone();
    let approve_logs = logs.clone();

    rsx! {
        div { class: "tab-body",
            section { class: "card span-2",
                h2 { "Request third-party authentication" }
                p { class: "helper-text", "Generate a pubkyauth:// link and QR code that another user can approve with their Pubky signer." }
                div { class: "form-grid",
                    label {
                        "Requested capabilities"
                        input {
                            value: caps_value,
                            oninput: move |evt| caps_binding.set(evt.value()),
                            title: "Capability string consumed by PubkyAuthFlow::builder and Capabilities::try_from",
                            placeholder: "Example: /pub/app/:rw"
                        }
                    }
                    label {
                        "Relay override (optional)"
                        input {
                            value: relay_value,
                            oninput: move |evt| relay_binding.set(evt.value()),
                            title: "Optional relay URL passed to PubkyAuthFlow::builder::relay",
                            placeholder: "https://your-relay.example/link/"
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Start PubkyAuthFlow::start to mint a pubkyauth:// authorization URL",
                        onclick: move |_| {
                        let caps_text = start_caps_signal.read().clone();
                        if caps_text.trim().is_empty() {
                            push_log(start_logs.clone(), LogLevel::Error, "Provide capabilities for the request");
                            return;
                        }
                        let relay_text = start_relay_signal.read().clone();
                        let maybe_pubky = { start_pubky_state.read().facade() };
                        let Some(pubky) = maybe_pubky else {
                            push_log(
                                start_logs.clone(),
                                LogLevel::Info,
                                "Pubky facade is still starting up. Try again shortly.",
                            );
                            return;
                        };
                        let pubky = pubky.clone();
                        let mut flow_slot = start_flow_signal.clone();
                        let mut url_slot = start_url_signal.clone();
                        let mut qr_slot = start_qr_signal.clone();
                        let mut status_slot = start_status_signal.clone();
                        let logs_task = start_logs.clone();
                        spawn(async move {
                            let result = async move {
                                let capabilities = Capabilities::try_from(caps_text.trim())
                                    .map_err(|e| anyhow!("Invalid capabilities: {e}"))?;
                                let flow = if relay_text.trim().is_empty() {
                                    pubky.start_auth_flow(&capabilities)?
                                } else {
                                    let relay = Url::parse(relay_text.trim())
                                        .context("Relay URL must be valid")?;
                                    PubkyAuthFlow::builder(&capabilities)
                                        .client(pubky.client().clone())
                                        .relay(relay)
                                        .start()?
                                };
                                let auth_url = flow.authorization_url().to_string();
                                let data_url = generate_qr_data_url(&auth_url)?;
                                flow_slot.set(Some(flow));
                                url_slot.set(auth_url.clone());
                                qr_slot.set(Some(data_url));
                                status_slot.set(String::from("Awaiting remote approval..."));
                                Ok::<_, anyhow::Error>(format!("Auth flow ready: {auth_url}"))
                            };
                            match result.await {
                                Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                Err(err) => {
                                    flow_slot.set(None);
                                    url_slot.set(String::new());
                                    qr_slot.set(None);
                                    status_slot.set(String::new());
                                    push_log(logs_task, LogLevel::Error, format!("Failed to start auth flow: {err}"));
                                }
                            }
                        });
                        },
                    "Start auth flow",
                    }
                    button {
                        class: "action",
                        title: "Wait on PubkyAuthFlow::await_approval to exchange the link for a session",
                        onclick: move |_| {
                        let maybe_flow = {
                            let mut guard = await_flow_signal.write();
                            guard.take()
                        };
                        if let Some(flow) = maybe_flow {
                            await_status_signal.set(String::from("Waiting for remote approval..."));
                            let mut url_slot = await_url_signal.clone();
                            let mut qr_slot = await_qr_signal.clone();
                            let mut status_slot = await_status_signal.clone();
                            let mut session_slot = await_session_signal.clone();
                            let mut details_slot = await_details_signal.clone();
                            let logs_task = await_logs.clone();
                            spawn(async move {
                                match flow.await_approval().await {
                                    Ok(new_session) => {
                                        let info = new_session.info().clone();
                                        details_slot.set(format_session_info(&info));
                                        session_slot.set(Some(new_session));
                                        status_slot.set(format!("Approved by {}", info.public_key()));
                                        url_slot.set(String::new());
                                        qr_slot.set(None);
                                        push_log(
                                            logs_task,
                                            LogLevel::Success,
                                            format!("Auth flow approved by {}", info.public_key()),
                                        );
                                    }
                                    Err(err) => {
                                        status_slot.set(String::from("Auth approval failed"));
                                        push_log(
                                            logs_task,
                                            LogLevel::Error,
                                            format!("Auth approval failed: {err}"),
                                        );
                                    }
                                }
                            });
                        } else {
                            push_log(await_logs, LogLevel::Error, "Start an auth flow first");
                        }
                        },
                    "Await approval",
                    }
                    button {
                        class: "action secondary",
                        title: "Discard the in-progress PubkyAuthFlow without contacting the relay",
                        onclick: move |_| {
                            let had_flow = {
                                let mut guard = cancel_flow_signal.write();
                                guard.take().is_some()
                            };
                            cancel_status_signal.set(String::new());
                            cancel_url_signal.set(String::new());
                            cancel_qr_signal.set(None);
                            if had_flow {
                                push_log(cancel_logs.clone(), LogLevel::Info, "Auth flow cancelled");
                            } else {
                                push_log(cancel_logs, LogLevel::Error, "No auth flow to cancel");
                            }
                        },
                    "Cancel",
                    }
                }
                if !status_value.is_empty() {
                    p { class: "auth-status", {status_value} }
                }
                if qr_value.is_some() || !url_value.trim().is_empty() {
                    div { class: "qr-container",
                        if let Some(data_url) = qr_value {
                            img { src: data_url, alt: "pubkyauth QR code" }
                        }
                        textarea {
                            class: "tall",
                            readonly: true,
                            value: url_value,
                            title: "Share this pubkyauth:// URL with a signer to request delegated capabilities",
                            placeholder: "Generated pubkyauth:// link"
                        }
                    }
                }
            }
            section { class: "card span-2",
                h2 { "Approve a pubkyauth:// request" }
                p { class: "helper-text", "Paste a request URL and approve it using the active keypair." }
                div { class: "form-grid",
                    label {
                        "pubkyauth:// URL"
                        textarea {
                            class: "tall",
                            value: request_value,
                            oninput: move |evt| request_binding.set(evt.value()),
                            title: "Paste a pubkyauth:// URL received from another party for signer.approve_auth",
                            placeholder: "pubkyauth:///?caps=..."
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Use signer.approve_auth to grant the requested capabilities",
                        onclick: move |_| {
                            let url = approve_request_signal.read().clone();
                            if url.trim().is_empty() {
                                push_log(approve_logs.clone(), LogLevel::Error, "Paste a pubkyauth:// URL to approve");
                                return;
                            }
                            let maybe_pubky = { approve_pubky_state.read().facade() };
                            let Some(pubky) = maybe_pubky else {
                                push_log(
                                    approve_logs.clone(),
                                    LogLevel::Info,
                                    "Pubky facade is still starting up. Try again shortly.",
                                );
                                return;
                            };
                            if let Some(kp) = approve_keypair.read().as_ref().cloned() {
                                let url_string = url.trim().to_string();
                                let logs_task = approve_logs.clone();
                                let pubky = pubky.clone();
                                spawn(async move {
                                    let result = async move {
                                        let signer = pubky.signer(kp.clone());
                                        signer.approve_auth(&url_string).await?;
                                        Ok::<_, anyhow::Error>(format!(
                                            "Approved auth request with {}",
                                            kp.public_key()
                                        ))
                                    };
                                    match result.await {
                                        Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                        Err(err) => push_log(
                                            logs_task,
                                            LogLevel::Error,
                                            format!("Failed to approve auth request: {err}"),
                                        ),
                                    }
                                });
                            } else {
                                push_log(approve_logs, LogLevel::Error, "Load or generate a keypair first");
                            }
                        },
                        "Approve request",
                    }
                }
            }
        }
    }
}
