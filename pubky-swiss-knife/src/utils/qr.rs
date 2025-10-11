use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use qrcode::{QrCode, render::svg};

pub fn generate_qr_data_url(content: &str) -> Result<String> {
    let code = QrCode::new(content.as_bytes()).context("failed to encode QR code")?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(280, 280)
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#f8fafc"))
        .build();
    let encoded = STANDARD.encode(svg.as_bytes());
    Ok(format!("data:image/svg+xml;base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn generate_qr_data_url_encodes_svg() -> Result<()> {
        let qr = generate_qr_data_url("pubkyauth://example")?;
        assert!(qr.starts_with("data:image/svg+xml;base64,"));
        let encoded = qr.trim_start_matches("data:image/svg+xml;base64,");
        let svg_bytes = STANDARD.decode(encoded)?;
        let svg = String::from_utf8(svg_bytes).expect("qr svg should be utf8");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#0f172a"), "dark color should be embedded");
        assert!(svg.contains("#f8fafc"), "light color should be embedded");
        Ok(())
    }
}
