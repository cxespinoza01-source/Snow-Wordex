# Native Android and Samsung DeX port

## Goal

Adapt this repository in place into a native Android application for tablets and Samsung DeX. Reuse the existing Rust document engine and `wordcraft-ui-egui`; do not replace the editor with a WebView or duplicate the document model.

## Current architecture found during the initial audit

- `crates/engine` and the document/format crates contain the reusable Rust editor and document functionality.
- `crates/ui-egui` implements the WordCraft ribbon UI and is shared by native desktop and WebAssembly builds.
- `apps/wordcraft` is the current desktop executable. Its entry point includes desktop-only window management, printing, clipboard, file-dialog, control-server and OS integration.
- `apps/wordcraft-web` is a separate browser target; it is useful as a reference for platform-specific services but is not the intended Android implementation.
- The workspace currently lists `apps/*` and has no dedicated Android application module in the inspected tree.

## Implementation principles

1. Keep the port in this repository and preserve the existing engine and UI crates.
2. Add a dedicated Android app/entry point instead of trying to compile the desktop executable unchanged.
3. Make platform integration explicit: document open/save via Android's Storage Access Framework, clipboard integration, keyboard/IME behavior, lifecycle handling, and graceful handling of unavailable desktop facilities.
4. Make the UI responsive to window size and input mode: touch-friendly controls on a tablet, compact ribbon when space is constrained, and desktop-style keyboard/mouse behavior in DeX.
5. Do not claim Android support or publish an APK until debug builds and device tests pass.

## Staged implementation

### Stage 1 — Build and dependency audit

- Confirm the selected `eframe`/egui/winit versions support the intended Android target and identify the exact Android entry-point API.
- Cross-compile the shared engine and UI crates for Android before adding platform-specific code.
- Inventory dependencies and modules in `apps/wordcraft/src/main.rs` that must not be compiled for Android.
- Keep printing, desktop updater, desktop control server and OS-specific window geometry out of the Android binary until equivalents are deliberately implemented.

### Stage 2 — Native Android shell

- Add a dedicated Android app package and Rust entry point that constructs the existing `Session`, `Services` and `WordApp`.
- Configure Android package metadata, app icon/name, supported ABIs, minimum SDK, and reproducible debug build instructions.
- Keep Android-specific code behind target-specific modules so desktop and WebAssembly builds remain unchanged.

### Stage 3 — Android services

- Implement open/save/export using Android's document picker and content URIs; avoid assuming documents have ordinary filesystem paths.
- Validate clipboard copy/paste, text input method (IME), soft keyboard visibility, app pause/resume, and unsaved-document recovery.
- Feature-gate or replace unsupported desktop-only facilities rather than adding stubs that silently lose data.

### Stage 4 — Tablet and DeX UX

- Adapt ribbon density and navigation panes to available width.
- Support touch selection as well as mouse selection, context menus, keyboard shortcuts, focus traversal and scroll-wheel/trackpad input.
- Handle resizing and display changes without losing editor state; ensure controls remain usable in both tablet and DeX window sizes.

### Stage 5 — Verification

- Run Rust formatting, target-specific checks and available tests.
- Build a debug APK for each selected ABI.
- Test new/open/edit/save round trips for DOCX and other advertised formats on a real Android device.
- Test keyboard, mouse, touch, split-screen/resizing, DeX external display, suspend/resume, and recovery from interrupted edits.
- Record known limitations before any release.

## Acceptance criteria

- Existing desktop and web targets are not regressed.
- Android app starts the shared WordCraft editor natively (not a WebView).
- Open and save work through Android's document picker without requiring broad storage permissions.
- Text editing works with both the soft keyboard and a physical keyboard.
- Tablet and DeX window resizing does not lose document state.
- A reproducible build and test report exists before an APK is described as ready.

## Current status

Planning/audit only. The Android module and APK have not yet been implemented or tested. The next code change should establish a minimal Android build target and prove that the shared Rust UI can be launched before broader UX work begins.
