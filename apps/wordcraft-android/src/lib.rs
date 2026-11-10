//! Native Android host for the shared WordCraft editor.
//!
//! Uses GameActivity for Android text input and the system document picker (SAF) for opening files.
//! Selected SAF content URIs are copied to an app-cache staging path because the shared engine
//! currently opens filesystem paths. Imported copies are deleted after parsing; the opened document
//! is left unnamed so Save asks for a destination instead of overwriting an inaccessible cache copy.

#[cfg(target_os = "android")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use std::sync::mpsc::{Receiver, channel};

#[cfg(target_os = "android")]
use wordcraft_engine::Session;
#[cfg(target_os = "android")]
use wordcraft_ui_egui::{FileDialogAnswer, FileDialogRequest, Services, WordApp};

/// Thin eframe host: the shared WordApp owns the editor UI, while the platform binary
/// implements eframe::App and forwards each frame to it (as the desktop host does).
#[cfg(target_os = "android")]
struct AndroidHost(WordApp);

#[cfg(target_os = "android")]
impl eframe::App for AndroidHost {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
}

/// Construct the shared editor with Android's document picker service.
#[cfg(target_os = "android")]
pub fn create_app(ctx: eframe::egui::Context, library_dir: Option<PathBuf>) -> WordApp {
    let region = wordcraft_ui_egui::i18n::system_region();
    let document = wordcraft_engine::blank_document(&region);
    let mut session = Session::new(document);
    session.system_region = region;
    let mut app = WordApp::new(session, android_services(ctx, library_dir));

    // Android opens directly into WordCraft's document hub (File > Home), rather than
    // dropping the user into an empty editor. This is the first launcher-shell milestone:
    // the existing WordApp remains the single editor and owns document navigation.
    app.start_screen();
    app
}

#[cfg(target_os = "android")]
fn android_services(ctx: eframe::egui::Context, library_dir: Option<PathBuf>) -> Services {
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
                log::warn!("Android Save As is not implemented yet; cancelling the save request");
                let _ = tx.send(Ok(Vec::new()));
                repaint.request_repaint();
                return rx;
            };

            let dialog = android_open_dialog(&purpose);
            let callback_repaint = repaint.clone();
            let callback_tx = tx.clone();
            let library_dir = library_dir.clone();
            let callback = move |
                picked: robius_file_picker::Result<Option<robius_file_picker::PickedFile>>,
            | {
                let answer = match picked {
                    Ok(Some(file)) => match library_dir.as_deref() {
                        Some(directory) => stage_picked_file(file, directory)
                            .map(|path| vec![path.to_string_lossy().into_owned()]),
                        None => Err("Android did not provide persistent app storage for the document library".to_owned()),
                    },
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
        library_dir,
        ..Services::default()
    }
}

#[cfg(target_os = "android")]
fn android_open_dialog(purpose: &str) -> robius_file_picker::FileDialog {
    use robius_file_picker::FileDialog;

    match purpose {
        "picture" => FileDialog::new()
            .set_title("Choose a picture")
            .set_filters([
                ("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"][..]),
                ("All files", &["*"][..]),
            ]),
        "document" => FileDialog::new()
            .set_title("Open a document")
            .set_filters([
                (
                    "WordCraft documents",
                    &[
                        "docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "rtf", "txt",
                        "md", "markdown", "html", "htm", "tex", "latex", "ltx", "json", "xml",
                    ][..],
                ),
                ("All files", &["*"][..]),
            ]),
        _ => FileDialog::new()
            .set_title("Choose a file")
            .set_filters([("All files", &["*"][..])]),
    }
}

/// Import a SAF-selected document into WordCraft's persistent, app-private library.
/// The shared engine currently opens filesystem paths, so the imported copy remains available
/// after the picker cache is cleaned up and can be listed as a recent document.
#[cfg(target_os = "android")]
fn stage_picked_file(
    file: robius_file_picker::PickedFile,
    library_dir: &Path,
) -> Result<PathBuf, String> {
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
    std::fs::create_dir_all(library_dir)
        .map_err(|error| format!("Could not prepare WordCraft's document library: {error}"))?;

    let requested_path = library_dir.join(safe_name);
    let staged_path = if requested_path.exists() {
        let stem = requested_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("document");
        let extension = requested_path.extension().and_then(|value| value.to_str());
        let mut index = 2_u32;
        loop {
            let name = match extension {
                Some(extension) => format!("{stem} ({index}).{extension}"),
                None => format!("{stem} ({index})"),
            };
            let candidate = library_dir.join(name);
            if !candidate.exists() {
                break candidate;
            }
            index = index.saturating_add(1);
        }
    } else {
        requested_path
    };

    std::fs::copy(local.path(), &staged_path)
        .map_err(|error| format!("Could not import the document into WordCraft's library: {error}"))?;
    // The temporary SAF download is cleaned up on drop; the library copy persists.
    Ok(staged_path)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Android's internal data directory is private to this app and survives process restarts.
    // No broad storage permission is needed; users still choose external files through SAF.
    let library_dir = app
        .internal_data_path()
        .map(|path| path.join("WordCraft").join("Documents"));
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "WordCraft",
        options,
        Box::new(move |cc| {
            Ok(Box::new(AndroidHost(create_app(cc.egui_ctx.clone(), library_dir))))
        }),
    ) {
        log::error!("WordCraft Android exited with an error: {error}");
    }
}
