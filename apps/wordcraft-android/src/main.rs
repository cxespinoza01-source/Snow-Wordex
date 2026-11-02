//! Android is launched through the exported android_main entry point in lib.rs.
//! This binary is only a host-side hint for developers browsing the workspace.

#[cfg(not(target_os = "android"))]
fn main() {
    eprintln!("wordcraft-android is an Android target; build the native library from the repository root. See apps/wordcraft-android/README.md");
}
