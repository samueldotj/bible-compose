//! Opening a book's USFM in easy-usfm.
//!
//! The two applications already share the engine that reads USFM
//! ([ADR-001](../../../docs/adr/001-usfm-core.md)), so a diagnostic means the
//! same thing in both. What was missing is the short way between them: a
//! publisher who sees a warning against Genesis in the book list had to find
//! the file themselves and open it in whatever was to hand.
//!
//! So the editor is carried inside this application and started on the file.
//!
//! # Where the editor comes from
//!
//! Three answers, in this order, and the first two exist for the reasons
//! SILE's do:
//!
//! 1. **`BIBLECOMPOSE_EDITOR`**, so a developer can point at a build without
//!    rebuilding the bundle. Highest priority, because that is what an
//!    override means.
//! 2. **The bundle**, unpacked once into the cache directory beside the
//!    typesetter. This is what a shipped build uses and the reason the feature
//!    needs nothing installed.
//! 3. **Beside this executable**, which is what a `cargo build` of this
//!    workspace has: no bundle is embedded by default, and a development
//!    window should still be able to open a book.
//!
//! # Why here rather than in `biblecompose-app`
//!
//! The same argument that put the HTTP client here (ARCHITECTURE §7.6): the
//! shell is the layer that already owns dialogs, the filesystem and starting
//! other programs. A build that composes a PDF has no business being able to
//! start a process, and keeping that out of `biblecompose-app` makes it a fact
//! about the dependency graph rather than a promise.

use biblecompose_diagnostics::{code, Diagnostic};
use camino::{Utf8Path, Utf8PathBuf};

/// Point this at an `easy-usfm-tauri` to use it instead of the bundled one.
pub const EDITOR_ENV: &str = "BIBLECOMPOSE_EDITOR";

/// What the editor is called inside the bundle and beside this executable.
pub const EXE_NAME: &str = if cfg!(windows) {
    "easy-usfm-tauri.exe"
} else {
    "easy-usfm-tauri"
};

/// The embedded editor, when one was compiled in.
///
/// A folder rather than a file because it is only one file on Windows: macOS
/// wants an `.app` and Linux an AppImage beside its libraries, and a tree
/// extracts the same way whatever is in it.
#[cfg(feature = "embedded-editor")]
mod bundled {
    use rust_embed::Embed;

    #[derive(Embed)]
    #[folder = "$BIBLECOMPOSE_EDITOR_BUNDLE"]
    pub struct Bundle;

    /// Present in the placeholder directory and in no real bundle.
    pub const PLACEHOLDER_MARKER: &str = "PLACEHOLDER";
}

/// Where the unpacked editor lives.
///
/// Beside the unpacked typesetter, in the cache directory ARCHITECTURE §4
/// already accounts for — this is a copy of something shipped, not anything a
/// publisher would miss, and deleting it costs one unpack.
///
/// Keyed by this application's version *and* the bundle's size, because a
/// development build changes what is embedded without changing the version,
/// and an editor that is one release behind the window that started it is the
/// kind of thing nobody would think to check.
#[cfg(feature = "embedded-editor")]
fn unpack_root(bytes: usize) -> Option<Utf8PathBuf> {
    Some(
        cache_root()?
            .join("editor")
            .join(format!("{}-{bytes}", env!("CARGO_PKG_VERSION"))),
    )
}

#[cfg(feature = "embedded-editor")]
fn cache_root() -> Option<Utf8PathBuf> {
    let root = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
    }?;
    Utf8PathBuf::from_path_buf(root)
        .ok()
        .map(|r| r.join("biblecompose"))
}

/// Unpack the embedded editor if it is not already there.
///
/// Written to a temporary name and renamed into place, so a window killed
/// half way through leaves no half-written executable for the next one to
/// start. Two windows unpacking at once is the ordinary case and not a race
/// worth locking for: both write the same bytes, and the loser's rename is
/// the one that fails.
#[cfg(feature = "embedded-editor")]
fn unpack() -> Result<Utf8PathBuf, Diagnostic> {
    use bundled::{Bundle, PLACEHOLDER_MARKER};

    if Bundle::get(PLACEHOLDER_MARKER).is_some() {
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            "this build was compiled to carry the editor but without an editor to carry",
        )
        .help(
            "rebuild with BIBLECOMPOSE_EDITOR_BUNDLE pointing at a stage, \
             or drop the embedded-editor feature",
        ));
    }

    let files: Vec<(String, Vec<u8>)> = Bundle::iter()
        .filter_map(|p| Bundle::get(&p).map(|f| (p.into_owned(), f.data.into_owned())))
        .collect();
    let total: usize = files.iter().map(|(_, data)| data.len()).sum();

    let root = unpack_root(total).ok_or_else(|| {
        Diagnostic::error(
            code::NOT_FOUND,
            "there is nowhere to unpack the editor: no cache directory",
        )
        .help(format!("set {EDITOR_ENV} to an easy-usfm build instead"))
    })?;
    let exe = root.join(EXE_NAME);
    if exe.is_file() {
        return Ok(exe);
    }

    let staging = root.with_extension("partial");
    let _ = std::fs::remove_dir_all(staging.as_std_path());
    for (path, data) in &files {
        let out = staging.join(path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent.as_std_path()).map_err(|e| unpack_failed(&out, e))?;
        }
        std::fs::write(out.as_std_path(), data).map_err(|e| unpack_failed(&out, e))?;
        #[cfg(unix)]
        if path.ends_with(EXE_NAME) {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                out.as_std_path(),
                std::fs::Permissions::from_mode(0o755),
            );
        }
    }
    if let Some(parent) = root.parent() {
        let _ = std::fs::create_dir_all(parent.as_std_path());
    }
    // Already there means another window won, which is the outcome either way.
    if std::fs::rename(staging.as_std_path(), root.as_std_path()).is_err() && !exe.is_file() {
        let _ = std::fs::remove_dir_all(staging.as_std_path());
        return Err(Diagnostic::error(
            code::COULD_NOT_CREATE,
            format!("the editor could not be unpacked into {root}"),
        ));
    }
    Ok(exe)
}

#[cfg(feature = "embedded-editor")]
fn unpack_failed(path: &Utf8Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::error(
        code::COULD_NOT_CREATE,
        format!("the editor could not be unpacked: {path}: {error}"),
    )
}

/// The editor this window should start, or why there is not one.
pub fn locate() -> Result<Utf8PathBuf, Diagnostic> {
    if let Some(set) = std::env::var_os(EDITOR_ENV) {
        let path = Utf8PathBuf::from(set.to_string_lossy().into_owned());
        if path.is_file() {
            return Ok(path);
        }
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            format!("{EDITOR_ENV} points at {path}, which is not a file that exists"),
        )
        .help("unset it to use the editor this application carries"));
    }

    #[cfg(feature = "embedded-editor")]
    {
        return unpack();
    }

    #[cfg(not(feature = "embedded-editor"))]
    {
        beside_us().ok_or_else(|| {
            Diagnostic::error(
                code::NOT_FOUND,
                "this build does not carry the USFM editor",
            )
            .help(format!(
                "build with the embedded-editor feature, or set {EDITOR_ENV} to an \
                 easy-usfm build — in this workspace that is \
                 target/debug/{EXE_NAME} of the easy-usfm checkout"
            ))
        })
    }
}

/// An editor sitting next to this executable, which is what a development
/// build of the two workspaces side by side has.
#[cfg(not(feature = "embedded-editor"))]
fn beside_us() -> Option<Utf8PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = Utf8PathBuf::from_path_buf(exe.parent()?.to_path_buf()).ok()?;
    let candidate = dir.join(EXE_NAME);
    candidate.is_file().then_some(candidate)
}

/// The editors this window started, kept so they can be closed with it.
///
/// The `Child` handles and not their process ids. On Windows a live handle
/// reserves the id, so what this holds cannot quietly become some other
/// program that started after an editor was closed — which is the way a list
/// of bare numbers goes wrong, and it goes wrong by shutting down something
/// nobody was talking about.
#[derive(Default)]
pub struct Started(std::sync::Mutex<Vec<std::process::Child>>);

/// Start the editor on one file.
///
/// One window per file, which is what easy-usfm is for — its own README calls
/// it an editor for individual USFM files, and a second window is cheaper than
/// deciding what should happen to unsaved work in the first.
///
/// Not waited on: the editor runs for as long as somebody is editing, and this
/// window has a build to get on with.
pub fn open(file: &Utf8Path, started: &Started) -> Result<(), Diagnostic> {
    if !file.is_file() {
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            format!("{file} is not a file that exists"),
        )
        .help("the book may have been moved or renamed since the project was opened"));
    }
    let editor = locate()?;
    let child = std::process::Command::new(editor.as_std_path())
        .arg(file.as_std_path())
        .spawn()
        .map_err(|e| {
            Diagnostic::error(
                code::COULD_NOT_CREATE,
                format!("the USFM editor could not be started: {e}"),
            )
            .help(format!("it was looked for at {editor}"))
        })?;

    if let Ok(mut open) = started.0.lock() {
        open.push(child);
    }
    Ok(())
}

/// Ask every editor this window started to close, and let it answer.
///
/// **Asked, not killed, and the difference is somebody's work.** easy-usfm is
/// a text editor: it guards its window against closing with unsaved changes,
/// and a request to close is what lets that guard run. `Child::kill` is
/// `TerminateProcess` on Windows and `SIGKILL` on Unix — neither can be caught,
/// so either would discard whatever had been typed since the last save without
/// asking, in a window the publisher may not even have been looking at.
///
/// So: `taskkill` without `/F`, which posts a close to the process's windows,
/// and `SIGTERM` elsewhere. An editor with unsaved work puts its question up
/// and stays until it is answered; one without simply goes. Nothing is forced
/// afterwards, because "it would not close" is a decision the person made.
///
/// Best-effort throughout, and silent: this runs while the application is
/// already on its way out, where there is nowhere left to report anything.
pub fn close_started(started: &Started) {
    let Ok(mut open) = started.0.lock() else { return };
    for child in open.iter_mut() {
        // Already gone is the ordinary case — most people close the editor
        // themselves — and asking a finished process to close would be asking
        // an id that no longer means anything.
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
            continue;
        }
        let pid = child.id();
        let _ = if cfg!(windows) {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        } else {
            std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
        };
    }
    open.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises the tests that reach for `BIBLECOMPOSE_EDITOR`.
    ///
    /// It is one variable for the whole process and the harness runs tests on
    /// threads, so without this the override test's deliberately wrong path
    /// was what the launch test read — and the launch test failed on a defect
    /// belonging to the suite rather than to the code. Poison is stepped over:
    /// a test that panicked holding it has already failed, and taking the rest
    /// down after it reports nothing new.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// **The override wins, and a bad one is reported rather than ignored.**
    ///
    /// The same bargain SILE's override makes: it exists so a developer can
    /// point at a newer build, which it would not do if a typo silently fell
    /// through to the bundled copy.
    #[test]
    fn the_override_is_honoured_and_a_wrong_one_is_named() {
        let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().expect("temp dir");
        let fake = Utf8PathBuf::from_path_buf(dir.path().join(EXE_NAME)).expect("UTF-8");
        std::fs::write(fake.as_std_path(), b"not really an editor").expect("write");

        let previous = std::env::var_os(EDITOR_ENV);
        std::env::set_var(EDITOR_ENV, fake.as_str());
        let found = locate();
        std::env::set_var(EDITOR_ENV, "/definitely/not/here/easy-usfm");
        let missing = locate();
        match previous {
            Some(v) => std::env::set_var(EDITOR_ENV, v),
            None => std::env::remove_var(EDITOR_ENV),
        }

        assert_eq!(found.expect("the override"), fake);
        let err = missing.expect_err("a missing override is an error");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(err.message.contains(EDITOR_ENV), "{}", err.message);
    }

    /// **The carried editor unpacks, and the second look is the warm one.**
    ///
    /// Only compiled when the feature is on, and it asserts whichever of the
    /// two shapes that build has: a real stage unpacks to a runnable
    /// executable, and the placeholder — which is what `cargo build
    /// --features embedded-editor` gets without a stage, so that the feature
    /// type-checks in CI — says so in a sentence rather than unpacking
    /// nothing and failing later at a launch.
    #[cfg(feature = "embedded-editor")]
    #[test]
    fn the_carried_editor_unpacks_and_is_then_found_warm() {
        match unpack() {
            Ok(exe) => {
                assert!(exe.is_file(), "{exe} should be a file");
                let size = std::fs::metadata(exe.as_std_path()).expect("metadata").len();
                assert!(size > 1_000_000, "{exe} is only {size} bytes");
                assert!(exe.as_str().ends_with(EXE_NAME), "{exe}");
                // Again: the same answer, and nothing rewritten.
                assert_eq!(unpack().expect("the warm path"), exe);
            }
            Err(e) => {
                assert!(
                    e.message.contains("without an editor to carry"),
                    "a build with no stage should say so: {}",
                    e.message
                );
            }
        }
    }

    /// **An editor this window started closes when asked.**
    ///
    /// The whole mechanism, against a real editor, because the interesting
    /// part is not the bookkeeping: `taskkill` without `/F` posts a close to a
    /// process's *windows*, and a process without one refuses it. easy-usfm has
    /// a window; a stand-in would have proved nothing.
    ///
    /// Skipped, loudly, unless `BIBLECOMPOSE_EDITOR` names one, because it
    /// opens a window for a moment and a test suite should not do that on a
    /// machine that did not ask for it.
    #[test]
    fn an_editor_this_window_started_closes_when_asked() {
        let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let Some(set) = std::env::var_os(EDITOR_ENV) else {
            eprintln!("skipping: {EDITOR_ENV} is not set");
            return;
        };
        if !Utf8PathBuf::from(set.to_string_lossy().into_owned()).is_file() {
            eprintln!("skipping: {EDITOR_ENV} does not name a file");
            return;
        }

        let dir = tempfile::tempdir().expect("temp dir");
        let book = Utf8PathBuf::from_path_buf(dir.path().join("01_GEN.usfm")).expect("UTF-8");
        std::fs::write(book.as_std_path(), "\\id GEN\n\\c 1\n\\p\n\\v 1 In the beginning.\n")
            .expect("write");

        let started = Started::default();
        open(book.as_path(), &started).expect("the editor starts");

        // It is ours and it is running.
        {
            let mut open = started.0.lock().expect("the lock");
            assert_eq!(open.len(), 1);
            assert!(
                matches!(open[0].try_wait(), Ok(None)),
                "the editor should still be running"
            );
        }

        // Give the window time to exist: a close posted before there is one to
        // post it to is a close that goes nowhere.
        std::thread::sleep(std::time::Duration::from_secs(6));
        close_started(&started);

        // And it goes. Polled rather than slept at, because a window closing
        // is not instant and the time it takes is not this test's to assert.
        let mut gone = false;
        for _ in 0..40 {
            if std::process::Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq easy-usfm-tauri.exe"])
                .output()
                .map(|o| !String::from_utf8_lossy(&o.stdout).contains("easy-usfm-tauri"))
                .unwrap_or(false)
            {
                gone = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        assert!(gone, "the editor was asked to close and did not");
        assert!(
            started.0.lock().expect("the lock").is_empty(),
            "the list should be emptied once they have been asked"
        );
    }

    /// A book that is not there is said so before anything is started.
    #[test]
    fn a_file_that_is_not_there_is_refused_without_launching_anything() {
        let err = open(
            Utf8Path::new("/definitely/not/here/01_GEN.usfm"),
            &Started::default(),
        )
        .expect_err("a missing file is an error");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(err.message.contains("01_GEN.usfm"), "{}", err.message);
    }
}
