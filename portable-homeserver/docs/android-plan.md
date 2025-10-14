# Android Port Plan for Portable Pubky Homeserver

This plan follows the Dioxus 0.7.0 mobile tooling so the Android build offers the exact same launcher experience as the desktop binary.

## 1. Toolchain prerequisites
- Install the Rust `aarch64-linux-android` target so the bundler can compile native libraries for modern 64-bit-only phones.
- Provision the Android SDK, NDK 25.2.9519653, command-line tools, CMake, and platform/build tools via `sdkmanager` (the same stack the CI workflow installs).
- Export `JAVA_HOME`, `ANDROID_HOME`/`ANDROID_SDK_ROOT`, and `NDK_HOME`, and extend `PATH` with the SDK binaries (`cmdline-tools`, `platform-tools`, and the emulator) before running the CLI or Gradle.

## 2. Platform-neutral project structure
- Share the `App` component between desktop and mobile launchers so every control (network selector, config editor, start/stop buttons) behaves the same on both platforms.
- Gate the launch entrypoints with `#[cfg(target_os = "android")]` so the desktop binary still uses the WebView window title configuration while Android relies on `LaunchBuilder::mobile()`.
- Reuse the same data directory helpers and configuration pipeline; Android users can edit absolute paths manually just like on desktop when a native picker is unavailable.

## 3. Dioxus configuration
- Add a `Dioxus.toml` that sets the bundle metadata (identifier, title, publisher) and enables the Android target directory used by the bundler.
- Keep the metadata aligned with the desktop branding so release automation can package both binaries together.

## 4. Build & validation steps
- Run `cargo check` for the desktop target to ensure refactors keep the native build green.
- Use `dx bundle --android --release` to generate the Gradle project and compile the `aarch64-linux-android` native libraries.
- Patch the generated `app/src/main/AndroidManifest.xml` with `python3 scripts/patch_android_manifest.py` so the build includes the network permissions required by the static testnet and references a scoped network security config that only allows cleartext calls to localhost.
- Copy the generated `jniLibs/arm64-v8a` output into the shared Gradle project (the workflow automates this and adds `libc++_shared.so` from the NDK to satisfy runtime linking).
- Assemble the release APK via `./gradlew assembleRelease --console=plain` and archive the resulting unsigned APK (`app/build/outputs/apk/release/app-release-unsigned.apk`).

The launcher also sets `PUBKY_LMDB_MAP_SIZE_BYTES=268435456` when running on Android so LMDB uses a smaller 256 MiB map and avoids exhausting the device’s address space. Override the variable if a custom build needs a different cap.

## 5. Continuous integration
- Automate the bundling on GitHub Actions by installing the Android SDK, NDK 25.2.9519653, the Dioxus CLI, and the Rust target.
- Run the bundler, align and sign the APK with the debug keystore, verify the ABI coverage, and upload the signed artifact for downstream releases.
