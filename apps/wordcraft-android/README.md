# WordCraft Android (native Rust host)

Experimental native Android host for the existing WordCraft Rust engine and egui editor. It is not a WebView wrapper and does not fork the document engine.

## Current scope

- Uses Android GameActivity so the host can participate in Android soft-keyboard and text-input handling.
- Uses the Android system document picker (Storage Access Framework) to open documents. The selected content URI is staged temporarily because the current shared engine reads filesystem paths.
- Imported staging copies are deleted after parsing. Since the current app does not yet write back to the original SAF URI, the opened document is treated as unsaved and Save must use Save As.
- Android Save As, write-back to provider URIs, export, clipboard integration, lifecycle recovery and physical-device/Samsung DeX verification are not complete. This branch is not release-ready.

## Build prerequisites

Install Rust, the Android SDK/NDK, Java 17, Gradle, and cargo-ndk. From a terminal:

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

The Android Gradle project includes the AndroidX GameActivity dependency and invokes cargo-ndk to build the native library. From this directory:

```sh
cd apps/wordcraft-android/android
gradle assembleDebug
```

The debug APK is normally written to `apps/wordcraft-android/android/app/build/outputs/apk/debug/app-debug.apk`. This build path has not yet been run in CI or on a physical device.

## Validation still required

1. Run `cargo check -p wordcraft-android --target aarch64-linux-android` with the Android NDK configured.
2. Build and install the APK on a physical ARM64 Android device.
3. Open DOCX, ODT, RTF, TXT and Markdown documents through the system picker.
4. Verify the on-screen keyboard, accented/multilingual input, composing text, backspace, selection and physical keyboard shortcuts.
5. Test rotation, background/resume, resizing and Samsung DeX window management.
6. Implement provider-backed Save As and verify that failures never discard unsaved changes.
