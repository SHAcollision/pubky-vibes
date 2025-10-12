# Android Installation Troubleshooting

## "You can't install the app on your device"

Pixel 7/8-era handsets are 64-bit only; Google removed 32-bit runtime support entirely starting with the Pixel 7, so devices such
as the Pixel 7a refuse to install packages that lack an `arm64-v8a` payload.[^pixel64] Early iterations of our workflow only copied
the library that matched the GitHub runner architecture (for example, `x86_64` on Ubuntu runners). The resulting APK installed on
emulators but failed on physical phones with the "You can't install the app on your device" message because the `lib/arm64-v8a`
directory was missing entirely.

The CI workflow now targets `aarch64-linux-android` exclusively and reuses the generated `jniLibs/arm64-v8a` directory when
assembling the final Gradle project. That yields an APK that installs cleanly on Pixel 7-class hardware. If you see the error
again, confirm that the workflow logged an `dx bundle (aarch64-linux-android -> arm64-v8a)` step and that the produced APK contains
`lib/arm64-v8a/libpubky_swiss_knife.so`.

The resulting APK weighs roughly 8–9 MB when stripped and zipped. A significantly smaller artifact usually indicates that the
native library directory failed to make it into the build.

## Legacy ABI considerations

If we later reintroduce additional ABIs, keep the following historical issues in mind:

- Dioxus 0.7.0's Android backend expects the NDK to expose `armv7-linux-androideabiXX-clang` tool aliases even though the NDK
  ships `armv7a-` executables. Creating symbolic links from the `armv7a` filenames to the `armv7` aliases inside
  `$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin` resolves the mismatch.
- The 32-bit `i686-linux-android` variant requires linking against `libatomic` because mimalloc references 64-bit atomic
  intrinsics. If we need that target again we must add a build-script or `RUSTFLAGS` shim that locates and links the archive from
  the active NDK.

[^pixel64]: See Google's "[Moving the Android ecosystem to 64-bit only](https://android-developers.googleblog.com/2022/08/moving-android-ecosystem-to-64-bit-only.html)"
announcement and contemporaneous Pixel 7 reviews noting the absence of 32-bit app support, such as Ars Technica's
"[Google Pixel 7 review: Improved cameras make a great phone even better](https://arstechnica.com/gadgets/2022/10/google-pixel-7-review/)".
