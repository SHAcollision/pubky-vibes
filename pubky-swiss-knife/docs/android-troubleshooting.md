# Android Installation Troubleshooting

## "You can't install the app on your device"

Pixel 7/8-era handsets are 64-bit only; Google removed 32-bit runtime support entirely starting with the Pixel 7, so devices such
as the Pixel 7a refuse to install packages that lack an `arm64-v8a` payload even when an `armeabi-v7a` build is present.[^pixel64]
The default `dx bundle --android` workflow only copies the library that matches the GitHub runner architecture (for example,
`x86_64` on Ubuntu runners). The resulting APK installs on emulators but fails on physical phones with the "You can't install the
app on your device" message because the `lib/arm64-v8a` directory is missing entirely.

Ensure every build run compiles the Rust crate for all Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`,
`i686-linux-android`, and `x86_64-linux-android`) and copy each `jniLibs/<abi>` directory into the Gradle project before
assembling the release APK. That guarantees the artifact carries native code for every ABI Google Play and modern devices require.

The resulting universal APK weighs roughly 9–10 MB when stripped and zipped, which aligns with the two Rust dynamic libraries plus
Android support binaries; a significantly smaller artifact usually indicates that one or more architecture directories failed to
make it into the build.

## `dx bundle`: Android linker not found

Dioxus 0.7.0's Android backend expects the NDK to expose `armv7-linux-androideabiXX-clang` tool aliases, but Google's NDK ships the
executables as `armv7a-linux-androideabiXX-clang`. When the alias is missing the CLI aborts with:

```
ERROR dx bundle: Android linker not found at ".../armv7-linux-androideabi24-clang". Please set the `ANDROID_NDK_HOME` environment variable...
```

Creating symbolic links from the `armv7a` filenames to the `armv7` aliases inside
`$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin` resolves the mismatch and lets the bundle step finish for the
`armv7-linux-androideabi` target without downgrading the NDK.

[^pixel64]: See Google's "[Moving the Android ecosystem to 64-bit only](https://android-developers.googleblog.com/2022/08/moving-android-ecosystem-to-64-bit-only.html)"
announcement and contemporaneous Pixel 7 reviews noting the absence of 32-bit app support, such as Ars Technica's
"[Google Pixel 7 review: Improved cameras make a great phone even better](https://arstechnica.com/gadgets/2022/10/google-pixel-7-review/)".
