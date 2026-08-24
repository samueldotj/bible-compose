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

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use biblecompose_app::{
    project, BuildEvent, BuildReporter, BuildRequest, BuildState, CancelToken, Fingerprint,
};
use biblecompose_config::style::PROPERTIES;
use biblecompose_config::{
    cascade, edit, form, ConfigDocument, Origin, Settings, TomlFile, SCHEMA_VERSION,
};
use biblecompose_diagnostics::{Diagnostic as AppDiagnostic, Diagnostics, Severity};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
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
    /// Whether this book is in the publication (BOOK-003).
    ///
    /// A book that is out is still listed — it is on disk, it has a place in
    /// the order, and a list that hid it would be a list you could not put it
    /// back from. It is not parsed, so it has no chapters and no diagnostics.
    pub included: bool,
    /// `old`, `new` or `deuterocanon`, from the canon table — so the window can
    /// group the list without a second copy of which book is where.
    pub testament: &'static str,
}

/// One row of the settings form (GUI-002).
#[derive(Debug, Clone, Serialize)]
pub struct WireSetting {
    pub key: String,
    /// Which control to render: `text`, `length`, `page_size`, `integer`,
    /// `boolean`, `path`, `list`, `choice`.
    pub kind: &'static str,
    pub value: String,
    /// For `choice`, every spelling the resolver accepts, in the order to offer
    /// them; empty for every other kind.
    ///
    /// Sent rather than restated in TypeScript, because a list of head slots or
    /// caller styles kept in the window is a list that drifts from the one the
    /// resolver parses with — and the failure is a dropdown offering a value
    /// that gets rejected the moment it is chosen.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub choices: &'static [&'static str],
    /// `true` when the project file set it — what the reset control reads
    /// (CFG-007).
    pub overridden: bool,
    /// Where it was set, for the inspector. Absent for a built-in value:
    /// ADR-005 refuses to invent a location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<WireLocation>,
}

/// A project the window has opened before (GUI-001).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireRecent {
    pub root: String,
    /// The publication's name if its settings give one, else the folder's.
    ///
    /// Read from the settings file rather than remembered, so a project
    /// renamed since it was last opened is listed under the name it has now.
    pub name: String,
    /// Whether the folder is still there. A project moved or deleted stays on
    /// the list, greyed, because the useful thing to do with it is forget it
    /// deliberately — and a row that silently disappeared would leave somebody
    /// wondering whether they had imagined it.
    pub missing: bool,
}

/// The page, in points, for the window to draw (GUI-003).
///
/// Points and not the written units, and computed here rather than in the
/// frontend, for the reason the canon order is: `biblecompose-config` already
/// parses `0.55in`, `39.6pt` and `13.97mm` into one number, and a second
/// parser in TypeScript would be a second answer to what a margin is. The
/// drawing is then arithmetic on numbers somebody else has already agreed on.
#[derive(Debug, Clone, Serialize)]
pub struct WireGeometry {
    #[serde(rename = "pageWidth")]
    pub page_width: f64,
    #[serde(rename = "pageHeight")]
    pub page_height: f64,
    #[serde(rename = "marginTop")]
    pub margin_top: f64,
    #[serde(rename = "marginBottom")]
    pub margin_bottom: f64,
    #[serde(rename = "marginInner")]
    pub margin_inner: f64,
    #[serde(rename = "marginOuter")]
    pub margin_outer: f64,
    #[serde(rename = "columnGap")]
    pub column_gap: f64,
    #[serde(rename = "headerGap")]
    pub header_gap: f64,
    #[serde(rename = "footerGap")]
    pub footer_gap: f64,
    pub columns: u8,
}

impl WireGeometry {
    fn of(s: &Settings) -> WireGeometry {
        WireGeometry {
            page_width: s.page.size.width.points(),
            page_height: s.page.size.height.points(),
            margin_top: s.page.margin_top.points(),
            margin_bottom: s.page.margin_bottom.points(),
            margin_inner: s.page.margin_inner.points(),
            margin_outer: s.page.margin_outer.points(),
            column_gap: s.page.column_gap.points(),
            header_gap: s.page.header_gap.points(),
            footer_gap: s.page.footer_gap.points(),
            columns: *s.page.columns,
        }
    }
}

/// One font the picker offers (GUI-003).
#[derive(Debug, Clone, Serialize)]
pub struct WireFont {
    pub family: String,
    /// `project`, `backend`, or `system` — where a build would find it.
    pub source: &'static str,
    /// How many of the open Scripture's distinct characters it cannot draw.
    /// Absent when there is no project open to check it against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<usize>,
}

/// One property of one style, with where its value was decided (STY-008).
#[derive(Debug, Clone, Serialize)]
pub struct WireStyleProperty {
    pub name: &'static str,
    pub value: String,
    /// `builtin`, `file`, or `inherited`.
    pub origin: &'static str,
    /// The selector this was inherited from, when it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<WireLocation>,
}

/// One element's finished appearance (GUI-004).
#[derive(Debug, Clone, Serialize)]
pub struct WireStyle {
    pub selector: String,
    pub properties: Vec<WireStyleProperty>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WireProject {
    pub root: String,
    pub books: Vec<WireBook>,
    pub diagnostics: Vec<WireDiagnostic>,
    pub settings: Vec<WireSetting>,
    /// Every selector's resolved appearance, whether or not the project set
    /// any of it. The editor shows a curated few; the inspector needs them
    /// all, and sending the lot costs a few kilobytes once per open.
    pub styles: Vec<WireStyle>,
    /// The same books in canonical order, whatever `books.order` says.
    ///
    /// Sent because the window needs to know when an arrangement is the
    /// canonical one — that is when it clears `books.order` rather than
    /// writing an explicit copy of it — and the canon table lives here.
    /// Mirroring sixty-six rows into TypeScript would be a second copy of
    /// canon knowledge, kept where it cannot be checked against the first.
    #[serde(rename = "canonicalOrder")]
    pub canonical_order: Vec<String>,
    /// The page as numbers, for the diagram beside the page settings.
    pub geometry: WireGeometry,
    /// Where a build would write, resolved against the project folder.
    pub output: String,
    /// Whether a build can start at all (DIA-002).
    pub blocked: bool,
}

/// What the application is configured to do before any project says otherwise.
///
/// The window shows these at startup rather than two empty panes. A publisher
/// deciding whether this tool suits them should be able to see what it does
/// with a Bible before they have one open, and CFG-001 and STY-001 both say
/// there is always an answer — so showing nothing was showing less than the
/// truth.
#[derive(Debug, Clone, Serialize)]
pub struct WireDefaults {
    pub settings: Vec<WireSetting>,
    pub styles: Vec<WireStyle>,
    /// So the page can be drawn before a folder is open, which is the same
    /// reason the settings and styles are sent at all (CFG-001, STY-001).
    pub geometry: WireGeometry,
}

/// What has changed on disk since the window last read the project.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WireChanges {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
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
    /// Where the backend's output is being written. Announced before the run,
    /// so the window can offer it even for a build that never finishes.
    LogFile {
        path: String,
    },
    /// How far the typesetter has got. `expected` is the last build's page
    /// count and is absent the first time a project is built, because nothing
    /// knows how long a document is until it has been set.
    Pages {
        done: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected: Option<u32>,
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
    /// What the open project looked like on disk when it was last read
    /// (FUN-007).
    watched: Mutex<Option<(Utf8PathBuf, Fingerprint)>>,
    /// The distinct characters the open Scripture sets, kept so the font
    /// picker can check three hundred families against them without reading
    /// the books again (GUI-003).
    characters: Mutex<BTreeSet<char>>,
}

// ------------------------------------------------------------ recent projects

/// How many projects the start screen remembers.
///
/// Ten: enough that a publisher working on a handful of publications never has
/// to go looking for one, few enough that the list stays a list rather than a
/// history to be searched.
const REMEMBERED: usize = 10;

/// Where the list lives.
///
/// The application's own config directory, not the project folders — which
/// projects this machine has opened is a fact about this machine, and writing
/// it into a publication would put one person's habits into everybody's
/// repository.
fn recents_path(app: &tauri::AppHandle) -> Option<Utf8PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    let dir = Utf8PathBuf::from_path_buf(dir).ok()?;
    Some(dir.join("recent.json"))
}

fn read_recents(app: &tauri::AppHandle) -> Vec<Utf8PathBuf> {
    let Some(path) = recents_path(app) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path.as_std_path()) else {
        return Vec::new();
    };
    // A list that will not parse is a list worth discarding: it is a
    // convenience, and the cost of losing it is one folder picker.
    serde_json::from_str::<Vec<String>>(&text)
        .unwrap_or_default()
        .into_iter()
        .map(Utf8PathBuf::from)
        .collect()
}

fn write_recents(app: &tauri::AppHandle, roots: &[Utf8PathBuf]) {
    let Some(path) = recents_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent.as_std_path());
    }
    let text: Vec<&str> = roots.iter().map(|r| r.as_str()).collect();
    if let Ok(json) = serde_json::to_string_pretty(&text) {
        // Best-effort throughout. A machine that cannot write this still opens
        // projects; it just does not offer them back.
        let _ = std::fs::write(path.as_std_path(), json);
    }
}

/// Put a project at the top of the list, and keep it there once.
fn remember(app: &tauri::AppHandle, root: &Utf8Path) {
    let mut roots = read_recents(app);
    roots.retain(|r| r != root);
    roots.insert(0, root.to_owned());
    roots.truncate(REMEMBERED);
    write_recents(app, &roots);
}

/// What the start screen shows.
#[tauri::command]
fn recent_projects(app: tauri::AppHandle) -> Vec<WireRecent> {
    read_recents(&app).into_iter().map(describe).collect()
}

/// Drop one from the list. The folder is not touched.
#[tauri::command]
fn forget_project(app: tauri::AppHandle, root: String) -> Vec<WireRecent> {
    let mut roots = read_recents(&app);
    roots.retain(|r| r.as_str() != root);
    write_recents(&app, &roots);
    roots.into_iter().map(describe).collect()
}

/// A remembered folder, as a row.
fn describe(root: Utf8PathBuf) -> WireRecent {
    let missing = !root.is_dir();
    // The settings file is read directly rather than through `project::open`:
    // this runs for every row on the start screen, and opening ten projects to
    // draw a list of ten names would parse every book in all of them.
    let named = (!missing)
        .then(|| project::settings(&root).0.project.name.as_deref().cloned())
        .flatten();

    WireRecent {
        name: named.unwrap_or_else(|| root.file_name().unwrap_or_else(|| root.as_str()).to_owned()),
        root: root.to_string(),
        missing,
    }
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

/// The built-in settings and styles, with no project involved.
#[tauri::command]
fn defaults() -> WireDefaults {
    builtin_config()
}

/// [`defaults`] without the Tauri binding.
pub fn builtin_config() -> WireDefaults {
    let builtin = Settings::builtin();
    WireDefaults {
        settings: builtin.fields().into_iter().map(wire_field).collect(),
        styles: wire_styles(&cascade::resolve(None, false).0),
        geometry: WireGeometry::of(&builtin),
    }
}

#[tauri::command]
fn open_project(
    app: tauri::AppHandle,
    session: tauri::State<'_, Arc<Session>>,
    root: String,
) -> WireProject {
    let root = Utf8PathBuf::from(root);
    remember(&app, &root);
    grow_to_workspace(&app);
    name_the_window(&app, Some(&root));
    observe(&session, &root)
}

/// Show a folder in the platform's own file manager (GUI-009).
///
/// A spawn rather than a plugin. The three commands below are the whole of
/// what a file-manager plugin would do here, and a dependency that has to be
/// kept current, audited and given a capability in order to run `explorer` is
/// a poor trade for three lines.
///
/// The path is one this application produced — a project root or a build
/// directory — and never text from a document, which is what makes handing it
/// to a shell-adjacent program reasonable.
#[tauri::command]
fn open_folder(path: String) -> Result<(), Vec<WireDiagnostic>> {
    let path = Utf8PathBuf::from(path);
    if !path.is_dir() {
        return Err(vec![WireDiagnostic::from(
            &AppDiagnostic::error(
                biblecompose_diagnostics::code::COULD_NOT_CREATE,
                format!("{path} is not a folder that exists"),
            )
            .help("build first, or the folder was moved since it was opened"),
        )]);
    }

    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("explorer", &[])
    } else if cfg!(target_os = "macos") {
        ("open", &[])
    } else {
        ("xdg-open", &[])
    };

    // `explorer` reports failure through its exit code even when it worked, so
    // what is checked is whether the program could be started at all — which
    // is the failure worth reporting anyway: a machine with no file manager
    // where this application expects one.
    std::process::Command::new(program)
        .args(args)
        .arg(path.as_std_path())
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            vec![WireDiagnostic::from(
                &AppDiagnostic::error(
                    biblecompose_diagnostics::code::COULD_NOT_CREATE,
                    format!("could not open {path} in a file manager"),
                )
                .detail(e.to_string()),
            )]
        })
}

/// Open the finished PDF in whatever the platform reads PDFs with (GUI-009).
///
/// **There is no integrated preview and this is what stands in for one.**
/// GUI-008 is a SHOULD and [ADR-003](../../../docs/adr/003-gui.md) declines it:
/// a viewer inside the window would be a second PDF renderer to keep, and the
/// one the publisher already trusts is the one their printer will use.
///
/// Same spawn as `open_folder`, and the same argument for it. The path is one
/// this application produced and never text out of a document.
///
/// On Windows the file is handed to `explorer`, which opens it with its
/// registered application. `cmd /c start` would do the same and would also put
/// the path through a shell — for a filename this application chose that is
/// safe today and is one refactor away from not being.
#[tauri::command]
fn open_pdf(path: String) -> Result<(), Vec<WireDiagnostic>> {
    let path = Utf8PathBuf::from(path);
    if !path.is_file() {
        return Err(vec![WireDiagnostic::from(
            &AppDiagnostic::error(
                biblecompose_diagnostics::code::COULD_NOT_CREATE,
                format!("{path} is not a file that exists"),
            )
            .help("build first, or the file was moved or renamed since"),
        )]);
    }

    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    std::process::Command::new(program)
        .arg(path.as_std_path())
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            vec![WireDiagnostic::from(
                &AppDiagnostic::error(
                    biblecompose_diagnostics::code::COULD_NOT_CREATE,
                    format!("could not open {path}"),
                )
                .help("no application on this machine is registered for PDF files")
                .detail(e.to_string()),
            )]
        })
}

/// Put the project down and go back to the start screen.
///
/// The window forgets what it was watching, so the change detector stops
/// comparing a folder nobody is looking at, and the font picker stops holding
/// a closed publication's alphabet.
///
/// Nothing on disk is touched: this closes a *view*, and the project is a
/// folder that was there before the window opened it.
#[tauri::command]
fn close_project(app: tauri::AppHandle, session: tauri::State<'_, Arc<Session>>) {
    *session
        .watched
        .lock()
        .expect("the session lock is not poisoned") = None;
    session
        .characters
        .lock()
        .expect("the session lock is not poisoned")
        .clear();
    shrink_to_launcher(&app);
    name_the_window(&app, None);
}

/// Start a new project and open it (PRJ-001).
///
/// Creating and opening in one command rather than two, because a folder with
/// a settings file in it and no Scripture is not a state anybody wants to be
/// left holding: the window that made it should be showing it, with the empty
/// book list that tells the publisher what to do next.
#[tauri::command]
fn create_project(
    app: tauri::AppHandle,
    session: tauri::State<'_, Arc<Session>>,
    parent: String,
    name: String,
    language: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let root = project::create(Utf8Path::new(&parent), &name, &language)
        .map_err(|d| vec![WireDiagnostic::from(&d)])?;
    remember(&app, &root);
    grow_to_workspace(&app);
    name_the_window(&app, Some(&root));
    let project = observe(&session, &root);
    Ok(project)
}

/// Read the project and remember what it looked like.
///
/// Every command that touches the project goes through this, including the
/// ones that *write* — a settings file we just saved is not an external
/// change, and reporting it as one would put a "reload?" prompt on screen
/// after every edit the window itself made.
fn observe(session: &Session, root: &Utf8Path) -> WireProject {
    let opened = project::open(root);
    // From this open rather than a second one: the picker's answer must be
    // about the books the window is showing.
    *session
        .characters
        .lock()
        .expect("the session lock is not poisoned") =
        biblecompose_app::font::characters(&opened.document);
    *session
        .watched
        .lock()
        .expect("the session lock is not poisoned") =
        Some((root.to_owned(), Fingerprint::take(root)));
    wire_project(root, opened)
}

/// Every font a build could resolve, and whether it can set this Scripture
/// (GUI-003).
///
/// The coverage count is the reason this is not the operating system's font
/// dialog: choosing a font that cannot draw the book is the mistake FONT-002
/// exists to catch, and a picker that allows it silently has simply moved the
/// error later.
#[tauri::command]
fn fonts(session: tauri::State<'_, Arc<Session>>, root: Option<String>) -> Vec<WireFont> {
    let characters = session
        .characters
        .lock()
        .expect("the session lock is not poisoned")
        .clone();
    // Without a project there is still a list worth showing — the built-in
    // settings are editable before a folder is open — it just cannot say
    // which entries cover anything.
    let root = root.map(Utf8PathBuf::from).unwrap_or_default();

    biblecompose_app::fonts(&root, &characters)
        .into_iter()
        .map(|c| WireFont {
            family: c.family,
            source: c.source.as_str(),
            missing: c.missing,
        })
        .collect()
}

/// What has changed on disk since the project was last read (FUN-007).
#[tauri::command]
fn changed_files(session: tauri::State<'_, Arc<Session>>) -> WireChanges {
    let watched = session
        .watched
        .lock()
        .expect("the session lock is not poisoned");
    let Some((root, fingerprint)) = watched.as_ref() else {
        return WireChanges::default();
    };

    let changes = fingerprint.changes(root);
    WireChanges {
        modified: changes.modified.iter().map(name_of).collect(),
        added: changes.added.iter().map(name_of).collect(),
        removed: changes.removed.iter().map(name_of).collect(),
    }
}

/// File names rather than paths: the window shows them in one line, and the
/// project folder is already named above it.
fn name_of(path: &Utf8PathBuf) -> String {
    path.file_name().unwrap_or(path.as_str()).to_owned()
}

/// Write one setting and reopen the project (CFG-005).
///
/// Reopening rather than patching a value in the frontend, because a setting
/// can change what the project *is* — `books.include` decides which books are in the
/// pane — and a form that updated only itself would leave the rest of the
/// window describing the previous project.
#[tauri::command]
fn set_setting(
    session: tauri::State<'_, Arc<Session>>,
    root: String,
    key: String,
    value: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let root = Utf8PathBuf::from(root);
    let project = write_setting(&root, &key, &value)?;
    forget_changes(&session, &root);
    Ok(project)
}

/// Take a fresh fingerprint after the window has written something, so its own
/// edit is not reported back to it as an external one.
fn forget_changes(session: &Session, root: &Utf8Path) {
    *session
        .watched
        .lock()
        .expect("the session lock is not poisoned") =
        Some((root.to_owned(), Fingerprint::take(root)));
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

    edit::set_validated(&mut file, key, kind.read(value), &edit::settings_check)
        .map_err(|d| wire_all(&d))?;
    file.save().map_err(|d| vec![WireDiagnostic::from(&d)])?;

    Ok(project_at(root))
}

/// Remove one setting, so the built-in value applies again (CFG-007).
#[tauri::command]
fn reset_setting(
    session: tauri::State<'_, Arc<Session>>,
    root: String,
    key: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let root = Utf8PathBuf::from(root);
    let project = clear_setting(&root, &key)?;
    forget_changes(&session, &root);
    Ok(project)
}

/// [`reset_setting`] without the Tauri binding.
pub fn clear_setting(root: &Utf8Path, key: &str) -> Result<WireProject, Vec<WireDiagnostic>> {
    let mut file = open_settings_file(root).map_err(|d| vec![WireDiagnostic::from(&d)])?;
    if file.reset(key) {
        file.save().map_err(|d| vec![WireDiagnostic::from(&d)])?;
    }
    Ok(project_at(root))
}

/// Set one style property (STY-005).
#[tauri::command]
fn set_style(
    session: tauri::State<'_, Arc<Session>>,
    root: String,
    selector: String,
    property: String,
    value: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let root = Utf8PathBuf::from(root);
    let project = write_style(&root, &selector, &property, &value)?;
    forget_changes(&session, &root);
    Ok(project)
}

/// [`set_style`] without the Tauri binding.
pub fn write_style(
    root: &Utf8Path,
    selector: &str,
    property: &str,
    value: &str,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    if !PROPERTIES.contains(&property) {
        return Err(vec![WireDiagnostic::from(&not_a_property(property))]);
    }

    let mut file = open_styles_file(root).map_err(|d| vec![WireDiagnostic::from(&d)])?;
    let strict = *project::settings(root).0.strict;

    // Style properties are strings, integers or booleans. Which one a property
    // is comes from the schema, not from guessing at the text.
    let parsed = style_kind(property).read(value);

    edit::set_validated(
        &mut file,
        &format!("{selector}.{property}"),
        parsed,
        &edit::styles_check(strict),
    )
    .map_err(|d| wire_all(&d))?;
    file.save().map_err(|d| vec![WireDiagnostic::from(&d)])?;

    Ok(project_at(root))
}

/// Remove one style property, so the cascade decides it again (STY-005).
#[tauri::command]
fn reset_style(
    session: tauri::State<'_, Arc<Session>>,
    root: String,
    selector: String,
    property: String,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let root = Utf8PathBuf::from(root);
    let project = clear_style(&root, &selector, &property)?;
    forget_changes(&session, &root);
    Ok(project)
}

/// [`reset_style`] without the Tauri binding.
pub fn clear_style(
    root: &Utf8Path,
    selector: &str,
    property: &str,
) -> Result<WireProject, Vec<WireDiagnostic>> {
    let mut file = open_styles_file(root).map_err(|d| vec![WireDiagnostic::from(&d)])?;
    if file.reset(&format!("{selector}.{property}")) {
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
    // A proof rather than the publication (P5.4). An argument and not a
    // setting: it is what this one run is, and a project that remembered it
    // was drafting would eventually ship a stamped book.
    draft: bool,
    // Run the backend even if nothing that reaches it has changed (BLD-007).
    clean: bool,
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
        let mut request = BuildRequest::new(root, opened.output())
            .keeping_intermediates(*opened.settings.output.keep_intermediates)
            .with_settings(opened.settings.clone())
            .with_styles(opened.styles.clone());
        request.clean = clean;
        // Handed to the build rather than only announced, so a project that
        // cannot be opened cannot be built (SRS §16.2 scenario H).
        request.prior = opened.diagnostics.clone();
        if draft {
            request.draft = Some(biblecompose_app::draft_note(opened.document.books.len()));
        }

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
fn open_settings_file(root: &Utf8Path) -> Result<TomlFile, AppDiagnostic> {
    open_or_create(
        &root.join(project::SETTINGS_FILE),
        &TomlFile::settings_header(SCHEMA_VERSION),
    )
}

/// The project's style sheet, likewise.
fn open_styles_file(root: &Utf8Path) -> Result<TomlFile, AppDiagnostic> {
    open_or_create(&root.join(project::STYLES_FILE), &TomlFile::styles_header())
}

fn open_or_create(path: &Utf8Path, header: &str) -> Result<TomlFile, AppDiagnostic> {
    if path.exists() {
        Ok(TomlFile::new(ConfigDocument::read(path)?))
    } else {
        Ok(TomlFile::create(path.to_owned(), header))
    }
}

/// Which control a style property needs.
///
/// Small enough to state here, and stating it is what lets the frontend send
/// only the text a field holds. Anything not named is text — a length, a page
/// size, an alignment.
fn style_kind(property: &str) -> form::Kind {
    match property {
        "weight" => form::Kind::Integer,
        "italic" | "smallcaps" => form::Kind::Boolean,
        "font_family" => form::Kind::Font,
        // Colour is text on the wire — `#c81414` is a string in TOML, and the
        // reader in `biblecompose-config` is the one thing that decides
        // whether it is a colour.
        _ => form::Kind::Text,
    }
}

fn not_a_property(property: &str) -> AppDiagnostic {
    AppDiagnostic::error(
        biblecompose_diagnostics::code::UNKNOWN_PROPERTY,
        format!("`{property}` is not a style property"),
    )
    .help(format!("the properties are: {}", PROPERTIES.join(", ")))
}

/// Every resolved style, flattened for the window.
fn wire_styles(styles: &biblecompose_config::ResolvedStyles) -> Vec<WireStyle> {
    styles
        .iter()
        .map(|(selector, resolved)| WireStyle {
            selector: selector.key(),
            properties: wire_style_properties(resolved),
        })
        // Unfiltered, including the selectors nothing has set. STY-008 asks
        // what each property of an element is *and where it came from*, and
        // "nothing decides this" is an answer — the one a publisher wondering
        // why a paragraph looks like body text is looking for. The emitter's
        // own list is filtered separately, because an empty rule in the
        // document is a line of XML that says nothing.
        .collect()
}

fn wire_style_properties(resolved: &biblecompose_config::ResolvedStyle) -> Vec<WireStyleProperty> {
    let s = &resolved.style;
    let values: [(&'static str, Option<String>); PROPERTIES.len()] = [
        ("font_family", s.font_family.clone()),
        ("font_size", s.font_size.map(|l| l.to_string())),
        ("weight", s.weight.map(|w| w.to_string())),
        ("italic", s.italic.map(|b| b.to_string())),
        ("smallcaps", s.smallcaps.map(|b| b.to_string())),
        ("space_above", s.space_above.map(|l| l.to_string())),
        ("space_below", s.space_below.map(|l| l.to_string())),
        ("indent", s.indent.map(|l| l.to_string())),
        ("raise", s.raise.map(|l| l.to_string())),
        ("align", s.align.map(|a| a.as_str().to_owned())),
        ("color", s.color.map(|c| c.to_string())),
    ];

    values
        .into_iter()
        .filter_map(|(name, value)| {
            let value = value?;
            let origin = resolved.origin_of(name);
            Some(WireStyleProperty {
                name,
                value,
                origin: match origin {
                    Some(Origin::File(_)) => "file",
                    Some(Origin::Inherited { .. }) => "inherited",
                    _ => "builtin",
                },
                from: match origin {
                    Some(Origin::Inherited { from }) => Some(from.key()),
                    _ => None,
                },
                location: origin.and_then(Origin::location).map(|loc| WireLocation {
                    path: loc.path.to_string(),
                    line: loc.line,
                    column: loc.column,
                }),
            })
        })
        .collect()
}

/// Everything the window shows about a project folder.
pub fn project_at(root: &Utf8Path) -> WireProject {
    wire_project(root, project::open(root))
}

/// The wire form of a project already opened.
///
/// Split out because [`observe`] needs the [`project::Opened`] itself as well
/// as its wire form, and opening the folder twice to get both would parse
/// every book twice.
fn wire_project(root: &Utf8Path, opened: project::Opened) -> WireProject {
    let mut books: Vec<WireBook> = opened
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
                included: true,
                testament: book.code.testament().as_str(),
            }
        })
        .collect();

    // The ones left out, put back where they sit in the order. Ascending, so
    // each insertion lands before the later ones shift under it — which
    // reconstructs exactly the list the plan produced before selection.
    for out in &opened.left_out {
        let at = out.position.min(books.len());
        books.insert(
            at,
            WireBook {
                code: out.code.as_str().to_owned(),
                // The canon's name: there is no `\h` to read, because a book
                // that is out is deliberately never parsed.
                name: out.code.english_name().to_owned(),
                path: out.path.to_string(),
                chapters: 0,
                errors: 0,
                warnings: 0,
                included: false,
                testament: out.code.testament().as_str(),
            },
        );
    }

    // Canonical position, from the canon table rather than from the plan:
    // this is the answer `books.order` is compared against.
    let mut canonical_order: Vec<(u16, String)> = books
        .iter()
        .filter_map(|b| {
            biblecompose_scripture::BookCode::parse(&b.code).map(|c| (c.order(), b.code.clone()))
        })
        .collect();
    canonical_order.sort();

    WireProject {
        root: root.to_string(),
        books,
        canonical_order: canonical_order.into_iter().map(|(_, code)| code).collect(),
        geometry: WireGeometry::of(&opened.settings),
        diagnostics: wire_all(&opened.diagnostics),
        settings: opened
            .settings
            .fields()
            .into_iter()
            .map(wire_field)
            .collect(),
        styles: wire_styles(&opened.styles),
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
        choices: f.kind.choices(),
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
        BuildEvent::LogFile(path) => WireBuildEvent::LogFile {
            path: path.to_string(),
        },
        BuildEvent::Pages { done, expected } => WireBuildEvent::Pages { done, expected },
    }
}

/// How much of the screen the window takes, before a project and after one.
///
/// A launcher and a workspace are different windows wearing one frame. The
/// start screen is two buttons and a short list, and eight tenths of a large
/// monitor to hold them is a lot of nothing; a project is a book list, a
/// diagnostics panel and a page of settings meant to be read at once.
const OPENING_SHARE: f64 = 0.5;
const WORKING_SHARE: f64 = 0.8;

/// The widest this window is allowed to be, in logical pixels.
///
/// A share of the screen stops making sense past a point. Eight tenths of an
/// ultrawide monitor is nearly three thousand pixels of window holding a book
/// list, a page of settings and a great deal of nothing — and a line of type
/// that long is unreadable even when it is only a diagnostic. This is a
/// measure, in the typographic sense, rather than a preference.
///
/// Logical and not physical pixels: on a display at 150% this is 1500 of the
/// pixels the layout inside is written in, which is the number that decides
/// whether the two panes fit side by side.
const MAX_WIDTH: f64 = 1500.0;

/// A share of the screen's work area, in physical pixels.
///
/// The *work area* rather than the whole monitor, so a share of the screen
/// means a share of the part not already spoken for by a taskbar.
fn share_of_screen(window: &tauri::WebviewWindow, share: f64) -> Option<tauri::PhysicalSize<u32>> {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => Some(m),
        // A window that has not been shown yet may belong to no monitor.
        _ => window.primary_monitor().ok().flatten(),
    }?;

    let area = monitor.work_area().size;
    let cap = MAX_WIDTH * monitor.scale_factor();
    Some(tauri::PhysicalSize::new(
        (f64::from(area.width) * share).min(cap).round() as u32,
        (f64::from(area.height) * share).round() as u32,
    ))
}

/// Resize to a share of the screen and centre what is left.
///
/// A size computed here rather than a number in `tauri.conf.json`, because the
/// config takes only absolute pixels and a number chosen at packaging time is
/// a number chosen for somebody else's monitor.
///
/// Everything is best-effort. A monitor that cannot be identified leaves the
/// configured size in place, which is a reasonable window, and the configured
/// minimum still floors it on a small screen.
fn take_share_of_screen(window: &tauri::WebviewWindow, share: f64) {
    let Some(size) = share_of_screen(window, share) else {
        return;
    };
    let _ = window.set_size(size);
    // The remainder, split evenly, rather than wherever the previous size
    // happened to leave it.
    let _ = window.center();
}

/// What the title bar says.
///
/// The window's own chrome is where an application says which one it is and
/// which document it is holding — which is why the folder name is here and no
/// longer inside the page. Two places saying it is one place too many, and the
/// one the operating system puts in the task bar is the one that identifies
/// this window among others.
fn name_the_window(app: &tauri::AppHandle, root: Option<&Utf8Path>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let base = concat!("BibleCompose ", env!("CARGO_PKG_VERSION"));
    // The whole path, not the folder's name. Two projects called `usfm` are
    // the ordinary case — one per translation, each in its own tree — and the
    // task bar is where you tell two windows apart.
    let title = match root {
        Some(root) => format!("{base} - {root}"),
        None => base.to_owned(),
    };
    let _ = window.set_title(&title);
}

/// Grow from launcher to workspace when a project is opened.
fn grow_to_workspace(app: &tauri::AppHandle) {
    resize_between(app, OPENING_SHARE, WORKING_SHARE);
}

/// And back, when it is put down. The same rule in reverse.
fn shrink_to_launcher(app: &tauri::AppHandle) {
    resize_between(app, WORKING_SHARE, OPENING_SHARE);
}

/// Move from one share of the screen to the other — but only from the size
/// this application set.
///
/// Somebody who has sized the window by hand has said what they want it to be,
/// and opening or closing a project is not new information about that. So the
/// window moves if it is still where the application put it, and is left alone
/// otherwise. That also makes each move happen once without remembering it:
/// after growing, the window is no longer at the opening size.
fn resize_between(app: &tauri::AppHandle, from: f64, to: f64) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let (Some(expected), Ok(current)) = (share_of_screen(&window, from), window.inner_size())
    else {
        return;
    };

    // A few pixels of tolerance: the size that comes back has been through a
    // rounding and a window manager since it was set.
    let same = |a: u32, b: u32| a.abs_diff(b) <= 8;
    if same(current.width, expected.width) && same(current.height, expected.height) {
        take_share_of_screen(&window, to);
    }
}

/// Build and run the window.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(Arc::new(Session::default()));
            if let Some(window) = app.get_webview_window("main") {
                // Enforced by the window manager as well as by the arithmetic
                // above, so dragging the edge cannot go past it either.
                let _ = window.set_max_size(Some(tauri::LogicalSize::new(MAX_WIDTH, 100_000.0)));
                take_share_of_screen(&window, OPENING_SHARE);
            }
            name_the_window(app.handle(), None);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            versions,
            defaults,
            open_project,
            changed_files,
            set_setting,
            reset_setting,
            set_style,
            reset_style,
            fonts,
            recent_projects,
            forget_project,
            create_project,
            close_project,
            open_folder,
            open_pdf,
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
