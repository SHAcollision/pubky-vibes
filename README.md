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
trunk serve --open
```

### Android

Ensure you have the Android SDK + NDK, then build an APK:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
aNDROID_SDK_ROOT=/path/to/sdk ANDROID_NDK_HOME=/path/to/ndk cargo apk build --release
```

The GitHub workflow handles toolchain setup automatically.

## Continuous integration

- **CI (fmt, check, clippy, test):** `.github/workflows/ci.yml` keeps the codebase clean.
- **Web artifact:** `.github/workflows/web.yml` builds the WASM bundle with `trunk` and uploads `dist/web`.
- **Desktop artifact:** `.github/workflows/desktop.yml` builds a release binary for Linux and uploads it.
- **Android APK:** `.github/workflows/android.yml` provisions the Android SDK/NDK, then runs `cargo apk` to generate an installable APK artifact.
- **Release bundles:** `.github/workflows/release.yml` reuses the platform build steps and attaches artifacts to a GitHub Release when a tag is pushed.

## Project layout

- `src/lib.rs` – shared UI (`app`) plus platform-specific launchers.
- `src/main.rs` – desktop entrypoint.
- `web/` – index, styles, and icon for web builds.
- `Dioxus.toml` and `Trunk.toml` – configuration for the Dioxus CLI and Trunk-based builds.

## Customizing

- Update `Cargo.toml` metadata for your app id, signing, and SDK levels.
- Extend `web/` with additional assets or routes.
- Tune workflow matrices to add more targets (macOS, Windows, iOS) as needed.
