//! Embedded Iroh endpoint that bridges QUIC streams into the homeserver's HTTP listener.

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    app_context::AppContext,
    data_directory::IrohToml,
    discovery::iroh_records::IrohDiscoverySnapshot,
};
use iroh::{Endpoint, RelayMap, RelayMode, SecretKey};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use url::Url;

const ONLINE_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Errors that can occur when starting the embedded gateway.
#[derive(Debug, thiserror::Error)]
pub enum IrohGatewayError {
    /// Failed to bind the Iroh endpoint with the configured secret key and relays.
    #[error("failed to bind iroh endpoint: {0}")]
    Bind(#[from] iroh::endpoint::BindError),
}

/// Drives the embedded Iroh endpoint and keeps the accept loop alive.
pub struct IrohGateway {
    inner: Arc<IrohGatewayInner>,
    accept_task: JoinHandle<()>,
}

struct IrohGatewayInner {
    endpoint: Endpoint,
    cancel: CancellationToken,
}

impl IrohGateway {
    /// Starts the Iroh endpoint and begins accepting incoming connections.
    pub async fn start(
        context: &AppContext,
        forward_socket: SocketAddr,
    ) -> Result<Self, IrohGatewayError> {
        let secret_key = SecretKey::from_bytes(&context.keypair.secret_key());
        let alpn = context.config_toml.discovery.iroh.alpn.clone();

        let mut builder = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![alpn.into_bytes()]);

        if let Some(relay_url) = &context.config_toml.discovery.iroh.relay_url {
            let relay = iroh::RelayUrl::from(relay_url.clone());
            builder = builder.relay_mode(RelayMode::Custom(RelayMap::from(relay)));
        }

        let endpoint = builder.bind().await?;

        let cancel = CancellationToken::new();
        let accept_token = cancel.clone();
        let endpoint_for_task = endpoint.clone();
        let accept_task = tokio::spawn(async move {
            run_accept_loop(endpoint_for_task, forward_socket, accept_token).await;
        });

        let online_endpoint = endpoint.clone();
        tokio::spawn(async move {
            if let Err(err) = tokio::time::timeout(ONLINE_WAIT_TIMEOUT, online_endpoint.online()).await {
                warn!(?err, "Iroh endpoint did not report online within timeout");
            }
        });

        let inner = Arc::new(IrohGatewayInner { endpoint, cancel });

        Ok(Self { inner, accept_task })
    }

    /// Returns a lightweight handle for discovery snapshots.
    pub fn handle(&self) -> IrohGatewayHandle {
        IrohGatewayHandle {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Drop for IrohGateway {
    fn drop(&mut self) {
        self.inner.cancel.cancel();
        self.accept_task.abort();
        let endpoint = self.inner.endpoint.clone();
        tokio::spawn(async move {
            endpoint.close().await;
        });
    }
}

/// Cloneable handle that exposes discovery snapshots without owning the gateway runtime.
#[derive(Clone)]
pub struct IrohGatewayHandle {
    inner: Arc<IrohGatewayInner>,
}

impl IrohGatewayHandle {
    /// Build the `_iroh` discovery snapshot that should be published alongside HTTPS metadata.
    pub async fn discovery_snapshot(&self, config: &IrohToml) -> Option<IrohDiscoverySnapshot> {
        if let Err(err) = tokio::time::timeout(ONLINE_WAIT_TIMEOUT, self.inner.endpoint.online()).await {
            warn!(?err, "Iroh endpoint online check timed out");
        }

        let node_addr = self.inner.endpoint.node_addr();
        let relay_url = node_addr
            .relay_url()
            .cloned()
            .map(|relay| Url::from(relay));

        let mut direct_addresses: Vec<String> = node_addr
            .direct_addresses()
            .map(|addr| format!("quic://{addr}"))
            .collect();
        direct_addresses.extend(config.direct_addresses.clone());
        direct_addresses.sort();
        direct_addresses.dedup();

        let relay_url = relay_url.or_else(|| config.relay_url.clone());

        if relay_url.is_none() && direct_addresses.is_empty() && config.alpn.is_empty() {
            return None;
        }

        Some(IrohDiscoverySnapshot {
            relay_url,
            direct_addresses,
            alpn: Some(config.alpn.clone()),
            txt_ttl: config.txt_ttl_seconds,
            publish_interval: Duration::from_secs(
                config.discovery_interval_seconds(),
            ),
        })
    }
}

impl IrohToml {
    /// Helper to convert the publish cadence to seconds.
    fn discovery_interval_seconds(&self) -> u64 {
        self.publish_interval_minutes.get() * 60
    }
}

async fn run_accept_loop(endpoint: Endpoint, forward_socket: SocketAddr, cancel: CancellationToken) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("Stopping Iroh gateway accept loop");
                break;
            }
            incoming = endpoint.accept() => {
                match incoming {
                    Some(incoming) => match incoming.accept() {
                        Ok(connecting) => {
                            tokio::spawn(handle_connecting(connecting, forward_socket));
                        }
                        Err(err) => {
                            warn!(?err, "Failed to accept incoming Iroh handshake");
                        }
                    },
                    None => break,
                }
            }
        }
    }
}

async fn handle_connecting(connecting: iroh::endpoint::Connecting, forward_socket: SocketAddr) {
    match connecting.await {
        Ok(connection) => {
            let peer = connection
                .remote_node_id()
                .map(|id| format!("{}", id.fmt_short()))
                .unwrap_or_else(|_| "unknown".to_string());
            info!("Accepted Iroh connection from {peer}");
            loop {
                match connection.accept_bi().await {
                    Ok((send, recv)) => {
                        tokio::spawn(bridge_stream(send, recv, forward_socket));
                    }
                    Err(err) => {
                        debug!(?err, "Iroh connection closed");
                        break;
                    }
                }
            }
        }
        Err(err) => warn!(?err, "Failed to finalize Iroh handshake"),
    }
}

async fn bridge_stream(
    send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
    forward_socket: SocketAddr,
) {
    match TcpStream::connect(forward_socket).await {
        Ok(stream) => {
            if let Err(err) = pump_streams(stream, send, recv).await {
                debug!(?err, "Iroh stream bridge terminated");
            }
        }
        Err(err) => warn!(?err, "Failed to connect to local HTTP listener"),
    }
}

async fn pump_streams(
    stream: TcpStream,
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
) -> io::Result<()> {
    let (mut tcp_reader, mut tcp_writer) = stream.into_split();

    let client_to_server = tokio::spawn(async move {
        let result = tokio::io::copy(&mut recv, &mut tcp_writer).await;
        let _ = tcp_writer.shutdown().await;
        result
    });

    let server_to_client = tokio::spawn(async move {
        let result = tokio::io::copy(&mut tcp_reader, &mut send).await;
        let _ = send.finish();
        result
    });

    let (a, b) = tokio::join!(client_to_server, server_to_client);

    a.map_err(|err| io::Error::new(io::ErrorKind::Other, err))??;
    b.map_err(|err| io::Error::new(io::ErrorKind::Other, err))??;

    Ok(())
}
