//! Helpers for producing `_iroh` TXT resource records.

use std::time::Duration;

use pkarr::{dns::{CharacterString, Name, ResourceRecord, SimpleDnsError, CLASS}, PublicKey};
use pkarr::dns::rdata::{RData, TXT};
use thiserror::Error;
use url::Url;

/// Snapshot of the information that should be exposed via `_iroh` TXT records.
#[derive(Clone, Debug, PartialEq)]
pub struct IrohDiscoverySnapshot {
    /// Optional relay URL published by the gateway.
    pub relay_url: Option<Url>,
    /// Direct socket addresses that can be dialled without a relay.
    pub direct_addresses: Vec<String>,
    /// Application protocol negotiated over the QUIC tunnel.
    pub alpn: Option<String>,
    /// Time-to-live to apply to the generated records.
    pub txt_ttl: u32,
    /// Interval at which the records should be republished.
    pub publish_interval: Duration,
}

impl IrohDiscoverySnapshot {
    /// Returns `true` when the snapshot contains any discoverable data.
    pub fn is_empty(&self) -> bool {
        self.relay_url.is_none()
            && self.direct_addresses.is_empty()
            && self.alpn.as_ref().map(String::as_str) == Some("")
    }
}

/// Result of building `_iroh` TXT records for publication.
#[derive(Clone, Debug, PartialEq)]
pub struct IrohPublication {
    /// Concrete resource records ready to be merged with the homeserver packet.
    pub records: Vec<ResourceRecord<'static>>,
    /// Republish cadence recommended for the records.
    pub publish_interval: Duration,
}

/// Errors that can occur while constructing `_iroh` TXT records.
#[derive(Debug, Error)]
pub enum IrohRecordError {
    /// The owner name derived from the homeserver keypair was invalid.
    #[error("invalid _iroh owner name: {0}")]
    InvalidOwner(#[from] SimpleDnsError),
    /// An attribute failed validation when encoded as a TXT character string.
    #[error("invalid {attribute} value: {source}")]
    InvalidAttribute {
        /// Attribute that failed validation.
        attribute: &'static str,
        /// Underlying error raised by the TXT encoder.
        source: SimpleDnsError,
    },
}

/// Build the `_iroh` TXT records for the provided snapshot.
pub fn build_publication(
    owner: &PublicKey,
    snapshot: &IrohDiscoverySnapshot,
) -> Result<IrohPublication, IrohRecordError> {
    if snapshot.txt_ttl == 0 {
        return Ok(IrohPublication {
            records: Vec::new(),
            publish_interval: snapshot.publish_interval,
        });
    }

    let owner_name_str = format!("_iroh._udp.{}.", owner);
    let owner_name = Name::new(&owner_name_str)?.into_owned();

    let mut records = Vec::new();

    if let Some(relay) = &snapshot.relay_url {
        let mut txt = TXT::new();
        let relay_value = format!("relay={relay}");
        let relay_cs = CharacterString::new(relay_value.as_bytes())
            .map_err(|source| IrohRecordError::InvalidAttribute {
                attribute: "relay",
                source,
            })?
            .into_owned();
        txt.add_char_string(relay_cs);
        records.push(ResourceRecord::new(owner_name.clone(), CLASS::IN, snapshot.txt_ttl, RData::TXT(txt)));
    }

    for addr in &snapshot.direct_addresses {
        let mut txt = TXT::new();
        let addr_value = format!("addr={addr}");
        let addr_cs = CharacterString::new(addr_value.as_bytes())
            .map_err(|source| IrohRecordError::InvalidAttribute {
                attribute: "addr",
                source,
            })?
            .into_owned();
        txt.add_char_string(addr_cs);
        records.push(ResourceRecord::new(owner_name.clone(), CLASS::IN, snapshot.txt_ttl, RData::TXT(txt)));
    }

    if let Some(alpn) = snapshot.alpn.as_ref().filter(|value| !value.is_empty()) {
        let mut txt = TXT::new();
        let alpn_value = format!("alpn={alpn}");
        let alpn_cs = CharacterString::new(alpn_value.as_bytes())
            .map_err(|source| IrohRecordError::InvalidAttribute {
                attribute: "alpn",
                source,
            })?
            .into_owned();
        txt.add_char_string(alpn_cs);
        records.push(ResourceRecord::new(owner_name, CLASS::IN, snapshot.txt_ttl, RData::TXT(txt)));
    }

    Ok(IrohPublication {
        records,
        publish_interval: snapshot.publish_interval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkarr::Keypair;
    use std::time::Duration;

    #[test]
    fn build_publication_emits_records_for_all_attributes() {
        let owner = Keypair::random().public_key();
        let snapshot = IrohDiscoverySnapshot {
            relay_url: Some("https://relay.example".parse().unwrap()),
            direct_addresses: vec!["quic://10.1.1.4:4444".into(), "quic://[2001:db8::1]:4444".into()],
            alpn: Some("pubky/iroh-homeserver/0".into()),
            txt_ttl: 300,
            publish_interval: Duration::from_secs(60),
        };

        let publication = build_publication(&owner, &snapshot).expect("building records succeeds");
        assert_eq!(publication.records.len(), 3);
        assert_eq!(publication.publish_interval, Duration::from_secs(60));

        let strings: Vec<String> = publication
            .records
            .iter()
            .map(|record| match record.rdata() {
                RData::TXT(txt) => txt.clone().try_into().unwrap(),
                other => panic!("unexpected record: {other:?}"),
            })
            .collect();

        assert!(strings.iter().any(|value| value.contains("relay=https://relay.example")));
        assert!(strings.iter().any(|value| value.contains("addr=quic://10.1.1.4:4444")));
        assert!(strings.iter().any(|value| value.contains("addr=quic://[2001:db8::1]:4444")));
        assert!(strings.iter().any(|value| value.contains("alpn=pubky/iroh-homeserver/0")));
    }
}
