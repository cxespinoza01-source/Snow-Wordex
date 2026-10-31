//! Native Android host for the shared WordCraft editor.
//!
//! This first porting milestone reuses the existing Rust engine and UI. Android document-picker
//! and Storage Access Framework services are a follow-up; this scaffold is not release-ready.

use wordcraft_engine::Session;
use wordcraft_ui_egui::{Services, WordApp};

/// Construct the shared WordCraft editor for the native Android host.
pub fn create_app() -> WordApp {
    let region = wordcraft_ui_egui::i18n::system_region();
    let document = wordcraft_engine::blank_document(&region);
    let mut session = Session::new(document);
    session.system_region = region;
    WordApp::new(session, Services::default())
}
