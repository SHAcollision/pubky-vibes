//! Helpers for resolving `_iroh` TXT records and issuing HTTP requests over Iroh tunnels.

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::{self, Request};
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use iroh::{Endpoint, NodeAddr, RelayMap, RelayMode, SecretKey};
use pin_project_lite::pin_project;
use pkarr::{Client, PublicKey, dns::rdata::RData};
use pubky_common::constants::testnet_ports;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use reqwest::{Method, header::HeaderName as ReqwestHeaderName};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::lookup_host;
use tracing::debug;
use url::{Host, Url};

use std::{
    net::SocketAddr,
    task::{Context as TaskContext, Poll},
};

/// Summary of the discovery information published alongside HTTPS records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrohDiscoveryDetails {
    /// Relay URL advertised by the homeserver, if any.
    pub relay: Option<Url>,
    /// Direct socket addresses reachable without a relay.
    pub direct_addresses: Vec<String>,
    /// Application protocol negotiated over the QUIC tunnel.
    pub alpn: Option<String>,
}

impl IrohDiscoveryDetails {
    /// Returns `true` when the discovery snapshot does not expose any data.
    pub fn is_empty(&self) -> bool {
        self.relay.is_none() && self.direct_addresses.is_empty() && self.alpn.is_none()
    }
}

impl Default for IrohDiscoveryDetails {
    fn default() -> Self {
        Self {
            relay: None,
            direct_addresses: Vec::new(),
            alpn: None,
        }
    }
}

/// Resolve `_iroh` TXT records for the provided homeserver key.
///
/// Returns `Ok(Some(details))` when records are present, `Ok(None)` when the
/// packet does not publish any `_iroh` attributes, and `Err(_)` for transport
/// or parsing failures.
pub async fn resolve_iroh_records(
    homeserver: &PublicKey,
    use_testnet: bool,
) -> Result<Option<IrohDiscoveryDetails>> {
    let client = build_pkarr_client(use_testnet)?;
    let packet = client
        .resolve(homeserver)
        .await
        .with_context(|| format!("failed to resolve pkarr packet for {homeserver}"))?;

    let mut details = IrohDiscoveryDetails::default();

    for record in packet.all_resource_records() {
        if !record.name.to_string().contains("_iroh._udp") {
            continue;
        }

        if let RData::TXT(txt) = &record.rdata {
            let attrs = txt.clone().attributes();
            if let Some(Some(addr)) = attrs.get("addr") {
                details.direct_addresses.push(addr.clone());
            }
            if let Some(Some(relay)) = attrs.get("relay") {
                if details.relay.is_none() {
                    let url = Url::parse(relay)
                        .with_context(|| format!("invalid relay URL in _iroh record: {relay}"))?;
                    details.relay = Some(url);
                }
            }
            if let Some(Some(alpn)) = attrs.get("alpn") {
                details.alpn = Some(alpn.clone());
            }
        }
    }

    if details.is_empty() {
        Ok(None)
    } else {
        Ok(Some(details))
    }
}

/// Execute an HTTP request over an Iroh tunnel and return the formatted response string.
pub async fn request_over_iroh(
    homeserver: &PublicKey,
    method: &Method,
    url: &Url,
    headers: &[(ReqwestHeaderName, String)],
    body: &[u8],
    use_testnet: bool,
) -> Result<String> {
    let discovery = resolve_iroh_records(homeserver, use_testnet)
        .await?
        .ok_or_else(|| anyhow!("homeserver does not publish _iroh records"))?;

    let alpn = discovery
        .alpn
        .clone()
        .unwrap_or_else(|| "pubky/iroh-homeserver/0".to_string());
    let direct_addresses = parse_direct_addresses(&discovery.direct_addresses).await?;

    let node_id = iroh::NodeId::from_bytes(&homeserver.to_bytes())
        .map_err(|err| anyhow!("invalid homeserver key: {err}"))?;

    let relay_url = discovery
        .relay
        .clone()
        .map(|relay| iroh::RelayUrl::from(relay));

    let node_addr = NodeAddr::from_parts(node_id, relay_url.clone(), direct_addresses.clone());

    let mut rng = StdRng::from_entropy();
    let mut secret_seed = [0u8; 32];
    rng.fill_bytes(&mut secret_seed);

    let mut builder = Endpoint::builder()
        .secret_key(SecretKey::from_bytes(&secret_seed))
        .alpns(vec![alpn.clone().into_bytes()]);

    if let Some(relay) = relay_url.clone() {
        builder = builder.relay_mode(RelayMode::Custom(RelayMap::from(relay)));
    }

    let endpoint = builder
        .bind()
        .await
        .context("failed to bind Iroh endpoint")?;
    endpoint.online().await;

    let connection = endpoint
        .connect(node_addr, alpn.as_bytes())
        .await
        .context("failed to connect to homeserver via Iroh")?;

    let (send, recv) = connection
        .open_bi()
        .await
        .context("failed to open Iroh stream")?;

    let quic_stream = QuicStream::new(send, recv);
    let (mut sender, connection_driver) = http1::handshake(TokioIo::new(quic_stream))
        .await
        .context("HTTP handshake over Iroh failed")?;

    tokio::spawn(async move {
        if let Err(error) = connection_driver.await {
            debug!(?error, "Iroh HTTP driver terminated");
        }
    });

    let mut path = url.path().to_string();
    if path.is_empty() {
        path.push('/');
    }
    if let Some(query) = url.query() {
        path.push('?');
        path.push_str(query);
    }
    let http_method = http::Method::from_bytes(method.as_str().as_bytes())
        .with_context(|| format!("invalid HTTP method: {method}"))?;

    let mut request_builder = Request::builder()
        .method(http_method)
        .uri(path)
        .version(http::Version::HTTP_11);

    let mut host_header_present = false;
    let mut content_length_present = false;

    for (name, value) in headers {
        if name == &ReqwestHeaderName::from_static("host") {
            host_header_present = true;
        }
        if name == &ReqwestHeaderName::from_static("content-length") {
            content_length_present = true;
        }
        request_builder = request_builder.header(name.as_str(), value);
    }

    if !host_header_present {
        if let Some(host) = url.host_str() {
            let host_value = if let Some(port) = url.port() {
                format!("{host}:{port}")
            } else {
                host.to_string()
            };
            request_builder = request_builder.header("Host", host_value);
        }
    }

    if !content_length_present && !body.is_empty() {
        request_builder = request_builder.header("Content-Length", body.len().to_string());
    }

    let request = request_builder
        .body(Full::new(Bytes::copy_from_slice(body)))
        .context("failed to build HTTP request for Iroh tunnel")?;

    let response = sender
        .send_request(request)
        .await
        .context("Iroh tunnel request failed")?;

    let (parts, body_stream) = response.into_parts();
    let body_bytes = body_stream
        .collect()
        .await
        .context("failed to read response body over Iroh tunnel")?
        .to_bytes();

    connection.close(0u32.into(), b"done");
    endpoint.close().await;

    Ok(format_http_response(&parts, &body_bytes))
}

/// Render a human readable summary of the discovery data.
pub fn format_discovery_summary(details: &IrohDiscoveryDetails) -> String {
    let mut lines = Vec::new();
    if let Some(relay) = &details.relay {
        lines.push(format!("Relay: {relay}"));
    }
    if !details.direct_addresses.is_empty() {
        lines.push("Direct addresses:".to_string());
        for addr in &details.direct_addresses {
            lines.push(format!("  - {addr}"));
        }
    }
    if let Some(alpn) = &details.alpn {
        lines.push(format!("ALPN: {alpn}"));
    }
    if lines.is_empty() {
        "No _iroh TXT attributes published.".to_string()
    } else {
        lines.join("\n")
    }
}

fn build_pkarr_client(use_testnet: bool) -> Result<Client> {
    let mut builder = Client::builder();
    if use_testnet {
        builder.bootstrap(&[format!("localhost:{}", testnet_ports::BOOTSTRAP)]);
        builder
            .relays(&[format!("http://localhost:{}", testnet_ports::PKARR_RELAY)])
            .context("invalid testnet relay URL")?;
    }
    builder.build().context("failed to construct pkarr client")
}

async fn parse_direct_addresses(addresses: &[String]) -> Result<Vec<SocketAddr>> {
    let mut results = Vec::new();
    for value in addresses {
        let url = Url::parse(value)
            .with_context(|| format!("invalid addr attribute in _iroh record: {value}"))?;
        if url.scheme() != "quic" {
            return Err(anyhow!("_iroh addr entry must use quic:// scheme: {value}"));
        }
        let port = url
            .port()
            .ok_or_else(|| anyhow!("missing port in _iroh addr value: {value}"))?;
        match url.host() {
            Some(Host::Ipv4(addr)) => results.push(SocketAddr::new(addr.into(), port)),
            Some(Host::Ipv6(addr)) => results.push(SocketAddr::new(addr.into(), port)),
            Some(Host::Domain(domain)) => {
                for resolved in lookup_host((domain, port)).await? {
                    results.push(resolved);
                }
            }
            None => return Err(anyhow!("missing host in _iroh addr value: {value}")),
        }
    }
    results.sort();
    results.dedup();
    Ok(results)
}

fn format_http_response(parts: &http::response::Parts, body: &[u8]) -> String {
    let mut headers = Vec::new();
    for (name, value) in &parts.headers {
        if let Ok(text) = value.to_str() {
            headers.push(format!("{}: {}", name, text));
        }
    }

    let body_text = match String::from_utf8(body.to_vec()) {
        Ok(text) => text,
        Err(_) => format!("<binary {} bytes>", body.len()),
    };

    format!(
        "{version:?} {status}\n{}\n\n{body}",
        headers.join("\n"),
        version = parts.version,
        status = parts.status,
        body = body_text
    )
}

/// Attempt to extract a homeserver public key from the provided URL.
pub fn parse_homeserver_key(url: &Url) -> Option<PublicKey> {
    match url.scheme() {
        "pubky" => {
            if let Some(host) = url.host_str() {
                PublicKey::try_from(host).ok()
            } else {
                url.path()
                    .trim_start_matches('/')
                    .split('/')
                    .next()
                    .and_then(|segment| PublicKey::try_from(segment).ok())
            }
        }
        "https" | "http" => url.host_str().and_then(|host| {
            let candidate = host.strip_suffix(".pubky").unwrap_or(host);
            PublicKey::try_from(candidate).ok()
        }),
        _ => None,
    }
}

pin_project! {
    struct QuicStream {
        #[pin]
        send: iroh::endpoint::SendStream,
        #[pin]
        recv: iroh::endpoint::RecvStream,
    }
}

impl QuicStream {
    fn new(send: iroh::endpoint::SendStream, recv: iroh::endpoint::RecvStream) -> Self {
        Self { send, recv }
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().recv.poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project()
            .send
            .poll_write(cx, buf)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(
            self.project()
                .send
                .get_mut()
                .finish()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Response;
    use std::collections::HashSet;
    use std::net::SocketAddr;

    #[test]
    fn format_summary_handles_all_fields() {
        let details = IrohDiscoveryDetails {
            relay: Some(Url::parse("https://relay.example").unwrap()),
            direct_addresses: vec![
                "quic://10.0.0.1:4444".into(),
                "quic://[2001:db8::1]:4444".into(),
            ],
            alpn: Some("pubky/iroh-homeserver/0".into()),
        };
        let summary = format_discovery_summary(&details);
        assert!(summary.contains("Relay: https://relay.example"));
        assert!(summary.contains("quic://10.0.0.1:4444"));
        assert!(summary.contains("ALPN: pubky/iroh-homeserver/0"));
    }

    #[test]
    fn empty_details_render_placeholder() {
        let summary = format_discovery_summary(&IrohDiscoveryDetails::default());
        assert_eq!(summary, "No _iroh TXT attributes published.");
    }

    #[tokio::test]
    async fn parse_direct_addresses_supports_ipv4_ipv6_and_dns() {
        let entries = vec![
            "quic://127.0.0.1:4444".to_string(),
            "quic://[::1]:4445".to_string(),
            "quic://localhost:4446".to_string(),
            "quic://localhost:4446".to_string(),
        ];

        let parsed = parse_direct_addresses(&entries)
            .await
            .expect("parsing succeeds");

        let unique: HashSet<SocketAddr> = parsed.iter().copied().collect();
        assert_eq!(unique.len(), parsed.len(), "duplicates are removed");

        assert!(
            parsed
                .iter()
                .any(|addr| addr.ip().is_ipv4() && addr.port() == 4444)
        );
        assert!(
            parsed
                .iter()
                .any(|addr| addr.ip().is_ipv6() && addr.port() == 4445)
        );
        assert!(parsed.iter().any(|addr| addr.port() == 4446));
    }

    #[tokio::test]
    async fn parse_direct_addresses_rejects_non_quic_scheme() {
        let err = parse_direct_addresses(&["http://127.0.0.1:8080".to_string()])
            .await
            .expect_err("non-quic schemes fail");

        assert!(
            err.to_string()
                .contains("_iroh addr entry must use quic:// scheme")
        );
    }

    #[test]
    fn parse_homeserver_key_from_pubky_scheme() {
        let keypair = pkarr::Keypair::random();
        let key_text = keypair.public_key().to_string();
        let url = Url::parse(&format!("pubky://{key_text}/feed")).unwrap();

        let parsed = parse_homeserver_key(&url).expect("key is extracted");
        assert_eq!(parsed, keypair.public_key());
    }

    #[test]
    fn parse_homeserver_key_from_https_subdomain() {
        let keypair = pkarr::Keypair::random();
        let key_text = keypair.public_key().to_string();
        let url = Url::parse(&format!("https://{key_text}.pubky/api")).unwrap();

        let parsed = parse_homeserver_key(&url).expect("key is extracted");
        assert_eq!(parsed, keypair.public_key());
    }

    #[test]
    fn format_http_response_renders_headers_and_body() {
        let response = Response::builder()
            .status(200)
            .version(http::Version::HTTP_11)
            .header("Content-Type", "text/plain")
            .body(())
            .unwrap();
        let (parts, _) = response.into_parts();

        let formatted = format_http_response(&parts, b"hello world");

        assert!(formatted.contains("HTTP/1.1 200 OK"));
        assert!(formatted.contains("content-type: text/plain"));
        assert!(formatted.ends_with("hello world"));
    }

    #[test]
    fn format_http_response_notes_binary_bodies() {
        let response = Response::builder()
            .status(200)
            .version(http::Version::HTTP_11)
            .body(())
            .unwrap();
        let (parts, _) = response.into_parts();

        let formatted = format_http_response(&parts, &[0, 159, 146]);

        assert!(formatted.contains("<binary 3 bytes>"));
    }
}
