//! The SILE backend: XML emission, process invocation, log mapping.
//!
//! ADR-004: this is the only crate that has heard of SILE, reached only
//! through [`Backend`], and nothing but `biblecompose-app` may depend on it.
//! The rule is asserted by a test in `biblecompose-testkit`, because it is the
//! kind that erodes one convenient import at a time.

pub mod emit;
pub mod process;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use biblecompose_diagnostics::Diagnostic;
use camino::{Utf8Path, Utf8PathBuf};

pub use emit::{emit, Emitted, LineMap, LineRef, CONTRACT_VERSION};
pub use process::SileBackend;

/// One invocation of the typesetting backend.
#[derive(Debug, Clone)]
pub struct BackendJob {
    /// The emitted document. A `String`, not a path: the caller decides where
    /// it lands, and `keep_intermediates` is its decision to make.
    pub xml: String,
    /// A scratch directory this job owns. Never the project, never the output.
    pub work_dir: Utf8PathBuf,
    /// Where the PDF should be written *inside* `work_dir`. Publishing it to
    /// the project is a separate step, so a failure cannot reach the
    /// destination (BLD-009, BLD-010).
    pub pdf_name: String,
    /// Directories the backend may resolve classes and packages from.
    pub sile_path: Vec<Utf8PathBuf>,
    /// Resolved as the process working directory, so relative asset paths in
    /// the document mean what the project means by them.
    pub project_root: Utf8PathBuf,
    /// Passed explicitly rather than inferred from the root element, so the
    /// contract does not depend on how SILE resolves a non-standard root.
    pub class: String,
}

impl BackendJob {
    pub fn xml_path(&self) -> Utf8PathBuf {
        self.work_dir.join("document.xml")
    }

    pub fn pdf_path(&self) -> Utf8PathBuf {
        self.work_dir.join(&self.pdf_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendOutcome {
    /// The PDF, inside the job's work directory.
    pub pdf: Utf8PathBuf,
    pub version: BackendVersion,
    pub exit_code: Option<i32>,
}

/// Recorded in every build log (SILE-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendVersion {
    /// The whole banner, verbatim, because the parts we do not parse today are
    /// exactly the ones a support question will turn on.
    pub raw: String,
    pub semver: Option<String>,
}

impl std::fmt::Display for BackendVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

/// A line of backend output, tagged with the stream it came from.
///
/// SILE-006: nothing is dropped. Both streams are drained concurrently, so a
/// chatty stderr cannot deadlock a full stdout pipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub stream: Stream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Cooperative cancellation, shared with whoever can press the button.
///
/// Cheap to clone and safe to poll from any thread. What makes cancellation
/// actually work is not this flag but the process-tree kill in
/// [`process`] — the flag only decides when to pull it.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The only route to typesetting (SILE-001).
///
/// One trait, one implementation, no backend-neutral model behind it
/// ([ADR-004]). The three inputs to emission — document, settings, styles —
/// are already backend-neutral, so this is the portable boundary and a second
/// backend would be a second implementation reading the same three.
///
/// [ADR-004]: ../../../docs/adr/004-no-layout-crate.md
pub trait Backend {
    fn version(&self) -> Result<BackendVersion, Diagnostic>;

    fn run(
        &self,
        job: &BackendJob,
        cancel: &CancelToken,
        log: &mut dyn FnMut(LogLine),
    ) -> Result<BackendOutcome, Diagnostic>;
}

/// Where the BibleCompose SILE class lives, relative to a repository root.
pub fn class_dir(repo_root: &Utf8Path) -> Utf8PathBuf {
    repo_root.join("sile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token_is_shared_across_clones() {
        let a = CancelToken::new();
        let b = a.clone();
        assert!(!b.is_cancelled());
        a.cancel();
        assert!(b.is_cancelled(), "a clone must observe the cancellation");
    }

    #[test]
    fn job_paths_stay_inside_the_work_directory() {
        let job = BackendJob {
            xml: String::new(),
            work_dir: Utf8PathBuf::from("/tmp/build-1"),
            pdf_name: "MyBible.pdf".to_owned(),
            sile_path: vec![],
            project_root: Utf8PathBuf::from("/project"),
            class: "biblecompose".to_owned(),
        };
        assert_eq!(job.xml_path(), "/tmp/build-1/document.xml");
        assert_eq!(job.pdf_path(), "/tmp/build-1/MyBible.pdf");
    }
}
