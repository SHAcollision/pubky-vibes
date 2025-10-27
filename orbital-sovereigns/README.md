# Orbital Sovereigns

Orbital Sovereigns is a turn-based starship tactics prototype built to demonstrate decentralized
state management with the `pubky` 0.6.0-rc.6 SDK. The client is powered by Dioxus 0.7 and
presents three major pillars:

- **Self-sovereign identity** – Manage Pubky keypairs, recovery bundles, and homeserver sessions
  without relying on a centralized lobby.
- **Match orchestration** – Coordinate players around a deterministic battle simulation, with
  metadata and moves written to `/pub/orbital-sovereigns/...` on each participant's homeserver.
- **Asynchronous battles** – Apply tactical orders locally, serialize the resulting turn snapshot,
  and verify opponent updates by following a cryptographic hash chain stored on Pubky storage.

The crate ships a polished Dioxus desktop app, reusable storage utilities, and pure game logic that
can be embedded in headless bots or future frontends.

## Running the client

```bash
cargo run -p orbital-sovereigns
```

The UI allows you to:

1. Generate or import a Pubky keypair.
2. Sign up or sign in against a homeserver.
3. Create matches, publish metadata, and produce deterministic turn snapshots.
4. Poll an opponent's namespace for new moves, validate their integrity, and merge them into the
   local battle log.

All storage writes happen under `/pub/orbital-sovereigns/matches/<match-id>/`. Sensitive blueprints
and planning notes are encrypted client-side and stored in `/pub/orbital-sovereigns/vault/` until
homeservers expose a dedicated private namespace.

## Repository layout

```
orbital-sovereigns/
├── src/
│   ├── app.rs              — Dioxus root component and context wiring.
│   ├── components/         — Composable UI panels.
│   ├── models/             — Deterministic battle simulation data.
│   ├── services/           — Pubky facade, storage codecs, and sync loops.
│   ├── style.rs            — Centralized stylesheet.
│   ├── lib.rs              — Launch helpers shared between desktop and mobile builds.
│   └── main.rs             — Platform entry-points.
└── tests/                  — (Future) Integration flow harnesses.
```

## Game loop summary

1. **Blueprint preparation** – Compose hull tiles, reactors, and hardpoints into modular ships.
   The resulting JSON blueprints are encrypted and stored under the player's vault folder.
2. **Lobby & invites** – Hosts publish a `meta.json` document describing the arena, ship hashes,
   and initial turn order. Invites optionally include scoped capabilities for observers and AI
   copilots.
3. **Turn exchange** – On each turn the acting commander writes a `moves/<turn>-<color>.json`
   payload and updates the `latest.json` pointer. Opponents poll, verify the hash chain, and replay
   the deterministic simulation locally.
4. **Resolution** – When the match concludes, both sides archive the log and optionally prune or
   compress earlier turns.

This project is an evolving showcase for the Pubky ecosystem; contributions are welcome!
