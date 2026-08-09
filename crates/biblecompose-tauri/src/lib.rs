//! The desktop shell — a bridge, not a layer.
//!
//! Every command here translates a frontend request into a call on
//! [`biblecompose_app`] and translates the answer back. There is no domain
//! logic in this crate and there should never be: the CLI and the GUI must
//! agree about what a build is, and the only way to guarantee that is for both
//! to ask the same crate.
//!
//! The mirror of ADR-003's frontend rule. No Svelte component may reach for a
//! Tauri API; no Tauri command may reach past `biblecompose-app`.

use biblecompose_app::project;
use biblecompose_diagnostics::{Diagnostic as AppDiagnostic, Severity};
use biblecompose_scripture::plan::BookPlan;
use camino::Utf8PathBuf;
use serde::Serialize;

/// What the frontend's `Diagnostic` interface expects.
///
/// Declared here rather than derived on the diagnostics type because the wire
/// shape is the shell's business: the domain type is free to change its
/// internals, and this is the thing that has to stay compatible with a
/// TypeScript definition.
#[derive(Debug, Serialize)]
pub struct WireDiagnostic {
    pub code: String,
    pub severity: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<WireLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WireLocation {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

impl From<&AppDiagnostic> for WireDiagnostic {
    fn from(d: &AppDiagnostic) -> Self {
        WireDiagnostic {
            code: d.code.as_str().to_owned(),
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
            message: d.message.clone(),
            location: d.location.as_ref().map(|l| WireLocation {
                path: l.path.to_string(),
                line: l.line,
                column: l.column,
            }),
            help: d.help.clone(),
            detail: d.detail.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WireBook {
    pub code: String,
    pub path: String,
    pub chapters: usize,
}

#[derive(Debug, Serialize)]
pub struct WireProject {
    pub books: Vec<WireBook>,
    pub diagnostics: Vec<WireDiagnostic>,
}

#[derive(Debug, Serialize)]
pub struct WireVersions {
    pub app: String,
    pub contract: String,
    pub backend: String,
}

#[tauri::command]
fn versions() -> WireVersions {
    WireVersions {
        app: format!("biblecompose {}", env!("CARGO_PKG_VERSION")),
        contract: biblecompose_app::CONTRACT_VERSION.to_owned(),
        // A missing backend is reported rather than fatal: knowing the
        // application version is exactly what is wanted when the backend is
        // the thing that is broken.
        backend: biblecompose_app::backend_version()
            .unwrap_or_else(|d| format!("backend unavailable — {}", d.message)),
    }
}

#[tauri::command]
fn open_project(root: String) -> WireProject {
    let loaded = project::load(&Utf8PathBuf::from(root), &BookPlan::canonical());

    WireProject {
        books: loaded
            .document
            .books
            .iter()
            .zip(loaded.document.provenance.iter())
            .map(|(book, source)| WireBook {
                code: book.code.as_str().to_owned(),
                path: source.path.to_string(),
                chapters: book
                    .blocks
                    .iter()
                    .filter(|b| matches!(b, biblecompose_scripture::Block::Paragraph { .. }))
                    .count(),
            })
            .collect(),
        diagnostics: loaded
            .diagnostics
            .iter()
            .map(WireDiagnostic::from)
            .collect(),
    }
}

/// Build and run the window.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![versions, open_project])
        .run(tauri::generate_context!())
        .expect("the desktop shell failed to start");
}
