# Android Installation Troubleshooting

## "You can't install the app on your device"

Pixel-class devices that ship without 32-bit support require 64-bit (`arm64-v8a`) native libraries inside the APK. The default
`dx bundle --android` workflow only copies the library that matches the GitHub runner architecture (for example, `x86_64` on
Ubuntu runners). The resulting APK installs on emulators but fails on physical phones with the "You can't install the app on your
device" message because the `lib/arm64-v8a` directory is missing entirely.

Ensure every build run compiles the Rust crate for all Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`,
`i686-linux-android`, and `x86_64-linux-android`) and copy each `jniLibs/<abi>` directory into the Gradle project before assembling
the release APK. That guarantees the artifact carries native code for every ABI Google Play and modern devices require.
