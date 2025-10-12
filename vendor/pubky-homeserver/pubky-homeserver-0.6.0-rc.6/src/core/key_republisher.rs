//! Background task to republish the homeserver's pkarr packet to the DHT.
//!
//! This task is started by the [crate::HomeserverCore] and runs until the homeserver is stopped.
//!
//! The task is responsible for:
//! - Republishing the homeserver's pkarr packet to the DHT every hour.
//! - Stopping the task when the homeserver is stopped.

use std::net::IpAddr;

use anyhow::Result;
use pkarr::dns::{Name, ResourceRecord};
use pkarr::errors::PublishError;
use pkarr::{dns::rdata::SVCB, SignedPacket};

use crate::app_context::AppContext;
use crate::discovery::iroh_records::{build_publication, IrohDiscoverySnapshot};
use crate::iroh::gateway::IrohGatewayHandle;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

/// Republishes the homeserver's pkarr packet to the DHT every hour.
pub struct HomeserverKeyRepublisher {
    join_handle: JoinHandle<()>,
}

impl HomeserverKeyRepublisher {
    pub async fn start(
        context: &AppContext,
        icann_http_port: u16,
        pubky_tls_port: u16,
        iroh_gateway: Option<IrohGatewayHandle>,
    ) -> Result<Self> {
        let base_interval = Duration::from_secs(
            context
                .config_toml
                .discovery
                .pkarr_publish_interval_minutes
                .get()
                * 60,
        );

        let (extra_records, publish_interval) = if context.config_toml.discovery.iroh.enabled {
            let config_snapshot = || IrohDiscoverySnapshot {
                relay_url: context.config_toml.discovery.iroh.relay_url.clone(),
                direct_addresses: context
                    .config_toml
                    .discovery
                    .iroh
                    .direct_addresses
                    .clone(),
                alpn: Some(context.config_toml.discovery.iroh.alpn.clone()),
                txt_ttl: context.config_toml.discovery.iroh.txt_ttl_seconds,
                publish_interval: Duration::from_secs(
                    context
                        .config_toml
                        .discovery
                        .iroh
                        .publish_interval_minutes
                        .get()
                        * 60,
                ),
            };

            let snapshot = match iroh_gateway {
                Some(handle) => match handle
                    .discovery_snapshot(&context.config_toml.discovery.iroh)
                    .await
                {
                    Some(snapshot) => snapshot,
                    None => {
                        tracing::warn!(
                            "Iroh gateway did not expose discovery data; falling back to static configuration"
                        );
                        config_snapshot()
                    }
                },
                None => config_snapshot(),
            };

            match build_publication(&context.keypair.public_key(), &snapshot) {
                Ok(publication) if !publication.records.is_empty() => {
                    tracing::info!("Attaching {} _iroh TXT records", publication.records.len());
                    (
                        publication.records,
                        publication.publish_interval.min(base_interval),
                    )
                }
                Ok(_) => (Vec::new(), base_interval),
                Err(error) => {
                    tracing::warn!(?error, "failed to build _iroh TXT records");
                    (Vec::new(), base_interval)
                }
            }
        } else {
            (Vec::new(), base_interval)
        };

        let signed_packet = create_signed_packet(
            context,
            icann_http_port,
            pubky_tls_port,
            &extra_records,
        )?;
        let join_handle = Self::start_periodic_republish(
            context.pkarr_client.clone(),
            &signed_packet,
            publish_interval,
        )
        .await?;
        Ok(Self { join_handle })
    }

    async fn publish_once(
        client: &pkarr::Client,
        signed_packet: &SignedPacket,
    ) -> Result<(), PublishError> {
        let res = client.publish(signed_packet, None).await;
        if let Err(e) = &res {
            tracing::warn!(
                "Failed to publish the homeserver's pkarr packet to the DHT: {}",
                e
            );
        } else {
            tracing::info!("Published the homeserver's pkarr packet to the DHT.");
        }
        res
    }

    /// Start the periodic republish task which will republish the server packet to the DHT every hour.
    ///
    /// # Errors
    /// - Throws an error if the initial publish fails.
    /// - Throws an error if the periodic republish task is already running.
    async fn start_periodic_republish(
        client: pkarr::Client,
        signed_packet: &SignedPacket,
        publish_interval: Duration,
    ) -> anyhow::Result<JoinHandle<()>> {
        // Publish once to make sure the packet is published to the DHT before this
        // function returns.
        // Throws an error if the packet is not published to the DHT.
        Self::publish_once(&client, signed_packet).await?;

        // Start the periodic republish task.
        let signed_packet = signed_packet.clone();
        let handle = tokio::spawn(async move {
            let mut interval = interval(publish_interval);
            interval.tick().await; // This ticks immediatly. Wait for first interval before starting the loop.
            loop {
                interval.tick().await;
                let _ = Self::publish_once(&client, &signed_packet).await;
            }
        });

        Ok(handle)
    }

    /// Stop the periodic republish task.
    pub fn stop(&self) {
        self.join_handle.abort();
    }
}

impl Drop for HomeserverKeyRepublisher {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn create_signed_packet(
    context: &AppContext,
    local_icann_http_port: u16,
    local_pubky_tls_port: u16,
    extra_records: &[ResourceRecord<'static>],
) -> Result<SignedPacket> {
    let root_name: Name = "."
        .try_into()
        .expect(". is the root domain and always valid");

    let mut signed_packet_builder = SignedPacket::builder();

    let public_ip = context.config_toml.pkdns.public_ip;
    let public_pubky_tls_port = context
        .config_toml
        .pkdns
        .public_pubky_tls_port
        .unwrap_or(local_pubky_tls_port);
    let public_icann_http_port = context
        .config_toml
        .pkdns
        .public_icann_http_port
        .unwrap_or(local_icann_http_port);

    // `SVCB(HTTPS)` record pointing to the pubky tls port and the public ip address
    // This is what is used in all applications expect for browsers.
    let mut svcb = SVCB::new(1, root_name.clone());
    svcb.set_port(public_pubky_tls_port);
    match &public_ip {
        IpAddr::V4(ip) => {
            svcb.set_ipv4hint([ip.to_bits()])?;
        }
        IpAddr::V6(ip) => {
            svcb.set_ipv6hint([ip.to_bits()])?;
        }
    };
    signed_packet_builder = signed_packet_builder.https(root_name.clone(), svcb, 60 * 60);

    // `SVCB` record pointing to the icann http port and the ICANN domain for browsers support.
    // Low priority to not override the `SVCB(HTTPS)` record.
    // Why are we doing this?
    // The pubky-client in the browser can only do regular HTTP(s) requests.
    // Pubky TLS requests are therefore not possible. Therefore, we need to fallback to the ICANN domain./
    //
    // TODO: Is it possible to point the SVCB record to the IP address via a `A` record?
    // This would remove the ICANN domain dependency.
    if let Some(domain) = &context.config_toml.pkdns.icann_domain {
        let mut svcb = SVCB::new(10, root_name.clone());

        let http_port_be_bytes = public_icann_http_port.to_be_bytes();
        if domain.0 == "localhost" {
            svcb.set_param(
                pubky_common::constants::reserved_param_keys::HTTP_PORT,
                &http_port_be_bytes,
            )?;
        }
        svcb.target = domain.0.as_str().try_into()?;
        signed_packet_builder = signed_packet_builder.https(root_name.clone(), svcb, 60 * 60);
    }

    // `A` record to the public IP. This is used for regular browser connections.
    signed_packet_builder = signed_packet_builder.address(root_name.clone(), public_ip, 60 * 60);

    for record in extra_records {
        signed_packet_builder = signed_packet_builder.record(record.clone());
    }

    Ok(signed_packet_builder.build(&context.keypair)?)
}

#[cfg(test)]
mod tests {
    use futures_lite::StreamExt;
    use pkarr::extra::endpoints::Endpoint;
    use std::net::{Ipv4Addr, SocketAddr};
    use std::num::NonZeroU64;
    use url::Url;

    use super::*;

    #[tokio::test]
    async fn test_resolve_https_endpoint_with_pkarr_client() {
        let context = AppContext::test();
        let _republisher = HomeserverKeyRepublisher::start(&context, 8080, 8080, None)
            .await
            .unwrap();
        let pkarr_client = context.pkarr_client.clone();
        let hs_pubky = context.keypair.public_key();
        // Make sure the pkarr packet of the hs is resolvable.
        let _packet = pkarr_client.resolve(&hs_pubky).await.unwrap();
        // Make sure the pkarr client can resolve the endpoint of the hs.
        let qname = format!("{}", hs_pubky);
        let endpoint = pkarr_client
            .resolve_https_endpoint(qname.as_str())
            .await
            .unwrap();
        assert_eq!(
            endpoint.to_socket_addrs().first().unwrap().clone(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
        );
    }

    #[tokio::test]
    async fn test_endpoints() {
        let mut context = AppContext::test();
        context.keypair = pkarr::Keypair::random();
        let _republisher = HomeserverKeyRepublisher::start(&context, 8080, 8080, None)
            .await
            .unwrap();
        let pubkey = context.keypair.public_key();

        let client = pkarr::Client::builder().build().unwrap();
        let packet = client.resolve(&pubkey).await.unwrap();
        let rr: Vec<&pkarr::dns::ResourceRecord> = packet.all_resource_records().collect();
        assert_eq!(rr.len(), 3);

        let endpoints: Vec<Endpoint> = client
            .resolve_https_endpoints(&pubkey.to_z32())
            .collect()
            .await;
        assert_eq!(endpoints.len(), 2);

        //SignedPacket
        //{
        // ResourceRecord {
        // name: Name("8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty", "54"),
        // class: IN,
        // ttl: 3600,
        // rdata: A(A { address: 574725291 }),
        // cache_flush: false },
        //
        // ResourceRecord {
        // name: Name("8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty", "54"),
        // class: IN,
        // ttl: 3600,
        // rdata: HTTPS(HTTPS(SVCB {
        // priority: 0,
        // target: Name("", "1"),
        // params: {3: [24, 143]} })),
        // cache_flush: false },
        //
        // ResourceRecord {
        // name: Name("8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty", "54"),
        // class: IN,
        // ttl: 3600,
        // rdata: HTTPS(HTTPS(SVCB {
        // priority: 10,
        // target: Name("homeserver.pubky.app", "22"), params: {} })),
        // cache_flush: false }],
        //
        //[
        // Endpoint {
        // target: ".",
        // public_key: PublicKey(8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty),
        // port: 6287,
        // addrs: [34.65.156.171],
        // params: {3: [24, 143]} },
        //
        // Endpoint {
        // target: "homeserver.pubky.app",
        // public_key: PublicKey(8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty),
        // port: 0,
        // addrs: [],
        // params: {} }]
    }

    #[tokio::test]
    async fn test_iroh_records_are_published_when_enabled() {
        let mut context = AppContext::test();
        context.config_toml.discovery.iroh.enabled = true;
        context.config_toml.discovery.iroh.direct_addresses =
            vec!["quic://127.0.0.1:4444".to_string()];
        context.config_toml.discovery.iroh.relay_url =
            Some(Url::parse("https://relay.test").unwrap());
        context.config_toml.discovery.iroh.txt_ttl_seconds = 180;
        context.config_toml.discovery.iroh.publish_interval_minutes =
            NonZeroU64::new(5).unwrap();

        let _republisher = HomeserverKeyRepublisher::start(&context, 8080, 8080, None)
            .await
            .unwrap();

        let client = pkarr::Client::builder().build().unwrap();
        let packet = client
            .resolve(&context.keypair.public_key())
            .await
            .expect("packet available");

        let records: Vec<_> = packet.all_resource_records().collect();
        assert!(
            records
                .iter()
                .any(|record| record.name().to_string().contains("_iroh._udp")),
            "expected _iroh TXT records"
        );
    }
}
