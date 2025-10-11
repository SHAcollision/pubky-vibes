# Portable Pubky Homeserver

<p align="center">
  <img src="https://pubky.org/pubky-core-logo.svg" alt="Pubky logo" width="140" />
</p>

This crate ships a lightweight desktop launcher for the [`pubky-homeserver`](https://crates.io/crates/pubky-homeserver) stack. It wraps the homeserver inside a [Dioxus](https://dioxuslabs.com/) desktop shell so you can start and stop your Pubky agent with a couple of clicks on macOS, Windows, or Linux.

## Why this exists

- **Portable first** – the binary bundles a cross-platform WebView UI using Dioxus. If a platform supports WebView (Windows, macOS, Linux, and even ARM SBCs), the homeserver experience is identical.
- **Simple onboarding** – choose where to persist keys and data, press “Start server”, and you get direct links to the Pubky admin API and TLS endpoints.
- **Testnet ready** – flip a radio button to boot the static `pubky-testnet` bundle with local relays and bootstrap services for demos.
- **Safe shutdown** – the homeserver stops gracefully as soon as you close the app or press the “Stop server” button.

## Getting started

1. Install Rust 1.80 or newer.
2. Build and run the desktop app:

   ```bash
   cargo run --release
   ```

3. Choose Mainnet or the bundled Static Testnet, confirm the data directory for Mainnet runs, and start the server.

The app renders a status card with useful connection details:

- Admin API socket (`http://<ip>:<port>`) for management tools.
- ICANN-compatible HTTP endpoint for legacy consumers.
- Pubky TLS URL and your homeserver public key for the decentralised network.

Configuration lives in `config.toml` within the chosen directory. The homeserver automatically creates missing folders, secrets, and config files on first launch.

## Customising the experience

- **Change the storage location**: edit the path in the UI. The app reuses the same folder on subsequent launches (persisted by the operating system’s application storage conventions via the `directories` crate).
- **Static testnet profile**: the bundled Testnet ignores the data directory and binds to fixed localhost ports so you can demo Pubky without touching your live keys.
- **Tweaking behaviour**: open `config.toml` in the data directory to adjust storage backends, rates, and other Pubky options. Restart the server from the UI to apply changes.
- **Troubleshooting**: when something fails to boot, the status panel surfaces the full error chain so you can quickly identify missing permissions or invalid config entries.

## Architecture

- `dioxus = 0.7.0-rc.1` powers the desktop UI, allowing us to ship a single codebase that feels native on each OS.
- `pubky-homeserver = 0.6.0-rc.6` runs inside the app. We keep it alive via a signal state container and drop it to shut the node down.
- `pubky-testnet = 0.6.0-rc.6` spins up a static local network (DHT, relays, and homeserver) when you select the Testnet radio option.
- The UI state is built with reactive signals so long-running async tasks (like spinning up the homeserver) don’t block the interface.

This is intentionally small so teams can iterate quickly during hackathons: reuse the UI skeleton, drop in your Pubky extensions, and you have a production-friendly launcher.
