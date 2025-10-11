use anyhow::Result;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

pub async fn format_response(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let version = response.version();
    let mut headers = Vec::new();
    let mut content_type = None;
    for (name, value) in response.headers().iter() {
        if let Ok(text) = value.to_str() {
            if name == CONTENT_TYPE {
                content_type = Some(text.to_lowercase());
            }
            headers.push(format!("{}: {}", name, text));
        }
    }
    let bytes = response.bytes().await?;
    let body = render_body(&bytes, content_type.as_deref());
    Ok(format!(
        "{version:?} {status}\n{}\n\n{body}",
        headers.join("\n")
    ))
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
