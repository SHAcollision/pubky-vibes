# Milestone 2 – Unified Pkarr Publication

## Goal

Publish a single Pkarr `SignedPacket` that merges the existing Pubky service
records with the `_iroh` TXT entries emitted by the embedded Iroh endpoint.
This allows clients to obtain both the classical HTTPS/TLS coordinates and the
Iroh relay data from one lookup, removing signature races between publishers.

## Baseline

- `pubky-homeserver` exposes a `HomeserverKeyRepublisher` task that rebuilds and
  republishes the `SignedPacket` every hour using the homeserver keypair.
- The embedded Iroh gateway (Milestone 1) already runs an `Endpoint` with the
  same identity keypair and can report its current `NodeInfo`.
- `pubky-core` republishes user keys and homeserver SVCB/A records separately
  but lacks hooks to append arbitrary resource records.

## Design Overview

1. **Collect fresh `NodeInfo`** – expose a `GatewayDiscoverySnapshot` helper that
   reads the current relay mode, direct addresses, and ALPN tuple from the Iroh
   endpoint. The helper serialises the data as
   `iroh_relay::node_info::NodeInfo<SignedNodePublic>`.
2. **Augment the packet builder** – extend
   `pubky_homeserver::core::key_republisher::create_signed_packet` so callers can
   supply additional `ResourceRecord`s prior to signing. The function keeps
   existing behaviour when no extra records are provided.
3. **Merge `_iroh` TXT entries** – implement a `to_pkarr_records()` adapter that
   converts the `NodeInfo` snapshot into TXT `ResourceRecord`s using
   `NodeInfo::to_pkarr_resource_records`. The adapter will drop direct addresses
   when a relay is configured, mirroring Iroh’s publisher semantics.
4. **Single publication pipeline** – the hourly republisher becomes the sole
   owner of the signed packet. The embedded Iroh publisher is disabled to avoid
   conflicting signatures; instead, the homeserver republishes on a 5 minute
   cadence when `_iroh` data is available.
5. **Configuration plumbing** – reuse the existing `config.toml` settings from
   Milestone 1 to toggle relay usage and enable `_iroh` publication. Add a
   `discovery.pkarr_extra_publish_interval_minutes` knob when operators want to
   keep the five-minute cadence even without Iroh data (optional).

## Implementation Steps

1. **Surface discovery state**
   - Add a `pubky_homeserver::iroh::GatewayHandle::discovery_snapshot()` async
     method returning an `Option<NodeInfo>`.
   - Expose the helper through `HomeserverSuite::iroh_gateway()` so the
     republisher task can call it.
2. **Extend the packet builder API**
   - Change `create_signed_packet` to accept `extra_records: &[ResourceRecord]`.
   - Update all call sites (including tests) to pass `&[]` when no extras exist.
3. **Generate `_iroh` records**
   - Introduce `pubky_homeserver::discovery::iroh_records` module housing
     `fn build_iroh_records(snapshot: &NodeInfo, ttl: u32) -> Vec<ResourceRecord>`.
   - Ensure TXT names follow the `_iroh._udp.<pubkey>.pkarr.net.` convention
     already used by Iroh.
4. **Wire into republisher**
   - Modify `HomeserverKeyRepublisher::publish_once` to collect the snapshot,
     build records, append them to the packet builder, and publish.
   - Fall back to the legacy behaviour when the snapshot is `None` (e.g. the
     gateway is disabled).
   - Adjust the periodic schedule to run every five minutes when `_iroh` records
     are present; otherwise, keep the existing hourly cadence.
5. **Disable Iroh’s standalone publisher**
   - Behind the `portable_homeserver_iroh` feature flag, skip constructing the
     `PkarrPublisher` spawned by the gateway.
   - Document that the homeserver now republishes both HTTP and Iroh records.
6. **Regression coverage**
   - Add a unit test that feeds a fake `NodeInfo` with relay and direct
     addresses and asserts the generated records list matches Iroh’s format.
   - Extend the integration test that checks the signed packet contents to
     include the `_iroh` TXT entries when the gateway is enabled.

## Testing Strategy

- **Unit tests** for the new conversion helpers, covering relay-only, direct
  address-only, and mixed scenarios.
- **Integration test** invoking the republisher against a temporary Pkarr
  client to confirm a single packet contains both record families.
- **Smoke test** from the launcher: start the homeserver with Iroh enabled,
  resolve `_iroh` TXT records via the Swiss Knife, and assert the relay URL
  matches the runtime configuration.

## Rollout Considerations

- Operators running without Iroh support experience no change—the additional
  hooks are gated behind `Option<NodeInfo>` values.
- Relay credentials are kept out of the packet; only public URLs are published.
- Metrics (`tracing` spans) should log whenever `_iroh` records are attached to a
  packet to help diagnose discovery issues in the field.

## Next Steps

- Land the `pubky-homeserver` changes described above upstream.
- Update the launcher documentation once the upstream release is available so
  end users know that enabling the gateway automatically publishes `_iroh`
  records.
- Coordinate with the Swiss Knife (Milestone 3) to consume the new records.
