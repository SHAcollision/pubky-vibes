use anyhow::{Context, anyhow};
use dioxus::prelude::*;
use pubky::{Capabilities, PubkyAuthFlow};
use url::Url;

use crate::tabs::{AuthTabState, format_session_info};
use crate::utils::logging::ActivityLog;
use crate::utils::pubky::PubkyFacadeHandle;
use crate::utils::qr::generate_qr_data_url;

#[allow(clippy::too_many_arguments, clippy::clone_on_copy)]
pub fn render_auth_tab(
    pubky: PubkyFacadeHandle,
    state: AuthTabState,
    logs: ActivityLog,
) -> Element {
    let AuthTabState {
        keypair,
        session,
        details,
        capabilities,
        relay,
        url_output,
        qr_data,
        status,
        flow,
        request_body,
    } = state;

    let caps_value = { capabilities.read().clone() };
    let relay_value = { relay.read().clone() };
    let url_value = { url_output.read().clone() };
    let status_value = { status.read().clone() };
    let qr_value = { qr_data.read().clone() };
    let request_value = { request_body.read().clone() };

    let mut caps_binding = capabilities.clone();
    let mut relay_binding = relay.clone();
    let mut request_binding = request_body.clone();

    let start_caps_signal = capabilities.clone();
    let start_relay_signal = relay.clone();
    let start_flow_signal = flow.clone();
    let start_url_signal = url_output.clone();
    let start_qr_signal = qr_data.clone();
    let start_status_signal = status.clone();
    let start_logs = logs.clone();

    let mut await_flow_signal = flow.clone();
    let mut await_status_signal = status.clone();
    let await_url_signal = url_output.clone();
    let await_qr_signal = qr_data.clone();
    let await_session_signal = session.clone();
    let await_details_signal = details.clone();
    let await_logs = logs.clone();

    let mut cancel_flow_signal = flow.clone();
    let mut cancel_status_signal = status.clone();
    let mut cancel_url_signal = url_output.clone();
    let mut cancel_qr_signal = qr_data.clone();
    let cancel_logs = logs.clone();

    let start_pubky = pubky.clone();
    let approve_pubky = pubky.clone();

    let approve_keypair = keypair.clone();
    let approve_request_signal = request_body.clone();
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
                            title: "Describe the permissions you're requesting, using the usual capability syntax",
                            placeholder: "Example: /pub/app/:rw"
                        }
                    }
                    label {
                        "Relay override (optional)"
                        input {
                            value: relay_value,
                            oninput: move |evt| relay_binding.set(evt.value()),
                            title: "Optional relay URL to direct this authorization through",
                            placeholder: "https://your-relay.example/link/"
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Create an authorization link and QR code with the current settings",
                        onclick: move |_| {
                        let caps_text = start_caps_signal.read().clone();
                        if caps_text.trim().is_empty() {
                            start_logs.error("Provide capabilities for the request");
                            return;
                        }
                        let relay_text = start_relay_signal.read().clone();
                        let Some(pubky) = start_pubky.ready_or_log(&start_logs) else {
                            return;
                        };
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
                                Ok(msg) => logs_task.success(msg),
                                Err(err) => {
                                    flow_slot.set(None);
                                    url_slot.set(String::new());
                                    qr_slot.set(None);
                                    status_slot.set(String::new());
                                    logs_task.error(format!("Failed to start auth flow: {err}"));
                                }
                            }
                        });
                        },
                    "Start auth flow",
                    }
                    button {
                        class: "action",
                        title: "Wait for the other party to approve and retrieve the resulting session",
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
                                        logs_task.success(format!(
                                            "Auth flow approved by {}",
                                            info.public_key()
                                        ));
                                    }
                                    Err(err) => {
                                        status_slot.set(String::from("Auth approval failed"));
                                        logs_task.error(format!("Auth approval failed: {err}"));
                                    }
                                }
                            });
                        } else {
                            await_logs.error("Start an auth flow first");
                        }
                        },
                    "Await approval",
                    }
                    button {
                        class: "action secondary",
                        title: "Cancel the current authorization request",
                        onclick: move |_| {
                            let had_flow = {
                                let mut guard = cancel_flow_signal.write();
                                guard.take().is_some()
                            };
                            cancel_status_signal.set(String::new());
                            cancel_url_signal.set(String::new());
                            cancel_qr_signal.set(None);
                            if had_flow {
                                cancel_logs.info("Auth flow cancelled");
                            } else {
                                cancel_logs.error("No auth flow to cancel");
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
                            title: "Share this link with someone to request delegated capabilities",
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
                            title: "Paste a pubkyauth:// link you were given",
                            placeholder: "pubkyauth:///?caps=..."
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Approve the request using your loaded key",
                        onclick: move |_| {
                            let url = approve_request_signal.read().clone();
                            if url.trim().is_empty() {
                                approve_logs.error("Paste a pubkyauth:// URL to approve");
                                return;
                            }
                            let Some(pubky) = approve_pubky.ready_or_log(&approve_logs) else {
                                return;
                            };
                            if let Some(kp) = approve_keypair.read().as_ref().cloned() {
                                let url_string = url.trim().to_string();
                                let logs_task = approve_logs.clone();
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
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!(
                                            "Failed to approve auth request: {err}"
                                        )),
                                    }
                                });
                            } else {
                                approve_logs.error("Load or generate a keypair first");
                            }
                        },
                        "Approve request",
                    }
                }
            }
        }
    }
}
