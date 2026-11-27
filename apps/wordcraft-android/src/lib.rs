//! Native Android host for the shared WordCraft editor.
//!
//! Android documents selected through SAF or shared by another app are copied into the
//! persistent app-private WordCraft/Documents library. The editor always opens the library copy,
//! so it does not depend on a temporary content:// URI or a picker cache file.

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

/// Construct the shared editor with Android's document picker service. A document shared
/// through ACTION_VIEW is opened immediately; otherwise Android starts in the document hub.
#[cfg(target_os = "android")]
pub fn create_app(ctx: eframe::egui::Context, library_dir: Option<PathBuf>, initial_document: Option<PathBuf>) -> WordApp {
    let region = wordcraft_ui_egui::i18n::system_region();
    let document = wordcraft_engine::blank_document(&region);
    let mut session = Session::new(document);
    session.system_region = region;
    let mut app = WordApp::new(session, android_services(ctx, library_dir));

    match initial_document {
        Some(path) => match app.open_file(&path.to_string_lossy(), None) {
            Ok(_) => {
                app.ui.backstage = false;
                app.canvas.want_focus = true;
            }
            Err(error) => {
                log::error!("Could not open shared document {}: {error}", path.display());
                app.start_screen();
            }
        },
        None => app.start_screen(),
    }
    app
}

#[cfg(target_os = "android")]
fn android_services(ctx: eframe::egui::Context, library_dir: Option<PathBuf>) -> Services {
    let picker_library_dir = library_dir.clone();
    let file_dialog: Box<dyn Fn(FileDialogRequest) -> Receiver<FileDialogAnswer>> = Box::new(move |request| {
        let (tx, rx) = channel();
        let repaint = ctx.clone();

        let purpose = match &request {
            FileDialogRequest::Open { purpose } | FileDialogRequest::OpenMany { purpose } => Some(purpose.clone()),
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
        let library_dir = picker_library_dir.clone();
        let callback = move |picked: robius_file_picker::Result<Option<robius_file_picker::PickedFile>>| {
            let answer = match picked {
                Ok(Some(file)) => match library_dir.as_deref() {
                    Some(directory) => stage_picked_file(file, directory).map(|path| vec![path.to_string_lossy().into_owned()]),
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
            .set_filters([("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"][..]), ("All files", &["*"][..])]),
        "document" => FileDialog::new().set_title("Open a document").set_filters([
            (
                "WordCraft documents",
                &[
                    "docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "rtf", "txt", "md", "markdown", "html", "htm", "tex", "latex", "ltx",
                    "json", "xml",
                ][..],
            ),
            ("All files", &["*"][..]),
        ]),
        _ => FileDialog::new().set_title("Choose a file").set_filters([("All files", &["*"][..])]),
    }
}

/// Copy a SAF-selected document into the persistent app-private library.
#[cfg(target_os = "android")]
fn stage_picked_file(file: robius_file_picker::PickedFile, library_dir: &Path) -> Result<PathBuf, String> {
    let display_name = file.file_name().filter(|name| !name.trim().is_empty()).unwrap_or("document").to_owned();
    let local = file.into_local_file().map_err(|error| format!("Could not read the selected document: {error}"))?;
    std::fs::create_dir_all(library_dir).map_err(|error| format!("Could not prepare WordCraft's document library: {error}"))?;
    let staged_path = unique_library_path(library_dir, &display_name);
    std::fs::copy(local.path(), &staged_path).map_err(|error| format!("Could not import the document into WordCraft's library: {error}"))?;
    Ok(staged_path)
}

/// Import the ACTION_VIEW content URI delivered by Android's "Open with" flow.
/// SAF grants temporary read access to the URI; copying it here makes it available to the
/// path-based document engine and keeps it in the library for later reopening.
#[cfg(target_os = "android")]
fn import_shared_document(app: &winit::platform::android::activity::AndroidApp, library_dir: &Path) -> Result<Option<PathBuf>, String> {
    use jni::{JavaVM, objects::{JObject, JString, JValue}, refs::Global};

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _) }.map_err(|error| format!("Could not connect to Android: {error}"))?;
    let activity_raw = app.activity_as_ptr() as jni::sys::jobject;
    let imported = vm
        .attach_current_thread(|env| -> jni::errors::Result<Option<(String, Vec<u8>)>> {
            // activity_as_ptr is an unowned JNI global reference. Keep it borrowed as a Global;
            // do not delete it when this function returns.
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&activity_raw)? };
            let intent = env.call_method(activity.as_ref(), "getIntent", "()Landroid/content/Intent;", &[])?.l()?;
            if intent.is_null() {
                return Ok(None);
            }
            let action = env.call_method(&intent, "getAction", "()Ljava/lang/String;", &[])?.l()?;
            if action.is_null() {
                return Ok(None);
            }
            let action: String = env.get_string(&JString::from(action))?.to_string_lossy().into_owned();
            if action != "android.intent.action.VIEW" {
                return Ok(None);
            }

            let uri = env.call_method(&intent, "getData", "()Landroid/net/Uri;", &[])?.l()?;
            if uri.is_null() {
                return Ok(None);
            }
            let segment = env.call_method(&uri, "getLastPathSegment", "()Ljava/lang/String;", &[])?.l()?;
            let mut name = if segment.is_null() {
                "shared-document".to_owned()
            } else {
                env.get_string(&JString::from(segment))?.to_string_lossy().into_owned()
            };
            let mime = env.call_method(&intent, "getType", "()Ljava/lang/String;", &[])?.l()?;
            let mime = if mime.is_null() { String::new() } else {
                env.get_string(&JString::from(mime))?.to_string_lossy().into_owned()
            };
            if Path::new(&name).extension().is_none() {
                if let Some(extension) = extension_for_mime(&mime) {
                    name.push('.');
                    name.push_str(extension);
                }
            }

            let resolver = env.call_method(activity.as_ref(), "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
            let input = env.call_method(
                &resolver,
                "openInputStream",
                "(Landroid/net/Uri;)Ljava/io/InputStream;",
                &[JValue::Object(&uri)],
            )?.l()?;
            if input.is_null() {
                return Err(jni::errors::Error::NullPtr("ContentResolver.openInputStream returned null"));
            }

            let buffer = env.new_byte_array(64 * 1024)?;
            let mut bytes = Vec::new();
            loop {
                let count = env.call_method(&input, "read", "([B)I", &[JValue::Object(&buffer)])?.i()?;
                if count < 0 {
                    break;
                }
                if count == 0 {
                    continue;
                }
                if bytes.len().saturating_add(count as usize) > 128 * 1024 * 1024 {
                    let _ = env.call_method(&input, "close", "()V", &[]);
                    return Err(jni::errors::Error::JavaException);
                }
                let mut chunk = vec![0_i8; count as usize];
                env.get_byte_array_region(&buffer, 0, &mut chunk)?;
                bytes.extend(chunk.into_iter().map(|byte| byte as u8));
            }
            env.call_method(&input, "close", "()V", &[])?;
            Ok(Some((name, bytes)))
        })
        .map_err(|error| format!("Android could not read the shared document: {error}"))?;

    let Some((display_name, bytes)) = imported else {
        return Ok(None);
    };
    if bytes.is_empty() {
        return Err("The selected document is empty or could not be read".to_owned());
    }
    std::fs::create_dir_all(library_dir).map_err(|error| format!("Could not prepare WordCraft's document library: {error}"))?;
    let destination = unique_library_path(library_dir, &display_name);
    std::fs::write(&destination, bytes).map_err(|error| format!("Could not import the shared document: {error}"))?;
    Ok(Some(destination))
}

#[cfg(target_os = "android")]
fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some("docx"),
        "application/vnd.ms-word.document.macroEnabled.12" => Some("docm"),
        "application/msword" => Some("doc"),
        "application/vnd.oasis.opendocument.text" => Some("odt"),
        "application/rtf" | "text/rtf" => Some("rtf"),
        "text/plain" => Some("txt"),
        "text/markdown" => Some("md"),
        "text/html" | "application/xhtml+xml" => Some("html"),
        "application/json" => Some("json"),
        "application/xml" | "text/xml" => Some("xml"),
        _ => None,
    }
}

#[cfg(target_os = "android")]
fn unique_library_path(library_dir: &Path, display_name: &str) -> PathBuf {
    let safe_name = Path::new(display_name)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("document"));
    let requested = library_dir.join(safe_name);
    if !requested.exists() {
        return requested;
    }

    let stem = requested.file_stem().and_then(|value| value.to_str()).unwrap_or("document");
    let extension = requested.extension().and_then(|value| value.to_str());
    let mut index = 2_u32;
    loop {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = library_dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
        index = index.saturating_add(1);
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // The library is private to WordCraft and survives process restarts. External documents
    // arrive through SAF or ACTION_VIEW; neither needs broad storage permissions.
    let library_dir = app.internal_data_path().map(|path| path.join("WordCraft").join("Documents"));
    let initial_document = match library_dir.as_deref() {
        Some(directory) => match import_shared_document(&app, directory) {
            Ok(path) => path,
            Err(error) => {
                log::error!("Could not import document from Android Files: {error}");
                None
            }
        },
        None => {
            log::error!("Android did not provide internal storage for the document library");
            None
        }
    };

    let options = eframe::NativeOptions { android_app: Some(app), ..Default::default() };
    if let Err(error) = eframe::run_native(
        "WordCraft",
        options,
        Box::new(move |cc| Ok(Box::new(AndroidHost(create_app(cc.egui_ctx.clone(), library_dir, initial_document))))),
    ) {
        log::error!("WordCraft Android exited with an error: {error}");
    }
}
