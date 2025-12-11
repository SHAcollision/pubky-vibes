# dioxus-template

A minimal Dioxus starter tailored for a single-application repository. The template ships ready-to-use workflows for desktop, web, Android, and release packaging while keeping the source as small as possible.

## What you get

- **Cross-platform entrypoints** powered by `dioxus` 0.6 with desktop, web, and Android launchers.
- **Allocator tuned for apps** via `mimalloc` on native builds.
- **Web assets** in `web/` with `Trunk.toml` for quick `trunk serve` sessions.
- **Android metadata** baked into `Cargo.toml` so `cargo apk` or `dx build --platform android` can produce an APK.
- **Opinionated CI** pipelines for formatting, linting, tests, platform builds, and release artifacts.

## Running locally

### Desktop

```bash
cargo run
```

### Web (wasm)

Install `trunk` once and serve:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
sudo apt-get install -y binaryen
sudo mv /usr/bin/wasm-opt /usr/bin/wasm-opt-orig
sudo cp scripts/wasm-opt /usr/bin/wasm-opt
sudo chmod +x /usr/bin/wasm-opt
trunk serve --open
```

The web entry disables `wasm-opt` (via `data-wasm-opt="0"`) to sidestep bulk-memory validation issues. If you want trunk to
optimize the wasm binary, remove that attribute after ensuring your chosen `wasm-opt` binary supports bulk-memory flags. Trunk
emits a `.wasm` module in `dist/` alongside the JS shim so CI can upload the full bundle and publish it to GitHub Pages.

### Android

Ensure you have the Android SDK + NDK (API level 30+) installed, then build an APK:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
ANDROID_SDK_ROOT=/path/to/sdk ANDROID_NDK_HOME=/path/to/ndk cargo apk build --lib --release --target aarch64-linux-android
```

The GitHub workflow builds the library target (`--lib`) so `cargo-apk` packages the `cdylib` Android expects while exporting
the native-activity entrypoint through `ndk_glue`. CI generates a throwaway release keystore
(password `android`) for builds to match the signing metadata in `Cargo.toml` and exports `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD` so `cargo-apk` can sign. If you prefer using your own signing keys, update the signing
fields in `Cargo.toml` and adjust the workflow to point at your keystore credentials.

## Continuous integration

- **CI (fmt, check, clippy, test):** `.github/workflows/ci.yml` keeps the codebase clean.
- **Web artifact:** `.github/workflows/web.yml` builds the WASM bundle with `trunk`, uploads the `dist` directory, and (on
  pushes) publishes it to GitHub Pages.
- **Desktop artifact:** `.github/workflows/desktop.yml` builds a release binary for Linux and uploads it.
- **Android APK:** `.github/workflows/android.yml` provisions the Android SDK/NDK, then runs `cargo apk` to generate an installable APK artifact.
- **Release bundles:** `.github/workflows/release.yml` reuses the platform build steps and attaches artifacts to a GitHub Release when a tag is pushed.

### Linux build prerequisites

Native builds (CI and local) rely on GTK/WebKit, indicator, and xdotool packages. Install them on Debian/Ubuntu with:

```bash
sudo apt-get update
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```

## Project layout

- `src/lib.rs` – shared UI (`app`) plus platform-specific launchers.
- `src/main.rs` – desktop entrypoint.
- `index.html` – Trunk entrypoint for web builds.
- `web/` – styles and other static assets copied into the web bundle.
- `icons/` – favicon and launcher icons referenced by `index.html`.
- `Dioxus.toml` and `Trunk.toml` – configuration for the Dioxus CLI and Trunk-based builds.

## Customizing

- Update `Cargo.toml` metadata for your app id, signing, and SDK levels.
- Extend `web/` with additional assets or routes.
- Tune workflow matrices to add more targets (macOS, Windows, iOS) as needed.
