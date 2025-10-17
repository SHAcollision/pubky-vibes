use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::{Context, Result};
use pkarr::PublicKey;
use pubky::PubkyHttpClient;
use reqwest::dns::{Addrs, Name, Resolve};

use crate::app::NetworkMode;

/// Build a [`PubkyHttpClient`] for the requested [`NetworkMode`], patching the
/// resolver so pkarr hosts prefer IPv4 endpoints and default to port 443 when
/// missing.
pub fn build_pubky_http_client(mode: NetworkMode) -> Result<PubkyHttpClient> {
    let client = match mode {
        NetworkMode::Mainnet => PubkyHttpClient::new(),
        NetworkMode::Testnet => PubkyHttpClient::testnet(),
    }
    .context("failed to construct Pubky HTTP client")?;

    prefer_ipv4(client)
}

#[cfg(not(target_arch = "wasm32"))]
fn prefer_ipv4(client: PubkyHttpClient) -> Result<PubkyHttpClient> {
    use std::ptr;

    #[repr(C)]
    struct NativeFields {
        http: reqwest::Client,
        pkarr: pkarr::Client,
        icann_http: reqwest::Client,
    }

    let mut client = client;
    let fields_ptr = &mut client as *mut PubkyHttpClient as *mut NativeFields;
    let fields = unsafe { ptr::read(fields_ptr) };

    let patched_http = build_ipv4_http_client(&fields.pkarr, &fields.http)?;
    let new_fields = NativeFields {
        http: patched_http,
        pkarr: fields.pkarr,
        icann_http: fields.icann_http,
    };

    unsafe { ptr::write(fields_ptr, new_fields) };

    Ok(client)
}

#[cfg(target_arch = "wasm32")]
fn prefer_ipv4(client: PubkyHttpClient) -> Result<PubkyHttpClient> {
    Ok(client)
}

#[cfg(not(target_arch = "wasm32"))]
fn build_ipv4_http_client(
    pkarr: &pkarr::Client,
    _template: &reqwest::Client,
) -> Result<reqwest::Client> {
    let resolver = Arc::new(Ipv4PreferredResolver::new(pkarr.clone()));
    reqwest::ClientBuilder::from(pkarr.clone())
        .dns_resolver(resolver)
        .build()
        .context("failed to build IPv4-preferred reqwest client")
}

#[cfg(not(target_arch = "wasm32"))]
struct Ipv4PreferredResolver {
    pkarr: pkarr::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl Ipv4PreferredResolver {
    fn new(pkarr: pkarr::Client) -> Self {
        Self { pkarr }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Resolve for Ipv4PreferredResolver {
    fn resolve(&self, name: Name) -> reqwest::dns::Resolving {
        let pkarr = self.pkarr.clone();
        let host = name.as_str().to_owned();

        Box::pin(async move {
            if PublicKey::try_from(host.as_str()).is_ok() {
                let endpoint = pkarr
                    .resolve_https_endpoint(host.as_str())
                    .await
                    .map_err(|_| {
                        Box::new(PkarrResolutionFailed) as Box<dyn std::error::Error + Send + Sync>
                    })?;

                let mut addrs = endpoint.to_socket_addrs();
                normalize_port(&mut addrs);
                addrs.sort_by_key(|addr| if addr.ip().is_ipv4() { 0 } else { 1 });

                Ok(Box::new(addrs.into_iter()) as Addrs)
            } else {
                let addrs = format!("{host}:0")
                    .to_socket_addrs()
                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(Box::new(addrs) as Addrs)
            }
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_port(addrs: &mut [SocketAddr]) {
    for addr in addrs {
        if addr.port() == 0 {
            *addr = SocketAddr::new(addr.ip(), 443);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct PkarrResolutionFailed;

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for PkarrResolutionFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pkarr endpoint resolution failed")
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for PkarrResolutionFailed {}
