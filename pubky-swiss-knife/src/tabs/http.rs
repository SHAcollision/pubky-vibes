use anyhow::anyhow;
use dioxus::prelude::*;
use pubky::PubkyHttpClient;
use reqwest::Method;
use reqwest::header::HeaderName;
use url::Url;

use crate::app::NetworkMode;
use crate::utils::http::format_response;
use crate::utils::logging::{LogEntry, LogLevel, push_log};

pub fn render_http_tab(
    network_mode: Signal<NetworkMode>,
    http_method: Signal<String>,
    http_url: Signal<String>,
    http_headers: Signal<String>,
    http_body: Signal<String>,
    http_response: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let method_value = { http_method.read().clone() };
    let url_value = { http_url.read().clone() };
    let headers_value = { http_headers.read().clone() };
    let body_value = { http_body.read().clone() };
    let response_value = { http_response.read().clone() };

    let mut method_binding = http_method;
    let mut url_binding = http_url;
    let mut headers_binding = http_headers;
    let mut body_binding = http_body;

    let request_method_signal = http_method;
    let request_url_signal = http_url;
    let request_headers_signal = http_headers;
    let request_body_signal = http_body;
    let request_response_signal = http_response;
    let request_logs = logs;
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
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Send the request through the Pubky-aware client",
                        onclick: move |_| {
                            let method = request_method_signal.read().clone();
                            let url = request_url_signal.read().clone();
                        if url.trim().is_empty() {
                            push_log(request_logs, LogLevel::Error, "Provide a URL");
                            return;
                        }
                            let headers = request_headers_signal.read().clone();
                            let body = request_body_signal.read().clone();
                        let mut response_signal = request_response_signal;
                        let logs_task = request_logs;
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
                                    Ok(msg) => push_log(logs_task, LogLevel::Success, format!("Request completed: {msg}")),
                                    Err(err) => push_log(logs_task, LogLevel::Error, format!("Request failed: {err}")),
                                }
                            });
                        },
                        "Send"
                    }
                }
                if !response_value.is_empty() {
                    div { class: "outputs", {response_value} }
                }
            }
        }
    }
}
