//! Native Android host for the shared WordCraft editor.
//!
//! Uses GameActivity for Android text input and the system document picker (SAF) for opening files.
//! Selected SAF content URIs are copied to an app-cache staging path because the shared engine
//! currently opens filesystem paths. Imported copies are deleted after parsing; the opened document
//! is left unnamed so Save asks for a destination instead of overwriting an inaccessible cache copy.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use wordcraft_engine::Session;
use wordcraft_ui_egui::{FileDialogAnswer, FileDialogRequest, Services, WordApp};

/// Construct the shared editor with Android's document picker service.
pub fn create_app(ctx: eframe::egui::Context) -> WordApp {
    let region = wordcraft_ui_egui::i18n::system_region();
    let document = wordcraft_engine::blank_document(&region);
    let mut session = Session::new(document);
    session.system_region = region;
    WordApp::new(session, android_services(ctx))
}

fn android_services(ctx: egui::Context) -> Services {
    let file_dialog: Box<dyn Fn(FileDialogRequest) -> Receiver<FileDialogAnswer>> =
        Box::new(move |request| {
            let (tx, rx) = channel();
            let repaint = ctx.clone();

            let purpose = match &request {
                FileDialogRequest::Open { purpose } | FileDialogRequest::OpenMany { purpose } => {
                    Some(purpose.clone())
                }
                FileDialogRequest::Save { .. } => None,
            };

            let Some(purpose) = purpose else {
                let _ = tx.send(Err(
                    "Android Save As still needs a writable Storage Access Framework provider."
                        .to_owned(),
                ));
                repaint.request_repaint();
                return rx;
            };

            let dialog = android_open_dialog(&purpose);
            let callback_repaint = repaint.clone();
            let callback_tx = tx.clone();
            let callback = move |picked: robius_file_picker::Result<Option<robius_file_picker::PickedFile>>| {
                let answer = match picked {
                    Ok(Some(file)) => stage_picked_file(file)
                        .map(|path| vec![path.to_string_lossy().into_owned()]),
                    Ok(None) => Ok(Vec::new()),
                    Err(error) => Err(error.to_string()),
                };
                let _ = callback_tx.send(answer);
                callback_repaint.request_repaint();
            };

            if let Err(error) = dialog.pick_file(callback) {
                let _ = tx.send(Err(format!("Could not open Android's document picker: {error}")));
                repaint.request_repaint();
            }
            rx
        });

    Services {
        file_dialog: Some(file_dialog),
        ..Services::default()
    }
}

fn android_open_dialog(purpose: &str) -> robius_file_picker::FileDialog {
    use robius_file_picker::FileDialog;

    if purpose == "picture" {
        FileDialog::new()
            .set_title("Choose a picture")
            .set_filters([
                ("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"][..]),
                ("All files", &["*"][..]),
            ])
    } else {
        FileDialog::new()
            .set_title("Open a document")
            .set_filters([
                (
                    "WordCraft documents",
                    [
                        "docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "rtf", "txt",
                        "md", "markdown", "html", "htm", "tex", "latex", "ltx", "json", "xml",
                    ][..],
                ),
                ("All files", &["*"][..]),
            ])
    }
}

/// Convert the SAF URI to a short-lived regular path for the current path-based engine API.
fn stage_picked_file(file: robius_file_picker::PickedFile) -> Result<PathBuf, String> {
    let display_name = file
        .file_name()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("document")
        .to_owned();
    let safe_name = Path::new(&display_name)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("document"));

    let local = file
        .into_local_file()
        .map_err(|error| format!("Could not read the selected document: {error}"))?;
    let picker_dir = local
        .path()
        .parent()
        .ok_or_else(|| "The selected file has no cache directory".to_owned())?;
    let staging_dir = picker_dir.join("wordcraft-open");
    std::fs::create_dir_all(&staging_dir)
        .map_err(|error| format!("Could not prepare the document cache: {error}"))?;
    let staged_path = staging_dir.join(safe_name);
    std::fs::copy(local.path(), &staged_path)
        .map_err(|error| format!("Could not copy the selected document into the cache: {error}"))?;
    // The local temporary file is dropped here; the staged copy survives until WordApp parses it.
    Ok(staged_path)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "WordCraft",
        options,
        Box::new(|cc| Ok(Box::new(create_app(cc.egui_ctx.clone())))),
    ) {
        log::error!("WordCraft Android exited with an error: {error}");
    }
}
