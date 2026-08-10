//! Build orchestration: the state machine, cancellation, and the one place
//! that knows the order of the pipeline.
//!
//! The only crate permitted to depend on `biblecompose-sile` (ADR-004), and
//! the only thing that orchestrates — `biblecompose-core` from SRS §12.1 is
//! gone because its stated job had exactly one caller.

pub mod backend_input;
pub mod project;
pub mod publish;
pub mod state;

use biblecompose_config::{ResolvedStyles, Settings};
use biblecompose_diagnostics::{code, Diagnostic, Diagnostics};
use biblecompose_scripture::ScriptureDocument;
use biblecompose_sile::{BackendEvent, BackendJob, SileBackend, Stream};
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use camino::{Utf8Path, Utf8PathBuf};

pub use publish::{publish, BuildDir};
pub use state::{BuildEvent, BuildReporter, BuildState};

/// The backend boundary, re-exported.
///
/// ADR-004's rule is that no crate but this one depends on
/// `biblecompose-sile`, and that no *SILE-specific* type leaks past here.
/// `Backend` and `CancelToken` are the boundary itself rather than anything
/// SILE-specific, so they cross; [`SileBackend`], the implementation, does
/// not, and nothing above this crate ever names it. The distinction is
/// asserted in `biblecompose-testkit/tests/architecture.rs`.
pub use biblecompose_sile::{Backend, CancelToken};

/// The version written on emitted documents (SILE-009).
pub const CONTRACT_VERSION: &str = biblecompose_sile::CONTRACT_VERSION;

/// Emitted backend input, with the SILE-specific parts left behind.
#[derive(Debug, Clone)]
pub struct EmittedDocument {
    pub xml: String,
    /// Markers carried through the model but not renderable by the contract.
    pub unsupported: Vec<String>,
}

/// Emit backend input without invoking anything.
///
/// The golden-file path: fast, hermetic, and needs no typesetter installed.
pub fn emit(doc: &ScriptureDocument, styles: &ResolvedStyles) -> EmittedDocument {
    let e = biblecompose_sile::emit(doc, &backend_input::style_rules(styles));
    EmittedDocument {
        xml: e.xml,
        unsupported: e.dropped.into_iter().map(|u| u.marker).collect(),
    }
}

/// The backend version string, for `--version` and the build log (SILE-002).
pub fn backend_version() -> Result<String, Diagnostic> {
    let backend = SileBackend::discover()?;
    backend.version().map(|v| v.raw)
}

/// Run a build against the backend this installation ships.
///
/// Callers never name a backend. Which one is used, and how it is found, is
/// this crate's business — which is what keeps `SileBackend` from appearing in
/// the CLI, the GUI, or anywhere else.
pub fn build(
    doc: &ScriptureDocument,
    request: &BuildRequest,
    cancel: &CancelToken,
    reporter: &mut BuildReporter,
) -> BuildReport {
    let backend = match SileBackend::discover() {
        Ok(b) => b,
        Err(e) => {
            reporter.advance(BuildState::Loading);
            reporter.advance(BuildState::Loaded);
            return failed(reporter, Diagnostics::new(), e);
        }
    };
    build_with(doc, request, &backend, cancel, reporter)
}

/// What a build needs to know that is not in the document.
#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub project_root: Utf8PathBuf,
    /// Where the finished PDF goes. Never written to until the build succeeds.
    pub output: Utf8PathBuf,
    /// Directories the backend resolves classes and packages from.
    pub sile_path: Vec<Utf8PathBuf>,
    /// SILE-008 / BLD-008.
    pub keep_intermediates: bool,
    /// Resolved settings. Defaults to the built-in ones, so a caller that has
    /// not read a project file still gets a complete, valid set (CFG-001).
    pub settings: Settings,
    /// Resolved styles, likewise (STY-001).
    pub styles: ResolvedStyles,
    /// Deterministic, so the build directory name does not vary between two
    /// otherwise identical runs.
    pub build_id: String,
}

impl BuildRequest {
    pub fn new(project_root: impl Into<Utf8PathBuf>, output: impl Into<Utf8PathBuf>) -> Self {
        BuildRequest {
            project_root: project_root.into(),
            output: output.into(),
            sile_path: Vec::new(),
            keep_intermediates: false,
            settings: Settings::builtin(),
            styles: biblecompose_config::cascade::resolve(None, false).0,
            build_id: "current".to_owned(),
        }
    }

    pub fn with_sile_path(mut self, dirs: Vec<Utf8PathBuf>) -> Self {
        self.sile_path = dirs;
        self
    }

    pub fn keeping_intermediates(mut self, keep: bool) -> Self {
        self.keep_intermediates = keep;
        self
    }

    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    pub fn with_styles(mut self, styles: ResolvedStyles) -> Self {
        self.styles = styles;
        self
    }
}

#[derive(Debug)]
pub struct BuildReport {
    pub state: BuildState,
    pub diagnostics: Diagnostics,
    pub output: Option<Utf8PathBuf>,
    pub backend: Option<String>,
}

impl BuildReport {
    pub fn succeeded(&self) -> bool {
        self.state == BuildState::Succeeded
    }
}

/// Run one build, start to finish.
///
/// The order here is the whole of ARCHITECTURE §9, and two properties of it
/// are load-bearing:
///
/// * **Validation runs to completion before the backend is invoked** (DIA-002),
///   so a blocked build reports every blocking issue at once rather than the
///   first.
/// * **Nothing writes to `request.output`** until a PDF exists in the build
///   directory and is non-empty (BLD-009, BLD-010).
pub fn build_with(
    doc: &ScriptureDocument,
    request: &BuildRequest,
    backend: &dyn Backend,
    cancel: &CancelToken,
    reporter: &mut BuildReporter,
) -> BuildReport {
    let mut diagnostics = Diagnostics::new();

    reporter.advance(BuildState::Loading);
    reporter.advance(BuildState::Loaded);

    // ---- validate ----------------------------------------------------------
    reporter.advance(BuildState::Validating);
    validate(doc, &mut diagnostics);
    publish::preflight_destination(&request.output, &mut diagnostics);
    for d in diagnostics.iter() {
        reporter.diagnostic(d.clone());
    }
    if diagnostics.has_blocking() {
        reporter.advance(BuildState::Blocked);
        return BuildReport {
            state: BuildState::Blocked,
            diagnostics,
            output: None,
            backend: None,
        };
    }
    if cancel.is_cancelled() {
        return cancelled(reporter, diagnostics);
    }

    // ---- emit --------------------------------------------------------------
    reporter.advance(BuildState::Emitting);
    let emitted = biblecompose_sile::emit(doc, &backend_input::style_rules(&request.styles));
    for u in &emitted.dropped {
        let d = Diagnostic::warning(
            code::UNSUPPORTED_MARKER,
            format!(
                "\\{} is not supported by this release and was not rendered",
                u.marker
            ),
        );
        reporter.diagnostic(d.clone());
        diagnostics.push(d);
    }

    let mut build_dir = match BuildDir::create(
        &request.project_root,
        &request.build_id,
        request.keep_intermediates,
    ) {
        Ok(d) => d,
        Err(e) => return failed(reporter, diagnostics, e),
    };

    // Everything the backend says goes here as well as to the event stream.
    // Opened before the run so a failure that happens immediately still leaves
    // a file to read.
    let log_path = build_dir.log_path();
    let mut log_file = std::fs::File::create(log_path.as_std_path())
        .ok()
        .map(std::io::BufWriter::new);
    reporter.log_file(log_path.clone());

    let job = BackendJob {
        xml: emitted.xml,
        work_dir: build_dir.path().to_owned(),
        pdf_name: output_file_name(&request.output),
        sile_path: request.sile_path.clone(),
        project_root: request.project_root.clone(),
        class: "biblecompose".to_owned(),
        class_options: backend_input::class_options(&request.settings),
    };

    if cancel.is_cancelled() {
        return cancelled(reporter, diagnostics);
    }

    // ---- typeset -----------------------------------------------------------
    reporter.advance(BuildState::Typesetting);
    let backend_version;
    // What the last successful build of this project needed, if there was one.
    let expected = remembered_pages(&request.project_root);
    let pages = AtomicU32::new(0);
    let outcome = {
        let reporter = &*reporter;
        backend.run(&job, cancel, &mut |event| match event {
            BackendEvent::Log(line) => {
                let stream = match line.stream {
                    Stream::Stdout => "stdout",
                    Stream::Stderr => "stderr",
                };
                if let Some(file) = log_file.as_mut() {
                    // Tagged, because the two streams interleave and which one
                    // a line came from is half of what it means. A write that
                    // fails is not worth a diagnostic: the build is the point,
                    // and the same text is going to the panel anyway.
                    let _ = writeln!(file, "{stream}: {}", line.text);
                }
                reporter.log(stream, line.text);
            }
            BackendEvent::Page(done) => {
                // Highest seen rather than last reported. A bar that goes
                // backwards is worse than no bar, and nothing guarantees the
                // backend's counter only rises — a second pass over a page
                // would report it twice.
                if done > pages.fetch_max(done, Ordering::Relaxed) {
                    reporter.pages(done, expected);
                }
            }
        })
    };

    if let Some(mut file) = log_file {
        let _ = file.flush();
    }

    let outcome = match outcome {
        Ok(o) => {
            backend_version = Some(o.version.raw.clone());
            reporter.backend(o.version.raw.clone());
            o
        }
        Err(e) => {
            // A backend error deep in a book becomes a Scripture reference
            // rather than an XML line number (SILE-007), using the map built
            // during emission.
            let e = enrich_with_reference(e, &emitted.line_map);
            if e.code == code::CANCELLED {
                return cancelled(reporter, diagnostics);
            }
            // SILE-008 removes intermediates after a *successful* build. A
            // failed one keeps them, because the log explaining the failure is
            // among them and deleting it is deleting the answer to the
            // question the failure just raised.
            build_dir.keep();
            return failed(reporter, diagnostics, e);
        }
    };

    // ---- publish -----------------------------------------------------------
    // Recorded before publishing, because the page count is a fact about the
    // typeset document whether or not moving it into place succeeds.
    remember_pages(&request.project_root, pages.load(Ordering::Relaxed));

    reporter.advance(BuildState::Publishing);
    if let Err(e) = publish(&outcome.pdf, &request.output) {
        build_dir.keep();
        return failed(reporter, diagnostics, e);
    }
    if request.keep_intermediates {
        build_dir.keep();
    }

    reporter.advance(BuildState::Succeeded);
    reporter.output(request.output.clone());
    BuildReport {
        state: BuildState::Succeeded,
        diagnostics,
        output: Some(request.output.clone()),
        backend: backend_version,
    }
}

/// M0's validation is thin on purpose: the document is hand-built, so the only
/// things worth checking are the ones a fixture can still get wrong. Real
/// validation arrives with the parser at P1.5 and the configuration layer at
/// P2.3.
fn validate(doc: &ScriptureDocument, diagnostics: &mut Diagnostics) {
    if doc.books.is_empty() {
        diagnostics.push(Diagnostic::error(
            code::NO_BOOKS_FOUND,
            "the project contains no books to compose",
        ));
    }

    let mut seen: Vec<_> = doc.books.iter().map(|b| b.code).collect();
    seen.sort_by_key(|b| b.order());
    for pair in seen.windows(2) {
        if pair[0] == pair[1] {
            diagnostics.push(
                Diagnostic::error(
                    code::DUPLICATE_BOOK_ID,
                    format!("{} appears more than once", pair[0]),
                )
                .help("remove or exclude the duplicate before building"),
            );
        }
    }
}

/// Where the last build's page count is kept.
///
/// Inside `.biblecompose/`, which discovery already ignores, so it cannot be
/// mistaken for part of the publication.
fn pages_file(project_root: &Utf8Path) -> Utf8PathBuf {
    project_root.join(".biblecompose").join("last-pages")
}

/// How many pages the last build of this project produced.
///
/// Best-effort in both directions: a missing, unreadable or nonsensical file
/// means there is no estimate, and the first build of a project shows a bar
/// with no end rather than a wrong one.
fn remembered_pages(project_root: &Utf8Path) -> Option<u32> {
    let text = std::fs::read_to_string(pages_file(project_root).as_std_path()).ok()?;
    let pages: u32 = text.trim().parse().ok()?;
    (pages > 0).then_some(pages)
}

fn remember_pages(project_root: &Utf8Path, pages: u32) {
    if pages == 0 {
        return;
    }
    let path = pages_file(project_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent.as_std_path());
    }
    // Failing to write is not worth a diagnostic: the only consequence is that
    // the next build's bar has no end.
    let _ = std::fs::write(path.as_std_path(), pages.to_string());
}

fn enrich_with_reference(d: Diagnostic, map: &biblecompose_sile::LineMap) -> Diagnostic {
    let Some(detail) = d.detail.as_deref().or(Some(d.message.as_str())) else {
        return d;
    };
    let Some(line) = first_line_number(detail) else {
        return d;
    };
    let Some(r) = map.resolve(line) else {
        return d;
    };
    d.about(biblecompose_diagnostics::ScriptureRef {
        book: r.book.clone(),
        chapter: r.chapter,
        verse: r.verse,
    })
}

/// Pull an XML line number out of backend text like `document.xml:412:`.
fn first_line_number(s: &str) -> Option<u32> {
    let idx = s.find(".xml:")?;
    let rest = &s[idx + 5..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn output_file_name(output: &Utf8Path) -> String {
    output
        .file_name()
        .filter(|n| !n.is_empty())
        .unwrap_or("output.pdf")
        .to_owned()
}

fn cancelled(reporter: &mut BuildReporter, diagnostics: Diagnostics) -> BuildReport {
    reporter.advance(BuildState::Cancelled);
    BuildReport {
        state: BuildState::Cancelled,
        diagnostics,
        output: None,
        backend: None,
    }
}

fn failed(
    reporter: &mut BuildReporter,
    mut diagnostics: Diagnostics,
    e: Diagnostic,
) -> BuildReport {
    reporter.diagnostic(e.clone());
    diagnostics.push(e);
    reporter.advance(BuildState::Failed);
    BuildReport {
        state: BuildState::Failed,
        diagnostics,
        output: None,
        backend: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biblecompose_scripture::fixtures;
    use biblecompose_sile::{BackendOutcome, BackendVersion, LogLine};

    /// A backend that does what it is told without needing SILE installed.
    /// The point of the trait: everything above it is testable on a machine
    /// with no typesetter at all.
    struct FakeBackend {
        write_pdf: bool,
        fail: bool,
    }

    impl Backend for FakeBackend {
        fn version(&self) -> Result<BackendVersion, Diagnostic> {
            Ok(BackendVersion {
                raw: "SILE v0.15.13 (fake)".to_owned(),
                semver: Some("0.15.13".to_owned()),
            })
        }

        fn run(
            &self,
            job: &BackendJob,
            cancel: &CancelToken,
            report: &mut dyn FnMut(BackendEvent),
        ) -> Result<BackendOutcome, Diagnostic> {
            std::fs::create_dir_all(job.work_dir.as_std_path()).unwrap();
            std::fs::write(job.xml_path().as_std_path(), &job.xml).unwrap();
            report(BackendEvent::Log(LogLine {
                stream: Stream::Stdout,
                text: "fake backend ran".to_owned(),
            }));
            // Two pages, so the progress path is exercised by every build in
            // these tests rather than only by a real typesetter.
            report(BackendEvent::Page(1));
            report(BackendEvent::Page(2));
            if cancel.is_cancelled() {
                return Err(Diagnostic::warning(code::CANCELLED, "build cancelled"));
            }
            if self.fail {
                // The line is found in the document this job was actually
                // given rather than hardcoded. A real backend reports a line
                // in the file it read, and anything above the Scripture — the
                // styles block, since P3.4 — moves it.
                let line = job
                    .xml
                    .lines()
                    .position(|l| l.contains("<verse"))
                    .map(|i| i + 1)
                    .expect("the fixture has a verse");
                return Err(Diagnostic::error(
                    code::NONZERO_EXIT,
                    "the typesetting backend exited with status 1",
                )
                .detail(format!("document.xml:{line}: something went wrong")));
            }
            if self.write_pdf {
                std::fs::write(job.pdf_path().as_std_path(), b"%PDF-1.5 fake").unwrap();
            }
            Ok(BackendOutcome {
                pdf: job.pdf_path(),
                version: self.version()?,
                exit_code: Some(0),
            })
        }
    }

    fn tmp() -> (tempfile::TempDir, Utf8PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let p = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).unwrap();
        (d, p)
    }

    fn states(rx: &std::sync::mpsc::Receiver<BuildEvent>) -> Vec<BuildState> {
        rx.try_iter()
            .filter_map(|e| match e {
                BuildEvent::State(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_successful_build_publishes_and_reports_every_state_in_order() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("MyBible.pdf"));
        let (mut r, rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );

        assert!(report.succeeded());
        assert!(req.output.exists());
        assert_eq!(
            states(&rx),
            [
                BuildState::Loading,
                BuildState::Loaded,
                BuildState::Validating,
                BuildState::Emitting,
                BuildState::Typesetting,
                BuildState::Publishing,
                BuildState::Succeeded,
            ]
        );
        assert!(
            report.backend.is_some(),
            "SILE-002: the version is recorded"
        );
    }

    #[test]
    fn a_backend_failure_leaves_the_previous_pdf_untouched() {
        let (_g, root) = tmp();
        let out = root.join("MyBible.pdf");
        std::fs::write(out.as_std_path(), b"the last known good PDF").unwrap();

        let req = BuildRequest::new(&root, out.clone());
        let (mut r, _rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: false,
                fail: true,
            },
            &CancelToken::new(),
            &mut r,
        );

        assert_eq!(report.state, BuildState::Failed);
        assert_eq!(
            std::fs::read(out.as_std_path()).unwrap(),
            b"the last known good PDF"
        );
    }

    /// SILE-007: a backend error becomes a Scripture reference.
    #[test]
    fn a_backend_error_is_reported_against_a_scripture_reference() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("out.pdf"));
        let (mut r, _rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: false,
                fail: true,
            },
            &CancelToken::new(),
            &mut r,
        );
        let d = report
            .diagnostics
            .iter()
            .find(|d| d.code == code::NONZERO_EXIT)
            .expect("the failure is reported");
        let r = d.reference.as_ref().expect("with a Scripture reference");
        assert_eq!(r.book, "John");
    }

    /// SILE-006: everything the backend said, in a file, tagged by stream.
    #[test]
    fn the_backend_output_is_written_to_a_log_file() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("out.pdf")).keeping_intermediates(true);
        let (mut r, _rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert!(report.succeeded());

        let log = root
            .join(".biblecompose")
            .join("build")
            .join("current")
            .join("build.log");
        let text = std::fs::read_to_string(log.as_std_path()).expect("the log was written");
        assert!(text.contains("stdout: fake backend ran"), "{text}");
    }

    /// A failed build keeps its directory, because the log explaining the
    /// failure is in it and deleting it deletes the answer.
    #[test]
    fn a_failed_build_leaves_its_log_behind() {
        let (_g, root) = tmp();
        // Not asking for intermediates: SILE-008 removes them after a
        // *successful* build, and this is the other case.
        let req = BuildRequest::new(&root, root.join("out.pdf"));
        let (mut r, _rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: false,
                fail: true,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert_eq!(report.state, BuildState::Failed);

        let log = root
            .join(".biblecompose")
            .join("build")
            .join("current")
            .join("build.log");
        assert!(
            log.exists(),
            "the log of a failed build must survive the build directory"
        );
    }

    #[test]
    fn cancelling_before_the_backend_never_starts_it() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("out.pdf"));
        let (mut r, rx) = BuildReporter::new();
        let cancel = CancelToken::new();
        cancel.cancel();
        let report = build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &cancel,
            &mut r,
        );
        assert_eq!(report.state, BuildState::Cancelled);
        assert!(!states(&rx).contains(&BuildState::Typesetting));
        assert!(!req.output.exists());
    }

    #[test]
    fn an_empty_document_blocks_before_the_backend() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("out.pdf"));
        let (mut r, rx) = BuildReporter::new();
        let report = build_with(
            &ScriptureDocument::new(vec![]),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert_eq!(report.state, BuildState::Blocked);
        assert!(!states(&rx).contains(&BuildState::Typesetting));
    }

    #[test]
    fn unsupported_markers_warn_without_blocking() {
        let (_g, root) = tmp();
        let req = BuildRequest::new(&root, root.join("out.pdf"));
        let (mut r, _rx) = BuildReporter::new();
        let report = build_with(
            &fixtures::kitchen_sink(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert!(report.succeeded(), "DIA-003: a warning does not block");
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code == code::UNSUPPORTED_MARKER));
    }

    #[test]
    fn intermediates_are_removed_unless_requested() {
        let (_g, root) = tmp();
        let build_dir = root.join(".biblecompose").join("build").join("current");

        let req = BuildRequest::new(&root, root.join("a.pdf"));
        let (mut r, _rx) = BuildReporter::new();
        build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert!(!build_dir.exists(), "SILE-008: cleaned up by default");

        let req = BuildRequest::new(&root, root.join("b.pdf")).keeping_intermediates(true);
        let (mut r, _rx) = BuildReporter::new();
        build_with(
            &fixtures::john_1_1_5(),
            &req,
            &FakeBackend {
                write_pdf: true,
                fail: false,
            },
            &CancelToken::new(),
            &mut r,
        );
        assert!(
            build_dir.join("document.xml").exists(),
            "BLD-008: retained on request"
        );
    }

    #[test]
    fn extracts_a_line_number_from_backend_text() {
        assert_eq!(first_line_number("document.xml:412: bad"), Some(412));
        assert_eq!(first_line_number("no line here"), None);
    }
}
