# Android Port Plan for Pubky Swiss Knife

This plan follows the Dioxus 0.7.0 guides for mobile tooling and bundling to deliver an Android APK with the same feature set as the desktop build while reusing the shared UI code.

## 1. Toolchain prerequisites
- Install the Rust Android `aarch64-linux-android` target so `dx` can cross-compile the app for modern 64-bit-only phones such as the Pixel 7a.
- Provision the Android SDK, NDK 25.2.9519653, command-line tools, CMake, and platform/build tools through `sdkmanager`, mirroring the setup described in the Dioxus mobile platform guide.
- Export `JAVA_HOME`, `ANDROID_HOME`/`ANDROID_SDK_ROOT`, `NDK_HOME`, and extend `PATH` with the SDK tools (`cmdline-tools`, `platform-tools`, and emulator binaries) to satisfy the CLI environment checks before building or serving Android targets.
- Google no longer ships 32-bit userspace on Pixel 7/8 hardware, so we intentionally skip legacy 32-bit and x86 ABIs in CI. If we need broader ABI coverage in the future we can expand the target list and reintroduce the compatibility steps.

## 2. Platform-neutral project structure
- Split platform bootstrapping from the UI so both desktop and mobile launch paths reuse the same `App` component tree.
- Introduce a thin platform facade (desktop vs. Android) that reuses the same sizing/title configuration on desktop while calling `LaunchBuilder::mobile()` on Android.
- Replace direct `rfd::FileDialog` usage with a platform abstraction so Android can still provide recovery-file interactions without depending on desktop-only crates. When bundling for Android the abstraction falls back to manual path entry because no native picker is wired up yet.

## 3. Dioxus configuration
- Add a `Dioxus.toml` that names the application, sets the output directories, and configures an Android bundle target with our desired package identifier and branding metadata.
- Ensure the bundle metadata matches the desktop build branding so both targets report the same identity and can share resources.

## 4. Build & validation steps
- Run `cargo check` for the desktop target to confirm the refactor keeps the native build healthy.
- Use the Dioxus CLI (`dx bundle --android --release`) to generate the Gradle project and compile the native libraries for Android. The project pins `dioxus-cli` 0.7.0-rc.1, the release candidate referenced by the Dioxus 0.7.0 Android guide.
- Repeat the bundle step for the `aarch64-linux-android` triple and copy the produced `jniLibs/arm64-v8a` folder into the shared Gradle project so the final APK installs cleanly on 64-bit devices.
- Archive the resulting APK (`target/dx/.../android/app/build/outputs/apk/release/app-release-unsigned.apk`) as a test artifact to confirm the tooling works end-to-end.

## 5. Continuous integration
- Automate the Android bundle on GitHub Actions by:
  - Installing the Rust toolchain and Android targets.
  - Installing the Android command-line tools & NDK 25.2.9519653.
  - Caching Gradle and Cargo directories to keep builds fast.
  - Running `dx bundle --android --release` and uploading the generated APK as a workflow artifact.

Executing these steps will yield an Android build with feature parity, centralized UI logic, and a reproducible CI pipeline.
