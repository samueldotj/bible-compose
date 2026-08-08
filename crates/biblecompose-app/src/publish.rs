//! Build directories and atomic publication.
//!
//! BLD-009 says a failed build must not replace the last known good PDF. The
//! way to guarantee that is not to check afterwards but to **never let the
//! backend write to the destination**: every build runs in a scratch directory
//! and the finished PDF is moved into place only after it exists and is
//! non-empty. The guarantee is then structural — a failure cannot replace the
//! output because it never had access to it.

use std::io::ErrorKind;

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};

/// A scratch directory owned by one build.
///
/// Lives under the project's own cache directory rather than the system
/// temporary directory, so the final move is a rename within one filesystem
/// rather than a copy across two — which is what makes publication atomic.
#[derive(Debug)]
pub struct BuildDir {
    path: Utf8PathBuf,
    keep: bool,
}

impl BuildDir {
    pub fn create(project_root: &Utf8Path, id: &str, keep: bool) -> Result<Self, Diagnostic> {
        let path = project_root.join(".biblecompose").join("build").join(id);
        std::fs::create_dir_all(path.as_std_path()).map_err(|e| {
            Diagnostic::error(
                code::DESTINATION_UNWRITABLE,
                "could not create the build directory",
            )
            .at(SourceLoc::file(path.clone()))
            .detail(e.to_string())
        })?;
        Ok(BuildDir { path, keep })
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    /// SILE-008: intermediates are removed after a successful build unless the
    /// user asked to keep them.
    pub fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for BuildDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(self.path.as_std_path());
        }
    }
}

/// Move a finished PDF into its destination, atomically.
///
/// Refuses to publish something that does not exist or is empty, so
/// "the backend said it succeeded" is never taken on trust (BLD-002).
pub fn publish(from: &Utf8Path, to: &Utf8Path) -> Result<(), Diagnostic> {
    let meta = std::fs::metadata(from.as_std_path()).map_err(|e| {
        Diagnostic::error(code::NO_OUTPUT, "the build produced no output file")
            .at(SourceLoc::file(from.to_owned()))
            .detail(e.to_string())
    })?;
    if meta.len() == 0 {
        return Err(
            Diagnostic::error(code::NO_OUTPUT, "the build produced an empty output file")
                .at(SourceLoc::file(from.to_owned())),
        );
    }

    if let Some(parent) = to.parent() {
        if !parent.as_str().is_empty() {
            std::fs::create_dir_all(parent.as_std_path()).map_err(|e| {
                Diagnostic::error(
                    code::DESTINATION_UNWRITABLE,
                    "could not create the output directory",
                )
                .at(SourceLoc::file(parent.to_owned()))
                .detail(e.to_string())
            })?;
        }
    }

    match std::fs::rename(from.as_std_path(), to.as_std_path()) {
        Ok(()) => Ok(()),
        // Different filesystem: fall back to copy-then-remove. Not atomic, so
        // it is the second choice and never the first.
        Err(e) if e.raw_os_error() == Some(CROSS_DEVICE) => {
            std::fs::copy(from.as_std_path(), to.as_std_path()).map_err(|e| locked_or(to, e))?;
            let _ = std::fs::remove_file(from.as_std_path());
            Ok(())
        }
        Err(e) => Err(locked_or(to, e)),
    }
}

#[cfg(windows)]
const CROSS_DEVICE: i32 = 17; // ERROR_NOT_SAME_DEVICE
#[cfg(unix)]
const CROSS_DEVICE: i32 = 18; // EXDEV

/// The case that actually happens: a successful typeset, then a rename that
/// fails because the previous PDF is open in a viewer holding a lock.
///
/// The user's mental model at that moment is that the build failed, so the
/// message has to say otherwise and name the file (BLD-011).
fn locked_or(to: &Utf8Path, e: std::io::Error) -> Diagnostic {
    let denied = matches!(e.kind(), ErrorKind::PermissionDenied)
        || e.raw_os_error() == Some(SHARING_VIOLATION);
    if denied {
        Diagnostic::error(
            code::DESTINATION_LOCKED,
            format!("{to} is open in another program, so the new PDF could not replace it"),
        )
        .at(SourceLoc::file(to.to_owned()))
        .help(
            "close the file in your PDF viewer and build again — the typesetting itself succeeded",
        )
        .detail(e.to_string())
    } else {
        Diagnostic::error(code::PUBLISH_FAILED, format!("could not write {to}"))
            .at(SourceLoc::file(to.to_owned()))
            .detail(e.to_string())
    }
}

#[cfg(windows)]
const SHARING_VIOLATION: i32 = 32; // ERROR_SHARING_VIOLATION
#[cfg(unix)]
const SHARING_VIOLATION: i32 = libc_eacces();

#[cfg(unix)]
const fn libc_eacces() -> i32 {
    13
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn tmp() -> (tempfile::TempDir, Utf8PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let p = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).unwrap();
        (d, p)
    }

    #[test]
    fn build_dir_is_removed_unless_kept() {
        let (_g, root) = tmp();
        let path = {
            let bd = BuildDir::create(&root, "abc", false).unwrap();
            assert!(bd.path().exists());
            bd.path().to_owned()
        };
        assert!(!path.exists(), "an unkept build directory is cleaned up");

        let path = {
            let mut bd = BuildDir::create(&root, "def", false).unwrap();
            bd.keep();
            bd.path().to_owned()
        };
        assert!(path.exists(), "keep_intermediates retains the directory");
    }

    #[test]
    fn publishing_replaces_the_destination() {
        let (_g, root) = tmp();
        let src = root.join("new.pdf");
        let dst = root.join("out.pdf");
        std::fs::write(src.as_std_path(), b"new").unwrap();
        std::fs::write(dst.as_std_path(), b"old").unwrap();

        publish(&src, &dst).unwrap();
        assert_eq!(std::fs::read(dst.as_std_path()).unwrap(), b"new");
        assert!(!src.exists(), "the scratch copy is moved, not copied");
    }

    #[test]
    fn publishing_creates_a_missing_output_directory() {
        let (_g, root) = tmp();
        let src = root.join("new.pdf");
        std::fs::write(src.as_std_path(), b"new").unwrap();
        let dst = root.join("output").join("deep").join("MyBible.pdf");
        publish(&src, &dst).unwrap();
        assert!(dst.exists());
    }

    /// BLD-009, the property the whole module exists for.
    #[test]
    fn a_failed_build_leaves_the_previous_pdf_byte_identical() {
        let (_g, root) = tmp();
        let dst = root.join("MyBible.pdf");
        std::fs::write(dst.as_std_path(), b"the last known good PDF").unwrap();
        let before = std::fs::read(dst.as_std_path()).unwrap();

        // A build that dies before producing anything.
        let bd = BuildDir::create(&root, "doomed", false).unwrap();
        let never_written = bd.path().join("MyBible.pdf");
        let err = publish(&never_written, &dst).expect_err("nothing to publish");
        assert_eq!(err.code, code::NO_OUTPUT);

        assert_eq!(std::fs::read(dst.as_std_path()).unwrap(), before);
    }

    #[test]
    fn an_empty_pdf_is_refused_rather_than_published() {
        let (_g, root) = tmp();
        let src = root.join("empty.pdf");
        let dst = root.join("out.pdf");
        std::fs::write(src.as_std_path(), b"").unwrap();
        std::fs::write(dst.as_std_path(), b"old").unwrap();

        let err = publish(&src, &dst).expect_err("an empty file is not a PDF");
        assert_eq!(err.code, code::NO_OUTPUT);
        assert_eq!(std::fs::read(dst.as_std_path()).unwrap(), b"old");
    }

    /// BLD-010: nothing partial ever appears at the destination.
    #[test]
    fn no_partial_file_appears_at_the_output_path() {
        let (_g, root) = tmp();
        let dst = root.join("MyBible.pdf");
        let bd = BuildDir::create(&root, "partial", false).unwrap();

        // The backend writes progressively into its own directory.
        let working = bd.path().join("MyBible.pdf");
        std::fs::write(working.as_std_path(), b"%PDF-1.5 partial...").unwrap();
        assert!(!dst.exists(), "the destination is untouched during a build");

        publish(&working, &dst).unwrap();
        assert!(dst.exists());
    }
}
