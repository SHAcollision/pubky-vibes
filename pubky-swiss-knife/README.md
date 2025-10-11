# Pubky Swiss Knife

![Pubky Core logotype](https://pubky.org/pubky-core-logo.svg)

A cross-platform Dioxus desktop application that exposes a graphical control panel for the Pubky SDK (`pubky` crate v0.6.0-rc.6`).

The interface ships with a fixed 1220×820 canvas, zero-scroll layouts, and a floating activity drawer so every workflow fits neatly on screen during demos.

The tool targets power users who need the flexibility of the CLI while offering a friendly multi-tab interface for:

- Generating, importing, and exporting Pubky keypairs.
- Managing encrypted recovery files.
- Signing arbitrary capability tokens.
- Bootstrapping sessions against homeservers (signup, signin, revalidate, signout).
- Running third-party authentication flows (present pubkyauth:// requests as QR codes, await approvals, or approve incoming deeplinks).
- Reading and writing session-scoped storage resources.
- Fetching public Pubky resources.
- Crafting raw Pubky/HTTPS requests with custom headers and bodies.

## Getting started

```bash
# Build the desktop app (requires a system `glib2` runtime for WebKit/GTK)
cargo run --release
```

The GTK/WebKit stack used by `dioxus-desktop` needs platform packages:

- **Linux:** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libglib2.0-dev` (package names vary per distro).
- **macOS:** the system SDK already ships the required frameworks.
- **Windows:** install the [WebKitGTK runtime](https://webkitgtk.org/) or use the official Dioxus bundle instructions.

## Tabs overview

### Keys

Manage signer key material:

- Generate a fresh keypair (random Ed25519).
- Import an existing secret key (base64-encoded 32 bytes).
- Export the loaded secret key back into the editor.
- Load or save encrypted recovery files using the shared passphrase format from `pubky-common` (paths auto-expand `~` and default to the `.pkarr` extension).

### Auth Tokens

Compose comma-separated capability strings (e.g. `/:rw,/pub/demo/:r`) and sign an `AuthToken` with the active keypair. The serialized token is rendered as base64 for sharing with other tools.

### Sessions

Interact with homeservers:

- Sign up to a homeserver using an optional invitation code.
- Sign in using root capabilities, revalidate the current session, or sign out explicitly.
- Inspect the hydrated `SessionInfo` debug dump to verify capabilities and metadata.

### Auth Flows

Coordinate QR-based authentication handshakes:

- Define capability scopes and optionally override the relay to spawn a `pubkyauth://` request.
- Present the resulting link as a QR code or copyable URL, await approval, or cancel the flow entirely.
- Automatically promote an approved flow to the active session (reusing the storage and HTTP tooling in other tabs).
- Paste any third-party `pubkyauth://` URL and approve it with the active keypair to deliver an encrypted token back to the requester.

### Storage

Two panels cover authenticated and public storage verbs:

- Session storage supports `GET`, `PUT`, and `DELETE` on absolute paths (e.g. `/pub/app/file.txt`).
- Public storage fetches arbitrary addressed resources like `pubky<pk>/pub/app/index.html` or `pubky://...` URLs.

Each action prints a cURL-style response preview (HTTP version, status, headers, and body or binary size).

### Raw Requests

A power-user console for issuing low-level Pubky or HTTPS requests:

- Select the HTTP method, target URL, free-form headers, and request body.
- Toggle between mainnet and testnet transport clients.
- Inspect the raw response just like in the storage view.

## Logging

All activity is appended to the "Activity" feed with color-coded status chips (info, success, error) to make debugging easier during hackathon development. The feed now lives in a floating drawer anchored to the lower-right corner—tap **Show activity** when you need insight and hide it again to keep the fixed-size workspace tidy.

## License

MIT
