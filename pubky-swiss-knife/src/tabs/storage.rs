use dioxus::prelude::*;
use pubky::PubkySession;

use crate::app::NetworkMode;
use crate::utils::http::format_response;
use crate::utils::logging::{LogEntry, LogLevel, push_log};
use crate::utils::pubky::build_pubky;

#[allow(clippy::too_many_arguments)]
pub fn render_storage_tab(
    network_mode: Signal<NetworkMode>,
    session: Signal<Option<PubkySession>>,
    storage_path: Signal<String>,
    storage_body: Signal<String>,
    storage_response: Signal<String>,
    public_resource: Signal<String>,
    public_response: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let path_value = { storage_path.read().clone() };
    let body_value = { storage_body.read().clone() };
    let session_response = { storage_response.read().clone() };
    let public_value = { public_resource.read().clone() };
    let public_resp = { public_response.read().clone() };

    let mut storage_path_binding = storage_path;
    let mut storage_body_binding = storage_body;

    let storage_session_get = session;
    let storage_path_get = storage_path;
    let storage_response_get = storage_response;
    let storage_logs_get = logs;

    let storage_session_put = session;
    let storage_path_put = storage_path;
    let storage_body_put = storage_body;
    let storage_response_put = storage_response;
    let storage_logs_put = logs;

    let storage_session_delete = session;
    let storage_path_delete = storage_path;
    let storage_response_delete = storage_response;
    let storage_logs_delete = logs;

    let mut public_resource_binding = public_resource;
    let public_resource_signal = public_resource;
    let public_response_signal = public_response;
    let public_logs = logs;
    let public_network = network_mode;

    rsx! {
        div { class: "tab-body",
            section { class: "card",
                h2 { "Session storage" }
                p { class: "helper-text", "Operate on authenticated storage using the active session." }
                div { class: "form-grid",
                    label {
                        "Absolute path"
                        input { value: path_value.clone(), oninput: move |evt| storage_path_binding.set(evt.value()) }
                    }
                    label {
                        "Body"
                        textarea { class: "tall", value: body_value.clone(), oninput: move |evt| storage_body_binding.set(evt.value()) }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                        if let Some(session) = storage_session_get.read().as_ref().cloned() {
                            let path = storage_path_get.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_get, LogLevel::Error, "Provide a path to GET");
                                return;
                            }
                            let mut response_signal = storage_response_get;
                            let logs_task = storage_logs_get;
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().get(path.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Fetched {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("GET failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_get, LogLevel::Error, "No active session");
                        }
                    },
                    "GET"
                    }
                    button { class: "action secondary", onclick: move |_| {
                        if let Some(session) = storage_session_put.read().as_ref().cloned() {
                            let path = storage_path_put.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_put, LogLevel::Error, "Provide a path to PUT");
                                return;
                            }
                            let body = storage_body_put.read().clone();
                            let mut response_signal = storage_response_put;
                            let logs_task = storage_logs_put;
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().put(path.clone(), body.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Stored {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("PUT failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_put, LogLevel::Error, "No active session");
                        }
                    },
                    "PUT"
                    }
                    button { class: "action secondary", onclick: move |_| {
                        if let Some(session) = storage_session_delete.read().as_ref().cloned() {
                            let path = storage_path_delete.read().clone();
                            if path.trim().is_empty() {
                                push_log(storage_logs_delete, LogLevel::Error, "Provide a path to DELETE");
                                return;
                            }
                            let mut response_signal = storage_response_delete;
                            let logs_task = storage_logs_delete;
                            spawn(async move {
                                let result = async move {
                                    let resp = session.storage().delete(path.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Deleted {path}"))
                                };
                                match result.await {
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("DELETE failed: {err}")),
                                }
                            });
                        } else {
                            push_log(storage_logs_delete, LogLevel::Error, "No active session");
                        }
                    },
                    "DELETE"
                    }
                }
                if !session_response.is_empty() {
                    div { class: "outputs", {session_response} }
                }
            }
            section { class: "card",
                h2 { "Public storage" }
                p { class: "helper-text", "Fetch any public resource (pubky<pk>/path or pubky://...)." }
                div { class: "form-grid",
                    label {
                        "Resource"
                        input { value: public_value.clone(), oninput: move |evt| public_resource_binding.set(evt.value()) }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                        let resource = public_resource_signal.read().clone();
                        if resource.trim().is_empty() {
                            push_log(public_logs, LogLevel::Error, "Provide a resource to fetch");
                            return;
                        }
                        let mut response_signal = public_response_signal;
                        let logs_task = public_logs;
                        let network = *public_network.read();
                        spawn(async move {
                            let result = async move {
                                let pubky = build_pubky(network)?;
                                let resp = pubky.public_storage().get(resource.clone()).await?;
                                let formatted = format_response(resp).await?;
                                response_signal.set(formatted.clone());
                                Ok::<_, anyhow::Error>(format!("Fetched public resource {resource}"))
                            };
                            match result.await {
                                Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                Err(err) => push_log(logs_task, LogLevel::Error, format!("Public GET failed: {err}")),
                            }
                        });
                    },
                    "GET"
                    }
                }
                if !public_resp.is_empty() {
                    div { class: "outputs", {public_resp} }
                }
            }
        }
    }
}
