use anyhow::Result;

pub async fn format_response(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let version = response.version();
    let mut headers = Vec::new();
    for (name, value) in response.headers().iter() {
        if let Ok(text) = value.to_str() {
            headers.push(format!("{}: {}", name, text));
        }
    }
    let bytes = response.bytes().await?;
    let body = match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => format!("<binary {} bytes>", bytes.len()),
    };
    Ok(format!(
        "{version:?} {status}\n{}\n\n{body}",
        headers.join("\n")
    ))
}
