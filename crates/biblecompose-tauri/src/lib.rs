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
//!
//! # Nothing long-running happens on the caller's thread
//!
//! GUI-012 and NFR-003: the window stays interactive and Cancel stays usable
//! through a build that takes minutes. So [`start_build`] returns as soon as
//! the work is handed to a thread, and everything the build has to say arrives
//! as events. A command that blocked would freeze the window even though Tauri
//! runs commands off the UI thread, because the frontend would be waiting on
//! its promise with nothing to show.

use std::sync::{Arc, Mutex};

use biblecompose_app::{project, BuildEvent, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_config::{edit, form, ConfigDocument, SettingsFile, SCHEMA_VERSION};
use biblecompose_diagnostics::{Diagnostic as AppDiagnostic, Diagnostics, Severity};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use tauri::{Emitter, Manager};

/// The name every build event is emitted under. One channel rather than one
/// per kind, so the frontend cannot process a state change before the
/// diagnostic that explains it.
const BUILD_EVENT: &str = "build";

// ---------------------------------------------------------------- wire types

/// What the frontend's `Diagnostic` interface expects.
///
/// Declared here rather than derived on the diagnostics type because the wire
/// shape is the shell's business: the domain type is free to change its
/// internals, and this is the thing that has to stay compatible with a
/// TypeScript definition.
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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
            severity: severity(d.severity),
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

const fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

/// One book, with enough for a row in the project pane (GUI-001).
#[derive(Debug, Clone, Serialize)]
pub struct WireBook {
    pub code: String,
    pub name: String,
    pub path: String,
    pub chapters: usize,
    /// Counted here rather than in the frontend, so the pane and the
    /// diagnostics panel cannot disagree about which book owns a problem.
    pub errors: usize,
    pub warnings: usize,
}

/// One row of the settings form (GUI-002).
#[derive(Debug, Clone, Serialize)]
pub struct WireSetting {
    pub key: String,
    /// Which control to render: `text`, `length`, `page_size`, `integer`,
    /// `boolean`, `path`, `list`.
    pub kind: &'static str,
    pub value: String,
    /// `true` when the project file set it — what the reset control reads
    /// (CFG-007).
    pub overridden: bool,
    /// Where it was set, for the inspector. Absent for a built-in value:
    /// ADR-005 refuses to invent a location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<WireLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WireProject {
    pub root: String,
    pub books: Vec<WireBook>,
    pub diagnostics: Vec<WireDiagnostic>,
    pub settings: Vec<WireSetting>,
    /// Where a build would write, resolved against the project folder.
    pub output: String,
    /// Whether a build can start at all (DIA-002).
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WireVersions {
    pub app: String,
    pub contract: String,
    pub backend: String,
}

/// A build event, flattened for the frontend.
///
/// One tagged shape rather than four event names, because the order matters:
/// a diagnostic explains the state that follows it, and separate channels
/// would let the window show "failed" before the reason arrived.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WireBuildEvent {
    State {
        state: &'static str,
    },
    Diagnostic {
        diagnostic: WireDiagnostic,
    },
    Log {
        stream: String,
        text: String,
    },
    Backend {
        version: String,
    },
    Output {
        path: String,
    },
    /// Not one of `BuildEvent`'s: the frontend needs to know the stream has
    /// ended even when the build died before reaching a terminal state.
    Finished {
        state: &'static str,
    },
}

const fn state_name(s: BuildState) -> &'static str {
    match s {
        BuildState::Idle => "idle",
        BuildState::Loading => "loading",
        BuildState::Loaded => "loaded",
        BuildState::Blocked => "blocked",
        BuildState::Validating => "validating",
        BuildState::Emitting => "emitting",
        BuildState::Typesetting => "typesetting",
        BuildState::Publishing => "publishing",
        BuildState::Succeeded => "succeeded",
        BuildState::Failed => "failed",
        BuildState::Cancelled => "cancelled",
    }
}

// ------------------------------------------------------------------- state

/// What the shell has to remember between commands.
#[derive(Default)]
pub struct Session {
    /// The cancel token of the build in flight. `Some` is also what "a build
    /// is running" means, so the two cannot get out of step.
    running: Mutex<Option<CancelToken>>,
}

// ---------------------------------------------------------------- commands

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
    project_at(&Utf8PathBuf::from(root))
}

/// Write one setting and reopen the project (CFG-005).
///
/// Reopening rather than patching a value in the frontend, because a setting
/// can change what the project *is* — `books.exclude` removes a book from the
/// pane — and a form that updated only itself would leave the rest of the
/// window describing the previous project.
#[tauri::command]
fn set_setting(
    root: String,
    key: String,
    value: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    write_setting(&Utf8PathBuf::from(root), &key, &value)
}

/// [`set_setting`] without the Tauri binding.
///
/// The commands are thin wrappers over functions like this one so the
/// interesting half of the shell is testable without a window —
/// `#[tauri::command]` generates helper macros whose names collide when the
/// function is made public, so the split is forced as well as useful.
pub fn write_setting(
    root: &Utf8Path,
    key: &str,
    value: &str,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let mut file = open_settings_file(root).map_err(|d| vec![WireDiagnostic::from(&d)])?;

    // The kind comes from the resolved schema, so the frontend never has to
    // say what type a key is — it only sends what the field holds.
    let (current, _) = project::settings(root);
    let kind = current.kind_of(key).unwrap_or(form::Kind::Text);

    edit::set_validated(&mut file, key, kind.read(value)).map_err(|d| wire_all(&d))?;
    file.save().map_err(|d| vec![WireDiagnostic::from(&d)])?;

    Ok(project_at(root))
}

/// Remove one setting, so the built-in value applies again (CFG-007).
#[tauri::command]
fn reset_setting(root: String, key: String) -> Result<WireProject, Vec<WireDiagnostic>> {
    clear_setting(&Utf8PathBuf::from(root), &key)
}

/// [`reset_setting`] without the Tauri binding.
pub fn clear_setting(root: &Utf8Path, key: &str) -> Result<WireProject, Vec<WireDiagnostic>> {
    let mut file = open_settings_file(root).map_err(|d| vec![WireDiagnostic::from(&d)])?;
    if file.reset(key) {
        file.save().map_err(|d| vec![WireDiagnostic::from(&d)])?;
    }
    Ok(project_at(root))
}

/// Start a build and return immediately (GUI-012, NFR-003).
///
/// Two threads: one runs the build, one drains its events onto the window.
/// The draining thread is what makes the log appear as it happens rather than
/// in one lump at the end, and it ends when the build drops its reporter —
/// which is the only signal that cannot lie about whether the build is over.
#[tauri::command]
fn start_build(
    app: tauri::AppHandle,
    session: tauri::State<'_, Arc<Session>>,
    root: String,
) -> Result<(), String> {
    let mut running = session
        .running
        .lock()
        .expect("the session lock is not poisoned");
    if running.is_some() {
        return Err("a build is already running".to_owned());
    }

    let cancel = CancelToken::new();
    *running = Some(cancel.clone());
    drop(running);

    let root = Utf8PathBuf::from(root);
    let session = Arc::clone(&session);
    let (reporter, rx) = BuildReporter::new();

    // The drain. Owns the receiver, so it ends when the build thread drops the
    // reporter and not before.
    let emitter = app.clone();
    let pump = std::thread::spawn(move || {
        let mut last = BuildState::Idle;
        for event in rx.iter() {
            if let BuildEvent::State(s) = &event {
                last = *s;
            }
            let _ = emitter.emit(BUILD_EVENT, wire_event(event));
        }
        last
    });

    std::thread::spawn(move || {
        let opened = project::open(&root);
        let request = BuildRequest::new(root, opened.output())
            .keeping_intermediates(*opened.settings.output.keep_intermediates)
            .with_settings(opened.settings.clone());

        let mut reporter = reporter;
        // Everything opening the project had to say reaches the panel before
        // the build starts, so a blocked build lists every reason at once
        // (DIA-002) rather than the first one the build happens to hit.
        for d in opened.diagnostics.iter() {
            reporter.diagnostic(d.clone());
        }

        biblecompose_app::build(&opened.document, &request, &cancel, &mut reporter);
        drop(reporter);

        let last = pump.join().unwrap_or(BuildState::Failed);
        let _ = app.emit(
            BUILD_EVENT,
            WireBuildEvent::Finished {
                state: state_name(last),
            },
        );
        *session
            .running
            .lock()
            .expect("the session lock is not poisoned") = None;
    });

    Ok(())
}

/// Ask the running build to stop (GUI-012).
///
/// Returns whether there was one. Cancellation is cooperative and the backend
/// process is killed by `biblecompose-sile`; all this does is set the flag
/// that decides when.
#[tauri::command]
fn cancel_build(session: tauri::State<'_, Arc<Session>>) -> bool {
    let running = session
        .running
        .lock()
        .expect("the session lock is not poisoned");
    match running.as_ref() {
        Some(token) => {
            token.cancel();
            true
        }
        None => false,
    }
}

#[tauri::command]
fn is_building(session: tauri::State<'_, Arc<Session>>) -> bool {
    session
        .running
        .lock()
        .expect("the session lock is not poisoned")
        .is_some()
}

// ---------------------------------------------------------------- plumbing

/// The project's settings file, opened for editing — created if the project
/// has never had one.
fn open_settings_file(root: &Utf8Path) -> Result<SettingsFile, AppDiagnostic> {
    let path = root.join(project::SETTINGS_FILE);
    if path.exists() {
        Ok(SettingsFile::new(ConfigDocument::read(&path)?))
    } else {
        Ok(SettingsFile::create(path, SCHEMA_VERSION))
    }
}

/// Everything the window shows about a project folder.
pub fn project_at(root: &Utf8Path) -> WireProject {
    let opened = project::open(root);

    let books = opened
        .document
        .books
        .iter()
        .zip(opened.document.provenance.iter())
        .map(|(book, source)| {
            let (errors, warnings) = counts_for(&opened.diagnostics, &source.path);
            WireBook {
                code: book.code.as_str().to_owned(),
                name: display_name(book),
                path: source.path.to_string(),
                chapters: book.chapter_count(),
                errors,
                warnings,
            }
        })
        .collect();

    WireProject {
        root: root.to_string(),
        books,
        diagnostics: wire_all(&opened.diagnostics),
        settings: opened
            .settings
            .fields()
            .into_iter()
            .map(wire_field)
            .collect(),
        output: opened.output().to_string(),
        blocked: opened.blocked(),
    }
}

/// The name to put on a row.
///
/// USFM's own precedence: the running head first, because it is the name the
/// publisher chose for the top of the page; then the short and long table-of
/// contents names; then the code, which is always there.
fn display_name(book: &biblecompose_scripture::Book) -> String {
    let names = &book.names;
    names
        .running
        .as_deref()
        .or(names.short.as_deref())
        .or(names.long.as_deref())
        .map(str::to_owned)
        .unwrap_or_else(|| book.code.to_string())
}

/// How many of each severity name this file. Attribution is by path because
/// that is what a diagnostic actually carries; a book with no diagnostics of
/// its own shows clean even when the project has problems elsewhere.
fn counts_for(diagnostics: &Diagnostics, path: &Utf8Path) -> (usize, usize) {
    let mine = diagnostics
        .iter()
        .filter(|d| d.location.as_ref().is_some_and(|l| l.path == path));
    let mut errors = 0;
    let mut warnings = 0;
    for d in mine {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => {}
        }
    }
    (errors, warnings)
}

fn wire_field(f: form::Field) -> WireSetting {
    WireSetting {
        key: f.key.to_owned(),
        kind: f.kind.as_str(),
        value: f.value,
        overridden: !f.origin.is_builtin(),
        // Through `location()` rather than matching the variants here: it is
        // the one place that decides which origins have a place to jump to,
        // and a settings field can never be `Inherited` anyway — only styles
        // inherit.
        location: f.origin.location().map(|loc| WireLocation {
            path: loc.path.to_string(),
            line: loc.line,
            column: loc.column,
        }),
    }
}

fn wire_all(diagnostics: &Diagnostics) -> Vec<WireDiagnostic> {
    diagnostics.iter().map(WireDiagnostic::from).collect()
}

fn wire_event(event: BuildEvent) -> WireBuildEvent {
    match event {
        BuildEvent::State(s) => WireBuildEvent::State {
            state: state_name(s),
        },
        BuildEvent::Diagnostic(d) => WireBuildEvent::Diagnostic {
            diagnostic: WireDiagnostic::from(&d),
        },
        BuildEvent::Log { stream, text } => WireBuildEvent::Log { stream, text },
        BuildEvent::Backend(version) => WireBuildEvent::Backend { version },
        BuildEvent::Output(path) => WireBuildEvent::Output {
            path: path.to_string(),
        },
    }
}

/// Build and run the window.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(Arc::new(Session::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            versions,
            open_project,
            set_setting,
            reset_setting,
            start_build,
            cancel_build,
            is_building,
        ])
        .run(tauri::generate_context!())
        .expect("the desktop shell failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire names and `BuildState` must not drift: the frontend switches
    /// on these strings, and a state it does not recognise is a window stuck
    /// on the previous one.
    #[test]
    fn every_build_state_has_a_wire_name() {
        use BuildState::*;
        let all = [
            Idle,
            Loading,
            Loaded,
            Blocked,
            Validating,
            Emitting,
            Typesetting,
            Publishing,
            Succeeded,
            Failed,
            Cancelled,
        ];
        let mut names: Vec<&str> = all.iter().map(|s| state_name(*s)).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len(), "two states share a wire name");
    }
}
