# Android, tablet and Samsung DeX validation

This checklist is for the experimental native Rust host in `apps/wordcraft-android`. A successful Rust cross-check is not proof that the app launches or that document workflows work.

## Build gate

- [ ] `cargo check -p wordcraft-android --target aarch64-linux-android` passes with the Android NDK configured.
- [ ] Package an APK with the `cargo-apk` version compatible with this repository's pinned eframe revision.
- [ ] Install and launch the APK on a physical ARM64 Android device.
- [ ] Record Android version, device model, NDK version and exact build command with the test result.

## Editing and input

- [ ] Create and edit a blank document.
- [ ] Test the on-screen keyboard, composition/IME input, accents and multilingual text.
- [ ] Test a physical keyboard: arrows, Shift selection, Ctrl shortcuts, Tab and Escape.
- [ ] Test touch selection, caret placement, long-press, scrolling and pinch/zoom behavior.
- [ ] Rotate the device and confirm the editor remains usable after activity recreation.
- [ ] Background and resume the app; verify unsaved edits remain intact.

## Document access

- [ ] Open a document through Android's system document picker (Storage Access Framework).
- [ ] Save a new document using a user-selected content URI.
- [ ] Save changes to an existing document where the provider permits it.
- [ ] Handle cancellation, read-only providers, revoked URI grants and write failures without losing edits.
- [ ] Confirm export formats report unsupported operations clearly instead of silently succeeding.

## Tablet and Samsung DeX

- [ ] Test portrait and landscape tablet layouts at common display scales.
- [ ] Resize the app window repeatedly in Samsung DeX and verify ribbon, page canvas and panes reflow.
- [ ] Verify the window can be maximized, restored and moved between displays.
- [ ] Verify focus indication and keyboard navigation without touch.
- [ ] Check that dialogs and menus stay inside the resized app window.
- [ ] Check behavior when an external display is connected or disconnected.

## Release gate

Do not mark the Android port production-ready until document open/save, lifecycle recovery, keyboard/IME, and a real Samsung DeX session have all passed. Attach logs and screenshots to the pull request for failures and fixes.
