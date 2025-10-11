# Cross-building Portable Pubky apps on Ubuntu

Both desktop crates in this repository (`portable-homeserver` and `pubky-swiss-knife`) use the Dioxus desktop stack. That means
we need GTK/WebKit headers for the Linux build, MinGW for the Windows target, and the macOS SDK shims exposed by `cargo-zigbuild`
and `zig` for Apple targets. The instructions below let you produce release binaries for Linux, Windows, and macOS from an Ubuntu
host without leaving the terminal.

## 1. Install the host dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libglib2.0-dev \
  libssl-dev \
  mingw-w64 \
  unzip \
  zip
```

The `libwebkit2gtk` package name changes between Ubuntu releases. If you are on a distribution that still ships
`libwebkit2gtk-4.0-dev`, substitute that package name in the command above.

## 2. Install Zig

Ubuntu 24.04 and newer no longer ship a `zig` package, so the easiest path is to download the prebuilt toolchain and put it on
your `PATH`:

```bash
curl -L https://ziglang.org/download/0.12.1/zig-linux-x86_64-0.12.1.tar.xz -o /tmp/zig.tar.xz
sudo mkdir -p /opt/zig
sudo tar -C /opt/zig --strip-components=1 -xf /tmp/zig.tar.xz
sudo ln -sf /opt/zig/zig /usr/local/bin/zig
zig version
```

You should see `0.12.1` (or a newer release if you adjust the download link) printed in the last step. GitHub Actions can use
[`mlugg/setup-zig`](https://github.com/mlugg/setup-zig) to install the same toolchain during CI runs.

## 3. Install the Rust targets and helper tooling

```bash
rustup target add \
  x86_64-unknown-linux-gnu \
  x86_64-pc-windows-gnu \
  x86_64-apple-darwin

cargo install --locked cargo-zigbuild
```

`cargo-zigbuild` bundles the required C toolchains so the Apple and Windows builds succeed without additional SDKs.

## 4. Download the macOS SDK shims

Even though Zig ships a full clang toolchain, the macOS target still needs a copy of the Apple SDK so `libobjc`,
Foundation, AppKit, and other system frameworks resolve at link time. Apple does not publish these files for Linux,
so the easiest source is the community maintained [`phracker/MacOSX-SDKs`](https://github.com/phracker/MacOSX-SDKs)
mirror. Grab the 11.3 SDK (the newest release that still contains x86_64 binaries) and unpack it somewhere on disk:

```bash
sudo mkdir -p /opt/MacOSX-SDKs
curl -L https://github.com/phracker/MacOSX-SDKs/releases/download/11.3/MacOSX11.3.sdk.tar.xz \
  -o /tmp/MacOSX11.3.sdk.tar.xz
sudo tar -C /opt/MacOSX-SDKs -xf /tmp/MacOSX11.3.sdk.tar.xz
```

Export the SDK path before invoking `cargo zigbuild` so Zig knows where to look for system libraries:

```bash
export SDKROOT=/opt/MacOSX-SDKs/MacOSX11.3.sdk
export MACOSX_DEPLOYMENT_TARGET=11.0
```

You only need to set these environment variables for the macOS builds, but keeping them in your shell session makes
it harder to forget.

## 5. Build every target

With the system tooling in place you can now cross-compile the desktop crates. From the repository root run:

```bash
# Linux binaries (native build)
cargo build --manifest-path portable-homeserver/Cargo.toml --target x86_64-unknown-linux-gnu --release
cargo build --manifest-path pubky-swiss-knife/Cargo.toml --target x86_64-unknown-linux-gnu --release

# Windows `.exe` binaries
cargo zigbuild --manifest-path portable-homeserver/Cargo.toml --target x86_64-pc-windows-gnu --release
cargo zigbuild --manifest-path pubky-swiss-knife/Cargo.toml --target x86_64-pc-windows-gnu --release

# macOS binaries (x86_64)
cargo zigbuild --manifest-path portable-homeserver/Cargo.toml --target x86_64-apple-darwin --release
cargo zigbuild --manifest-path pubky-swiss-knife/Cargo.toml --target x86_64-apple-darwin --release
```

Artifacts land in `target/<target-triple>/release/`. Windows builds gain the `.exe` extension automatically, while Linux and
macOS builds keep the plain binary name.

The Linux builds run with the native toolchain so OpenSSL and other system libraries resolve correctly. The Windows and macOS
targets require `cargo-zigbuild`, which bundles the appropriate C toolchains.

## 6. Package the binaries

The GitHub workflow uses `tar.gz` archives, but you can package manually:

```bash
mkdir -p dist
for crate in portable-homeserver pubky-swiss-knife; do
  for target in x86_64-unknown-linux-gnu x86_64-pc-windows-gnu x86_64-apple-darwin; do
    bin_name=$(basename "$crate")
    ext=""
    if [[ $target == *"windows"* ]]; then
      ext=".exe"
    fi
    out_dir="target/$target/release"
    tar -C "$out_dir" -czf "dist/${bin_name}-${target}.tar.gz" "${bin_name}${ext}"
  done
done
```

The resulting archives mirror the structure uploaded by the automated release pipeline.
