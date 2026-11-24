//! Opening a book's USFM in easy-usfm.
//!
//! The two applications already share the engine that reads USFM
//! ([ADR-001](../../../docs/adr/001-usfm-core.md)), so a diagnostic means the
//! same thing in both. What was missing is the short way between them: a
//! publisher who sees a warning against Genesis in the book list had to find
//! the file themselves and open it in whatever was to hand.
//!
//! # A window, not a program
//!
//! The editor is a window in this process
//! ([ADR-007](../../../docs/adr/007-editor-in-process.md)). easy-usfm's
//! desktop shell is a Tauri plugin, `tauri-plugin-easy-usfm`, registered by
//! this application; its interface is staged into `editor/` beside this
//! application's own (`scripts/stage-editor.mjs`). Opening a book opens an
//! `editor-…` window on it, or brings forward the one already open on it.
//!
//! What that replaced is worth a line, because each piece was a defect waiting
//! somewhere: a 9 MB executable embedded in this one, unpacked into the cache
//! on first use, started as a second process, and shut down at exit by asking
//! the operating system to close its windows. Now there is one binary, nothing
//! to unpack, and closing the editors is closing windows.
//!
//! # What the editor may do
//!
//! The plugin is registered book-only: a window opened on Genesis opens
//! Genesis, offers no New, Open or Open Recent, and the shell refuses any other
//! path whatever the page asks for. Its recovery snapshots are kept in
//! `editor-recovery/`, apart from those of easy-usfm's own window. And it is
//! granted none of this application's commands (`capabilities/editor.json`).

use biblecompose_diagnostics::{code, Diagnostic};
use camino::Utf8Path;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl};

/// Where the editor's interface is served from, inside this application's
/// own assets.
pub const EDITOR_PAGE: &str = "editor/index.html";

/// Where, in the application data directory, the editor keeps what it needs
/// to give back unsaved work after a crash. Its own folder, so it is never
/// confused with easy-usfm's.
pub const RECOVERY_FOLDER: &str = "editor-recovery";

/// The plugin, configured for this application.
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_easy_usfm::Builder::new()
        .book_only()
        .recovery_folder(RECOVERY_FOLDER)
        .build()
}

/// Open the editor on one file, or bring forward the window already on it.
pub fn open<R: Runtime>(app: &AppHandle<R>, file: &Utf8Path) -> Result<(), Diagnostic> {
    exists(file)?;

    // Without a stage the window would open onto a page that is not there —
    // Vite's 404 in development, a blank one in a build — which says nothing
    // about why.
    if !staged(app) {
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            "this build does not carry the USFM editor",
        )
        .help("stage it first: node scripts/stage-editor.mjs <easy-usfm checkout>"));
    }

    tauri_plugin_easy_usfm::open_editor(
        app,
        file.as_std_path(),
        WebviewUrl::App(EDITOR_PAGE.into()),
    )
    .map(|_| ())
    .map_err(|e| {
        Diagnostic::error(
            code::COULD_NOT_CREATE,
            format!("the USFM editor could not be opened: {e}"),
        )
    })
}

/// Whether the editor's page is there to open.
///
/// A built application looks in its own assets. A development window has
/// none — Vite serves `public/` as it stands — so it looks there, in the
/// checkout this binary was built from.
fn staged<R: Runtime>(app: &AppHandle<R>) -> bool {
    if tauri::is_dev() {
        Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../public")
            .join(EDITOR_PAGE)
            .is_file()
    } else {
        app.asset_resolver().get(EDITOR_PAGE.to_string()).is_some()
    }
}

/// Remove the editor 0.3.0 to 0.4.3 unpacked, if it is still there.
///
/// Those releases carried easy-usfm as an executable and unpacked it into
/// `editor/` in the cache, one folder per version — about 9 MB each, and
/// nothing reads them now. Only that folder: the typesetter beside it is
/// still this application's. Best-effort and silent, like any cache.
pub fn forget_unpacked_editor() {
    if let Some(cache) = cache_root() {
        forget_unpacked_editor_in(&cache);
    }
}

fn forget_unpacked_editor_in(cache: &Utf8Path) {
    let old = cache.join("editor");
    if old.is_dir() {
        let _ = std::fs::remove_dir_all(old.as_std_path());
    }
}

/// `biblecompose` in the platform's cache directory: where the releases that
/// unpacked an editor put it.
fn cache_root() -> Option<camino::Utf8PathBuf> {
    let root = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache"))
            })
    }?;
    camino::Utf8PathBuf::from_path_buf(root)
        .ok()
        .map(|r| r.join("biblecompose"))
}

/// A book that is not there is said so before anything is opened.
fn exists(file: &Utf8Path) -> Result<(), Diagnostic> {
    if file.is_file() {
        return Ok(());
    }
    Err(
        Diagnostic::error(code::NOT_FOUND, format!("{file} is not a file that exists"))
            .help("the book may have been moved or renamed since the project was opened"),
    )
}

/// Ask every editor window to close, and let each answer.
///
/// **Asked, not destroyed, and the difference is somebody's work.** `close`
/// raises the window's close request, which is where easy-usfm guards unsaved
/// changes: an editor with none simply goes, and one with some puts its
/// question up and stays until it is answered. Nothing is forced afterwards,
/// because "it would not close" is a decision the person made — and since an
/// open window keeps the application running, their work is still there.
pub fn close_all<R: Runtime>(app: &AppHandle<R>) {
    for label in tauri_plugin_easy_usfm::editor_labels(app) {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page the editor is opened on is the one the stage script writes.
    #[test]
    fn the_editor_page_is_where_the_stage_puts_it() {
        let script = include_str!("../../../scripts/stage-editor.mjs");
        assert!(
            script.contains(r#"join("public", "editor")"#),
            "stage-editor.mjs should stage into public/editor"
        );
        assert_eq!(EDITOR_PAGE, "editor/index.html");
    }

    /// Its recovery is its own, never easy-usfm's `recovery/`.
    #[test]
    fn the_editors_recovery_is_kept_apart() {
        assert_ne!(RECOVERY_FOLDER, "recovery");
    }

    /// **The declared commands are the handled ones.** `build.rs` declares
    /// this application's commands so an editor window can be refused them,
    /// and a command handled but not declared would be refused to `main` as
    /// well — at run time, as a click that does nothing.
    #[test]
    fn the_declared_commands_are_the_handled_ones() {
        let build = include_str!("../build.rs");
        let lib = include_str!("lib.rs");
        let handled = lib
            .split("tauri::generate_handler![")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("lib.rs has a handler list");
        let mut handled: Vec<&str> = handled
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let declared = build
            .split("const COMMANDS: &[&str] = &[")
            .nth(1)
            .and_then(|rest| rest.split("];").next())
            .expect("build.rs has a command list");
        let mut declared: Vec<&str> = declared
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        handled.sort_unstable();
        declared.sort_unstable();
        assert_eq!(handled, declared);

        let main = include_str!("../capabilities/default.json");
        for command in &declared {
            let grant = format!("\"allow-{}\"", command.replace('_', "-"));
            assert!(main.contains(&grant), "main is not granted {command}");
        }
        let editor = include_str!("../capabilities/editor.json");
        assert!(
            !editor.contains("\"allow-"),
            "an editor window is granted none of this application's commands"
        );
    }

    /// **The editor does no update check of its own here** (ADR-008).
    ///
    /// easy-usfm's own application turns the plugin's `updater` feature on and
    /// asks its user on first run; inside BibleCompose the editor is updated
    /// with this application, by the one check a person clicks for, and must
    /// not ask a second question or make a second request. Two locks, both
    /// asserted: the feature is off in the graph this crate is actually built
    /// with — so a dependency that turned it on through feature unification
    /// fails here, not in a publisher's window — and the editor window is
    /// denied the command even so.
    #[test]
    fn the_editor_brings_no_update_check_of_its_own() {
        assert!(
            !tauri_plugin_easy_usfm::updates::updates_possible(),
            "tauri-plugin-easy-usfm was built with its `updater` feature on — \
             something in this workspace enabled it, and the editor would offer \
             BibleCompose's users a second update check"
        );
        let editor = include_str!("../capabilities/editor.json");
        assert!(
            editor.contains("\"easy-usfm:deny-check-for-update\""),
            "the editor window should be denied easy-usfm's update check"
        );
    }

    /// **The old editor goes and the typesetter stays.** They share the cache
    /// folder, and removing the wrong one costs a publisher an unpack.
    #[test]
    fn only_the_old_editor_is_removed_from_the_cache() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8");
        let old = cache.join("editor").join("0.4.3-9322496");
        std::fs::create_dir_all(old.as_std_path()).expect("old editor");
        std::fs::write(old.join("easy-usfm-tauri.exe").as_std_path(), b"x").expect("write");
        let sile = cache.join("sile").join("abc");
        std::fs::create_dir_all(sile.as_std_path()).expect("typesetter");

        forget_unpacked_editor_in(&cache);
        assert!(
            !cache.join("editor").exists(),
            "the old editor should be gone"
        );
        assert!(sile.is_dir(), "the typesetter must be left alone");

        // And a cache with no old editor is nothing to do.
        forget_unpacked_editor_in(&cache);
    }

    /// A book that is not there is said so before anything is opened.
    #[test]
    fn a_file_that_is_not_there_is_refused_without_opening_anything() {
        let err = exists(Utf8Path::new("/definitely/not/here/01_GEN.usfm"))
            .expect_err("a missing file is an error");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(err.message.contains("01_GEN.usfm"), "{}", err.message);
    }
}
