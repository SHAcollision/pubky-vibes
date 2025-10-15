use anyhow::Result;
use reqwest::{
    StatusCode, Version,
    header::{CONTENT_TYPE, HeaderMap},
};
use serde_json::Value;

pub async fn format_response(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let version = response.version();
    let headers = response.headers().clone();
    let bytes = response.bytes().await?;
    Ok(format_response_parts(status, version, &headers, &bytes))
}

pub fn format_response_parts(
    status: StatusCode,
    version: Version,
    headers: &HeaderMap,
    body: &[u8],
) -> String {
    let mut header_lines = Vec::new();
    let mut content_type = None;
    for (name, value) in headers.iter() {
        if let Ok(text) = value.to_str() {
            if name == CONTENT_TYPE {
                content_type = Some(text.to_lowercase());
            }
            header_lines.push(format!("{}: {}", name, text));
        }
    }
    let body = render_body(body, content_type.as_deref());
    format!(
        "{version:?} {status}\n{}\n\n{body}",
        header_lines.join("\n")
    )
}

fn render_body(bytes: &[u8], content_type: Option<&str>) -> String {
    let ct = content_type.unwrap_or_default();
    if ct.contains("application/json") {
        match serde_json::from_slice::<Value>(bytes) {
            Ok(json) => {
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| fallback_text(bytes))
            }
            Err(_) => fallback_text(bytes),
        }
    } else if ct.starts_with("text/") || ct.contains("charset") {
        fallback_text(bytes)
    } else {
        binary_preview(bytes)
    }
}

fn fallback_text(bytes: &[u8]) -> String {
    match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => binary_preview(bytes),
    }
}

fn binary_preview(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::from("<empty body>");
    }
    let sample = bytes
        .iter()
        .take(32)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > 32 {
        format!("<binary {} bytes: {} …>", bytes.len(), sample)
    } else {
        format!("<binary {} bytes: {}>", bytes.len(), sample)
    }
}
