use anyhow::anyhow;
use dioxus::prelude::*;
use pubky::{PubkyHttpClient, PublicKey};
use reqwest::Method;
use reqwest::header::HeaderName;
use url::Url;

use crate::app::NetworkMode;
use crate::utils::http::format_response;
use crate::utils::iroh::{
    format_discovery_summary, parse_homeserver_key, request_over_iroh, resolve_iroh_records,
};
use crate::utils::logging::{LogEntry, LogLevel, push_log};

pub fn render_http_tab(
    network_mode: Signal<NetworkMode>,
    http_method: Signal<String>,
    http_url: Signal<String>,
    http_headers: Signal<String>,
    http_body: Signal<String>,
    http_response: Signal<String>,
    iroh_target: Signal<String>,
    iroh_summary: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let method_value = { http_method.read().clone() };
    let url_value = { http_url.read().clone() };
    let headers_value = { http_headers.read().clone() };
    let body_value = { http_body.read().clone() };
    let response_value = { http_response.read().clone() };
    let iroh_target_value = { iroh_target.read().clone() };
    let iroh_summary_value = { iroh_summary.read().clone() };

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
    let mut iroh_target_signal = iroh_target;
    let iroh_summary_signal = iroh_summary;

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
                        }
                    }
                    label {
                        "Body"
                        textarea {
                            class: "tall",
                            value: body_value.clone(),
                            oninput: move |evt| body_binding.set(evt.value()),
                            placeholder: "Request body (optional)",
                        }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
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
                                    let mut request = client.request(method_parsed.clone(), parsed_url.clone());
                                    let mut parsed_headers: Vec<(HeaderName, String)> = Vec::new();
                                    for line in headers.lines() {
                                        if line.trim().is_empty() {
                                            continue;
                                        }
                                        let (name, value) = line
                                            .split_once(':')
                                            .ok_or_else(|| anyhow!("Header must use Name: Value format"))?;
                                        let header_name: HeaderName = name.trim().parse()?;
                                        let header_value = value.trim().to_string();
                                        parsed_headers.push((header_name.clone(), header_value.clone()));
                                        request = request.header(header_name, header_value);
                                    }
                                    if !body.is_empty() {
                                        request = request.body(body.clone());
                                    }
                                    match request.send().await {
                                        Ok(response) => {
                                            let formatted = format_response(response).await?;
                                            response_signal.set(formatted.clone());
                                            Ok::<_, anyhow::Error>(format!("{method_parsed} {url_display}"))
                                        }
                                        Err(http_error) => {
                                            if let Some(homeserver) = parse_homeserver_key(&parsed_url) {
                                                let use_testnet = matches!(network, NetworkMode::Testnet);
                                                let fallback = request_over_iroh(
                                                    &homeserver,
                                                    &method_parsed,
                                                    &parsed_url,
                                                    &parsed_headers,
                                                    body.as_bytes(),
                                                    use_testnet,
                                                )
                                                .await;
                                                match fallback {
                                                    Ok(response_text) => {
                                                        response_signal.set(response_text.clone());
                                                        Ok(format!(
                                                            "{method_parsed} {url_display} via Iroh"
                                                        ))
                                                    }
                                                    Err(iroh_error) => Err(anyhow!(
                                                        "Request failed: {http_error}; Iroh fallback failed: {iroh_error}"
                                                    )),
                                                }
                                            } else {
                                                Err(anyhow!("Request failed: {http_error}"))
                                            }
                                        }
                                    }
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
                div { class: "form-grid",
                    label {
                        "Homeserver public key",
                        input {
                            value: iroh_target_value,
                            oninput: move |evt| iroh_target_signal.set(evt.value()),
                            placeholder: "o…",
                        }
                    }
                    label {
                        "_iroh discovery",
                        textarea {
                            class: "tall",
                            readonly: true,
                            value: iroh_summary_value,
                        }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action secondary", onclick: move |_| {
                        let target = iroh_target_signal.read().clone();
                        let testnet = matches!(*request_network.read(), NetworkMode::Testnet);
                        if target.trim().is_empty() {
                            push_log(request_logs.clone(), LogLevel::Error, "Provide a homeserver key");
                            return;
                        }
                        let mut summary_signal = iroh_summary_signal.clone();
                        summary_signal.set(String::new());
                        let logs_task = request_logs.clone();
                        spawn(async move {
                            let result = async move {
                                let trimmed = target.trim();
                                let homeserver = PublicKey::try_from(trimmed)
                                    .map_err(|e| anyhow!("Invalid homeserver key: {e}"))?;
                                match resolve_iroh_records(&homeserver, testnet).await? {
                                    Some(details) => {
                                        let formatted = format_discovery_summary(&details);
                                        summary_signal.set(formatted.clone());
                                        Ok::<_, anyhow::Error>(format!("Resolved _iroh records for {trimmed}"))
                                    }
                                    None => {
                                        summary_signal.set("No _iroh TXT attributes published.".to_string());
                                        Ok::<_, anyhow::Error>(format!("No _iroh records found for {trimmed}"))
                                    }
                                }
                            };
                            match result.await {
                                Ok(msg) => push_log(logs_task, LogLevel::Success, msg),
                                Err(err) => push_log(
                                    logs_task,
                                    LogLevel::Error,
                                    format!("_iroh discovery failed: {err}"),
                                ),
                            }
                        });
                    },
                    "Resolve _iroh"
                    }
                }
                if !response_value.is_empty() {
                    div { class: "outputs", {response_value} }
                }
            }
        }
    }
}
