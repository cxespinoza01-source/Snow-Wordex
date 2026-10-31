//! Android entry point. On desktop this binary only provides a helpful build hint.

#[cfg(target_os = "android")]
fn main() -> eframe::Result {
    eframe::run_native(
        "WordCraft",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(wordcraft_android::create_app()))),
    )
}

#[cfg(not(target_os = "android"))]
fn main() {
    eprintln!("wordcraft-android is an Android target; see apps/wordcraft-android/README.md");
}
