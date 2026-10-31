# WordCraft Android (native Rust host)

Experimental scaffold for adapting Snow-Wordex to Android. The goal is to launch the existing `wordcraft-ui-egui` editor and `wordcraft-engine` directly; this is not a WebView wrapper and does not fork the document engine.

## Status

Not release-ready. Android Storage Access Framework open/save flows, lifecycle recovery, clipboard integration, and tablet/Samsung DeX verification still need implementation. Do not distribute an APK as a usable word processor until these are complete.

## Build prerequisites

Install Rust Android targets, Android SDK/NDK, and a `cargo-apk` version compatible with the repository's pinned eframe version. Confirm the exact build command with `cargo apk --help`.

Typical target installation:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
```

## Next milestones

1. Prove the native host compiles and launches on an Android device.
2. Implement document open/save using Android content URIs and the Storage Access Framework.
3. Add export handling and unsaved-change recovery.
4. Verify IME, physical keyboard, pointer/touch selection, resizing, and Samsung DeX.
5. Add target checks and document a reproducible APK build.
