# NAT Traversal Integration Plan

## Executive Summary

Pubky's homeserver currently republishes its transport coordinates to the Pkarr DHT and assumes that the announced public IP and ports accept inbound traffic. Research performed by the team shows that this assumption fails when the homeserver or the Swiss Knife clients run behind restrictive NAT or firewall boundaries. Iroh already solves the same reachability gap by announcing `_iroh` records to Pkarr and establishing connections through QUIC with transparent relay-based hole punching. This document evaluates that research and captures the concrete execution plan to merge the two worlds so that Pubky nodes remain reachable even when end users are behind NAT.

## Evaluation of the Existing Research

### Strengths

- **Shared primitives** – Both stacks already rely on Pkarr and on the same identity keypair, so sharing a combined `SignedPacket` is feasible without changing the naming scheme.
- **Proven discovery loop** – Iroh's pipeline has production-ready tooling: an `Endpoint` exposes QUIC listeners with automatic relay fallback and a `PkarrPublisher` keeps discovery data fresh every five minutes.
- **Reusable Pubky infrastructure** – Pubky already republishes records on a schedule and has builders for bootstrap/relay nodes, which can feed directly into an Iroh endpoint without additional state stores.

### Risks and Gaps

- **Publication overlap** – Having both Pubky and Iroh publish `SignedPacket`s independently can lead to signature races. We need a single authority building and publishing the packet.
- **Transport authorization** – Clients must validate that the Iroh tunnel terminates at the same identity keypair that signs Pubky's TLS certificates to avoid downgrade attacks.
- **Operational knobs** – The homeserver configuration does not yet expose relay-related controls or TTLs, which are required for production deployments.
- **Client ergonomics** – `pubky-swiss-knife` lacks any QUIC runtime or code paths for `_iroh` TXT resolution, so NAT traversal is currently impossible for users relying on that tool.

## Plan of Record

The work is split into four streams that can proceed largely in parallel.

### Dependency strategy

The previous spike vendored full copies of both `pubky-homeserver` and `iroh-net` into this
repository to experiment with the embedded gateway. That approach created unnecessary
maintenance overhead, duplicated upstream history, and obscured which bits of the codebase are
ours versus external dependencies. The published crates already expose the APIs we need for
Milestone 1, so we rely on them directly from `crates.io` instead of pulling local forks. Any
required fixes should be contributed upstream through focused patches rather than long-lived
local mirrors.

### 1. Homeserver Transport Runtime

- Embed an `iroh::Endpoint` inside `portable-homeserver`, using the existing homeserver keypair.
- Define a dedicated ALPN identifier (e.g. `b"pubky/iroh-homeserver/0"`).
- Implement a TCP-to-QUIC bridge that forwards inbound QUIC streams to the existing TLS listener on `localhost`.
- Add feature flags and configuration entries for relay URLs, bootstrap nodes, and direct address publication.

### 2. Unified Pkarr Publication

- Extend the existing `SignedPacket` builder to accept externally supplied resource records.
- Convert the endpoint's `NodeInfo` into `_iroh` TXT records using `iroh_relay::node_info` helpers.
- Ensure the homeserver publishes a single combined packet on its hourly cadence while the Iroh publisher keeps its five minute refresh for redundancy without conflicting signatures.
- Expand automated republishers to include `_iroh` records whenever they are available.

### 3. Client Updates (`pubky-swiss-knife`)

- Teach the client resolver to fetch `_iroh` TXT records alongside the current `SVCB`/`A` data.
- Introduce a reusable Iroh client runtime that honours the published relay mode and ALPN.
- Implement connection fallback logic: try direct HTTPS, then attempt Iroh P2P, and finally rely on relay-assisted tunnels.
- Provide a small proxy layer that exposes an HTTP interface over the QUIC stream to reuse the existing command implementations.

### 4. Quality Engineering

- Stand up automated end-to-end tests using the `pubky-testnet` fixtures with iptables rules that block inbound ports to simulate hard NAT scenarios.
- Document the deployment steps for operators that want to run their own relays.
- Add regression tests to ensure `_iroh` records remain present in the published packets when the feature is enabled.

## Milestones and Deliverables

| Milestone | Deliverable | Owners | Target |
| --- | --- | --- | --- |
| M1 | Iroh endpoint embedded in homeserver behind feature flag | Core Services | Week 1 |
| M2 | Unified Pkarr packet containing `_iroh` records | Core Services | Week 2 |
| M3 | Swiss Knife client fallback logic implemented | Tooling | Week 3 |
| M4 | NAT traversal E2E test suite & operator docs | QA & Docs | Week 4 |

## Immediate Next Actions

1. Create a feature branch and scaffold the Iroh endpoint integration.
2. Define configuration schema updates and wire them into the `AppContext` builder.
3. Prototype the QUIC-to-TCP tunnel locally to validate relay traversal with an artificial NAT (e.g. `tailscale up --shields-up`).

## Success Criteria

- Homeserver instances behind NAT remain reachable by Swiss Knife clients without manual port forwarding.
- Pkarr packets always contain both classical Pubky records and `_iroh` data when the feature is enabled.
- Clients seamlessly fall back from HTTPS to Iroh without requiring user intervention.
- CI covers the NAT traversal path to prevent regressions.

## Current Status and Operations Guide

The embedded gateway and client fallback paths are now implemented in this repository. To run `portable-homeserver` on mainnet behind NAT and consume it from `pubky-swiss-knife` on another device:

1. **Enable the gateway in configuration.** Set `discovery.iroh.enabled = true` in the homeserver `config.toml` and optionally provide `relay_url`, `direct_addresses`, and TTL/interval overrides. The gateway reuses the homeserver keypair, binds an `iroh::Endpoint` with ALPN `pubky/iroh-homeserver/0`, and exposes discovery snapshots to the republisher.
2. **Let the republisher merge `_iroh` records.** When the gateway reports live connectivity, its relay/direct addresses are merged with the existing HTTPS records before signing the Pkarr packet. If the gateway has not yet announced connectivity, the republisher falls back to the static config values so discovery continues.
3. **Rely on automatic client fallback.** The Swiss Knife HTTP tab first issues the normal HTTPS request. If that fails (for example, because inbound ports are blocked), it resolves the `_iroh` TXT attributes, opens a QUIC tunnel that bridges to the homeserver's local HTTP listener, and displays the response annotated as "via Iroh".
4. **Inspect discovery details.** The "Resolve _iroh" helper honours the mainnet/testnet toggle and renders the relay and address set that the fallback uses, making debugging straightforward when operating across multiple devices.

With these steps, operators can deploy the homeserver without manual port forwarding while Swiss Knife users seamlessly reach it through Iroh.

