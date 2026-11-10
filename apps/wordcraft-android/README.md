# WordCraft Android (native Rust host)

Experimental native Android host for the existing WordCraft Rust engine and egui editor. It is not a WebView wrapper and does not fork the document engine.

## Current scope

- Uses Android GameActivity so the host can participate in Android soft-keyboard and text-input handling.
- Uses the Android system document picker (Storage Access Framework) to import documents into an app-private library at `files/WordCraft/Documents`. The library survives app restarts and requires no broad storage permission.
- Duplicate file names are preserved by adding a numbered suffix instead of overwriting an existing library document.
- Android's File > Home remains the document hub; imported documents are opened from their persistent library paths.
- Imported staging copies are deleted after parsing. Since the current app does not yet write back to the original SAF URI, the opened document is treated as unsaved and Save must use Save As.
- Selecting an entire external folder and retaining its SAF tree permission are not implemented yet; external files are currently imported one at a time.
- A dedicated library browser, Android Save As/write-back to provider URIs, export, clipboard integration, lifecycle recovery and physical-device/Samsung DeX verification are not complete. This branch is not release-ready.

## Build prerequisites

Install Rust, the Android SDK/NDK, Java 17, Gradle, and cargo-ndk. From a terminal:

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

The Android Gradle project includes the AndroidX GameActivity dependency and invokes cargo-ndk to build the native library. From the repository root:

```sh
cd apps/wordcraft-android/android
gradle assembleDebug
```

The local debug APK is normally written to `apps/wordcraft-android/android/app/build/outputs/apk/debug/app-debug.apk`.

## Continuous integration

The `Android native check and APK build` GitHub Actions workflow checks the Rust Android library and attempts to build the ARM64 debug APK. If all steps succeed, it uploads an artifact named `wordcraft-android-debug-apk`. Open the pull request's **Checks** tab, select the successful workflow run, and download that artifact.

A green CI build only confirms packaging for the configured ARM64 target; it does not prove that the app behaves correctly on a tablet or in Samsung DeX. The APK has not been validated on physical hardware yet.

## Validation still required

1. Confirm the GitHub Actions Rust check and Gradle APK build both pass.
2. Install the APK on a physical ARM64 Android device.
3. Open DOCX, ODT, RTF, TXT and Markdown documents through the system picker.
4. Verify the on-screen keyboard, accented/multilingual input, composing text, backspace, selection and physical keyboard shortcuts.
5. Test rotation, background/resume, resizing and Samsung DeX window management.
6. Implement provider-backed Save As and verify that failures never discard unsaved changes.
