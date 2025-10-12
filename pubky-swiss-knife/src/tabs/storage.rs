use dioxus::prelude::*;

use crate::tabs::StorageTabState;
use crate::utils::http::format_response;
use crate::utils::logging::ActivityLog;
use crate::utils::pubky::PubkyFacadeHandle;

#[allow(clippy::too_many_arguments, clippy::clone_on_copy)]
pub fn render_storage_tab(
    pubky: PubkyFacadeHandle,
    state: StorageTabState,
    logs: ActivityLog,
) -> Element {
    let StorageTabState {
        session,
        path,
        body,
        response,
        public_resource,
        public_response,
    } = state;

    let path_value = { path.read().clone() };
    let body_value = { body.read().clone() };
    let session_response = { response.read().clone() };
    let public_value = { public_resource.read().clone() };
    let public_resp = { public_response.read().clone() };

    let mut storage_path_binding = path.clone();
    let mut storage_body_binding = body.clone();

    let storage_session_get = session.clone();
    let storage_path_get = path.clone();
    let storage_response_get = response.clone();
    let storage_logs_get = logs.clone();

    let storage_session_put = session.clone();
    let storage_path_put = path.clone();
    let storage_body_put = body.clone();
    let storage_response_put = response.clone();
    let storage_logs_put = logs.clone();

    let storage_session_delete = session.clone();
    let storage_path_delete = path.clone();
    let storage_response_delete = response.clone();
    let storage_logs_delete = logs.clone();

    let mut public_resource_binding = public_resource.clone();
    let public_resource_signal = public_resource.clone();
    let public_response_signal = public_response.clone();
    let public_logs = logs.clone();

    rsx! {
        div { class: "tab-body",
            section { class: "card",
                h2 { "Session storage" }
                p { class: "helper-text", "Operate on authenticated storage using the active session." }
                div { class: "form-grid",
                    label {
                        "Absolute path"
                        input {
                            value: path_value.clone(),
                            oninput: move |evt| storage_path_binding.set(evt.value()),
                            title: "Absolute path inside your session's private storage",
                        }
                    }
                    label {
                        "Body"
                        textarea {
                            class: "tall",
                            value: body_value.clone(),
                            oninput: move |evt| storage_body_binding.set(evt.value()),
                            title: "Content to upload when storing data",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Fetch the stored value at this path",
                        onclick: move |_| {
                            if let Some(session) = storage_session_get.read().as_ref().cloned() {
                                let path = storage_path_get.read().clone();
                                if path.trim().is_empty() {
                                    storage_logs_get.error("Provide a path to GET");
                                    return;
                                }
                                let mut response_signal = storage_response_get.clone();
                                let logs_task = storage_logs_get.clone();
                                spawn(async move {
                                    let result = async move {
                                        let resp = session.storage().get(path.clone()).await?;
                                        let formatted = format_response(resp).await?;
                                        response_signal.set(formatted.clone());
                                        Ok::<_, anyhow::Error>(format!("Fetched {path}"))
                                    };
                                    match result.await {
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!("GET failed: {err}")),
                                    }
                                });
                            } else {
                                storage_logs_get.error("No active session");
                            }
                        },
                        "GET",
                    }
                    button {
                        class: "action secondary",
                        title: "Write the body above to this storage path",
                        onclick: move |_| {
                            if let Some(session) = storage_session_put.read().as_ref().cloned() {
                                let path = storage_path_put.read().clone();
                                if path.trim().is_empty() {
                                    storage_logs_put.error("Provide a path to PUT");
                                    return;
                                }
                                let body = storage_body_put.read().clone();
                                let mut response_signal = storage_response_put.clone();
                                let logs_task = storage_logs_put.clone();
                                spawn(async move {
                                    let result = async move {
                                        let resp = session.storage().put(path.clone(), body.clone()).await?;
                                        let formatted = format_response(resp).await?;
                                        response_signal.set(formatted.clone());
                                        Ok::<_, anyhow::Error>(format!("Stored {path}"))
                                    };
                                    match result.await {
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!("PUT failed: {err}")),
                                    }
                                });
                            } else {
                                storage_logs_put.error("No active session");
                            }
                        },
                        "PUT",
                    }
                    button {
                        class: "action secondary",
                        title: "Delete the resource stored at this path",
                        onclick: move |_| {
                            if let Some(session) = storage_session_delete.read().as_ref().cloned() {
                                let path = storage_path_delete.read().clone();
                                if path.trim().is_empty() {
                                    storage_logs_delete.error("Provide a path to DELETE");
                                    return;
                                }
                                let mut response_signal = storage_response_delete.clone();
                                let logs_task = storage_logs_delete.clone();
                                spawn(async move {
                                    let result = async move {
                                        let resp = session.storage().delete(path.clone()).await?;
                                        let formatted = format_response(resp).await?;
                                        response_signal.set(formatted.clone());
                                        Ok::<_, anyhow::Error>(format!("Deleted {path}"))
                                    };
                                    match result.await {
                                        Ok(msg) => logs_task.success(msg),
                                        Err(err) => logs_task.error(format!("DELETE failed: {err}")),
                                    }
                                });
                            } else {
                                storage_logs_delete.error("No active session");
                            }
                        },
                        "DELETE",
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
                        input {
                            value: public_value.clone(),
                            oninput: move |evt| public_resource_binding.set(evt.value()),
                            title: "Enter a public storage path or pubky:// link to fetch",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Fetch the public resource using the Pubky client",
                        onclick: move |_| {
                            let resource = public_resource_signal.read().clone();
                            if resource.trim().is_empty() {
                                public_logs.error("Provide a resource to fetch");
                                return;
                            }
                            let Some(pubky) = pubky.ready_or_log(&public_logs) else {
                                return;
                            };
                            let mut response_signal = public_response_signal.clone();
                            let logs_task = public_logs.clone();
                            spawn(async move {
                                let result = async move {
                                    let resp = pubky.public_storage().get(resource.clone()).await?;
                                    let formatted = format_response(resp).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("Fetched public resource {resource}"))
                                };
                                match result.await {
                                    Ok(msg) => logs_task.success(msg),
                                    Err(err) => logs_task.error(format!("Public GET failed: {err}")),
                                }
                            });
                        },
                        "GET",
                    }
                }
                if !public_resp.is_empty() {
                    div { class: "outputs", {public_resp} }
                }
            }
        }
    }
}
