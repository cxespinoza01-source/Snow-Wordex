//! Native Android host for the shared WordCraft editor.
//!
//! Android document access is handled by the platform Storage Access Framework (SAF).
//! The Activity copies the selected content URI into the app cache and returns a filesystem
//! path to this host, keeping the Rust editor independent of Android content providers.

#[cfg(target_os = "android")]
use std::sync::mpsc::{Receiver, Sender, channel};
#[cfg(target_os = "android")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JString};
#[cfg(target_os = "android")]
use jni::{JNIEnv, JavaVM, jni_sig, jni_str};

#[cfg(target_os = "android")]
use wordcraft_engine::Session;
#[cfg(target_os = "android")]
use wordcraft_ui_egui::{FileDialogAnswer, FileDialogRequest, Services, WordApp};

#[cfg(target_os = "android")]
static ANDROID_VM: OnceLock<usize> = OnceLock::new();
#[cfg(target_os = "android")]
static ANDROID_ACTIVITY: OnceLock<usize> = OnceLock::new();
#[cfg(target_os = "android")]
static PENDING_PICKER: OnceLock<Mutex<Option<Sender<FileDialogAnswer>>>> = OnceLock::new();

#[cfg(target_os = "android")]
fn pending_picker() -> &'static Mutex<Option<Sender<FileDialogAnswer>>> {
    PENDING_PICKER.get_or_init(|| Mutex::new(None))
}

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

/// Construct the shared editor with Android's native document picker service.
#[cfg(target_os = "android")]
pub fn create_app(ctx: eframe::egui::Context) -> WordApp {
    let region = wordcraft_ui_egui::i18n::system_region();
    let document = wordcraft_engine::blank_document(&region);
    let mut session = Session::new(document);
    session.system_region = region;
    WordApp::new(session, android_services(ctx))
}

#[cfg(target_os = "android")]
fn android_services(ctx: eframe::egui::Context) -> Services {
    let file_dialog: Box<dyn Fn(FileDialogRequest) -> Receiver<FileDialogAnswer>> =
        Box::new(move |request| {
            let (tx, rx) = channel();
            let repaint = ctx.clone();

            let purpose = match &request {
                FileDialogRequest::Open { purpose } | FileDialogRequest::OpenMany { purpose } => {
                    purpose.as_str()
                }
                FileDialogRequest::Save { .. } => {
                    let _ = tx.send(Err(
                        "Android Save As is not implemented yet. The selected document is imported as a copy."
                            .to_owned(),
                    ));
                    repaint.request_repaint();
                    return rx;
                }
            };
            let _ = purpose; // SAF opens any provider-supported document; the engine validates its format.

            {
                let mut pending = pending_picker()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if pending.is_some() {
                    let _ = tx.send(Err("A document picker is already open".to_owned()));
                    repaint.request_repaint();
                    return rx;
                }
                *pending = Some(tx.clone());
            }

            if let Err(error) = launch_android_picker() {
                let sender = pending_picker()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take();
                if let Some(sender) = sender {
                    let _ = sender.send(Err(error));
                }
                repaint.request_repaint();
            }
            rx
        });

    Services {
        file_dialog: Some(file_dialog),
        ..Services::default()
    }
}

/// Invoke the Java Activity method using the Activity and VM supplied by android-activity.
/// We deliberately do not use ndk-context's process context: the SAF launcher requires an Activity.
#[cfg(target_os = "android")]
fn launch_android_picker() -> Result<(), String> {
    let vm_ptr = *ANDROID_VM
        .get()
        .ok_or_else(|| "Android Java VM is not available yet".to_owned())?;
    let activity_ptr = *ANDROID_ACTIVITY
        .get()
        .ok_or_else(|| "Android Activity is not available yet".to_owned())?;

    let vm = unsafe { JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) };
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("Could not attach to Android's Java VM: {error}"))?;

    // AndroidApp exposes this as an unowned global reference. Do not delete it.
    let activity = unsafe { JObject::from_raw(activity_ptr as jni::sys::jobject) };
    let result = env.call_method(
        &activity,
        jni_str!("openDocumentPicker"),
        jni_sig!("()V"),
        &[],
    );
    std::mem::forget(activity);

    result
        .map(|_| ())
        .map_err(|error| format!("Could not launch Android's document picker: {error}"))
}

/// Called by WordCraftActivity after the SAF result has been copied into the app cache.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_wordcraft_android_WordCraftActivity_nativeOnDocumentPicked(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    path: JString<'_>,
    error: JString<'_>,
) {
    let path = env
        .get_string(&path)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let error = if error.is_null() {
        String::new()
    } else {
        env.get_string(&error)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "Android could not read the selected document".to_owned())
    };

    let sender = pending_picker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();

    if let Some(sender) = sender {
        let answer = if !error.is_empty() {
            Err(error)
        } else if path.is_empty() {
            Ok(Vec::new()) // User cancelled the system picker.
        } else {
            Ok(vec![path])
        };
        let _ = sender.send(answer);
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Keep the exact Activity reference for the SAF bridge. android-activity already
    // initializes its Android context; do not initialize ndk-context a second time.
    let _ = ANDROID_VM.set(app.vm_as_ptr() as usize);
    let _ = ANDROID_ACTIVITY.set(app.activity_as_ptr() as usize);

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "WordCraft",
        options,
        Box::new(|cc| Ok(Box::new(AndroidHost(create_app(cc.egui_ctx.clone()))))),
    ) {
        log::error!("WordCraft Android exited with an error: {error}");
    }
}
