use anyhow::anyhow;
use dioxus::prelude::*;
use pubky::PubkyHttpClient;
use reqwest::Method;
use reqwest::header::HeaderName;
use url::Url;

use crate::app::NetworkMode;
use crate::tabs::HttpTabState;
use crate::utils::http::format_response;
use crate::utils::logging::ActivityLog;
use crate::utils::mobile::{IS_ANDROID, touch_copy_option, touch_tooltip};

pub fn render_http_tab(
    network_mode: Signal<NetworkMode>,
    state: HttpTabState,
    logs: ActivityLog,
) -> Element {
    let HttpTabState {
        method,
        url,
        headers,
        body,
        response,
    } = state;

    let method_value = { method.read().clone() };
    let url_value = { url.read().clone() };
    let headers_value = { headers.read().clone() };
    let body_value = { body.read().clone() };
    let response_value = { response.read().clone() };
    let response_copy_value = if response_value.trim().is_empty() {
        None
    } else {
        Some(response_value.clone())
    };
    let response_copy_success = if IS_ANDROID {
        Some(String::from("Copied HTTP response to clipboard"))
    } else {
        None
    };

    let mut method_binding = method;
    let mut url_binding = url;
    let mut headers_binding = headers;
    let mut body_binding = body;

    let request_method_signal = method;
    let request_url_signal = url;
    let request_headers_signal = headers;
    let request_body_signal = body;
    let request_response_signal = response;
    let request_logs = logs.clone();
    let request_network = network_mode;

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Raw Pubky/HTTPS request" }
                div { class: "form-grid",
                    label {
                        "Method"
                        select {
                            value: method_value.clone(),
                            oninput: move |evt| method_binding.set(evt.value()),
                            title: "Choose the HTTP method for this request",
                            "data-touch-tooltip": touch_tooltip(
                                "Choose the HTTP method for this request",
                            ),
                            for option in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                                option { value: option, selected: method_value == option, "{option}" }
                            }
                        }
                    }
                    label {
                        "URL"
                        input {
                            value: url_value.clone(),
                            oninput: move |evt| url_binding.set(evt.value()),
                            placeholder: "https:// or pubky://",
                            title: "Enter the destination URL, either https:// or pubky://",
                            "data-touch-tooltip": touch_tooltip(
                                "Enter the destination URL, either https:// or pubky://",
                            ),
                        }
                    }
                }
                div { class: "form-grid",
                    label {
                        "Headers (one per line, Name: Value)"
                        textarea {
                            class: "tall",
                            value: headers_value.clone(),
                            oninput: move |evt| headers_binding.set(evt.value()),
                            placeholder: "Header-Name: value",
                            title: "List any request headers, one per line in Name: Value format",
                            "data-touch-tooltip": touch_tooltip(
                                "List any request headers, one per line in Name: Value format",
                            ),
                        }
                    }
                    label {
                        "Body"
                        textarea {
                            class: "tall",
                            value: body_value.clone(),
                            oninput: move |evt| body_binding.set(evt.value()),
                            placeholder: "Request body (optional)",
                            title: "Optional request body to send",
                            "data-touch-tooltip": touch_tooltip(
                                "Optional request body to send",
                            ),
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Send the request through the Pubky-aware client",
                        "data-touch-tooltip": touch_tooltip(
                            "Send the request through the Pubky-aware client",
                        ),
                        onclick: move |_| {
                            let method = request_method_signal.read().clone();
                            let url = request_url_signal.read().clone();
                            if url.trim().is_empty() {
                                request_logs.error("Provide a URL");
                                return;
                            }
                            let headers = request_headers_signal.read().clone();
                            let body = request_body_signal.read().clone();
                            let mut response_signal = request_response_signal;
                            let logs_task = request_logs.clone();
                            let network = *request_network.read();
                            spawn(async move {
                                let result = async move {
                                    let method_parsed = Method::from_bytes(method.as_bytes())
                                        .map_err(|e| anyhow!("Invalid HTTP method: {e}"))?;
                                    let parsed_url = Url::parse(&url)?;
                                    let url_display = parsed_url.to_string();
                                    let client = match network {
                                        NetworkMode::Mainnet => PubkyHttpClient::new()?,
                                        NetworkMode::Testnet => PubkyHttpClient::testnet()?,
                                    };
                                    let mut request = client.request(method_parsed.clone(), parsed_url);
                                    for line in headers.lines() {
                                        if line.trim().is_empty() {
                                            continue;
                                        }
                                        let (name, value) = line
                                            .split_once(':')
                                            .ok_or_else(|| anyhow!("Header must use Name: Value format"))?;
                                        let header_name: HeaderName = name.trim().parse()?;
                                        request = request.header(header_name, value.trim());
                                    }
                                    if !body.is_empty() {
                                        request = request.body(body.clone());
                                    }
                                    let response = request.send().await?;
                                    let formatted = format_response(response).await?;
                                    response_signal.set(formatted.clone());
                                    Ok::<_, anyhow::Error>(format!("{method_parsed} {url_display}"))
                                };
                                match result.await {
                                    Ok(msg) => logs_task.success(format!("Request completed: {msg}")),
                                    Err(err) => logs_task.error(format!("Request failed: {err}")),
                                }
                            });
                        },
                        "Send"
                    }
                }
                if !response_value.is_empty() {
                    div {
                        class: "outputs copyable",
                        "data-touch-tooltip": touch_tooltip(
                            "Tap to copy the HTTP response",
                        ),
                        "data-touch-copy": touch_copy_option(response_copy_value.clone()),
                        "data-copy-success": response_copy_success.clone(),
                        {response_value}
                    }
                }
            }
        }
    }
}
