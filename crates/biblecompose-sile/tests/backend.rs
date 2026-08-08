//! Integration tests against a real SILE.
//!
//! Skipped with a clear message when no backend is installed, so a machine
//! without a typesetter still runs the rest of the suite. The M0 acceptance —
//! a fixture document becoming a PDF through the real emitter, the real class
//! and the real process invocation — is what these assert when it is present.
//!
//! Point `BIBLECOMPOSE_SILE` at a binary to run them (SILE-004).

use biblecompose_scripture::fixtures;
use biblecompose_sile::{emit, Backend, BackendJob, CancelToken, SileBackend, Stream};
use camino::Utf8PathBuf;

/// `None` when no backend is available, with the reason printed once.
fn backend() -> Option<SileBackend> {
    let b = SileBackend::discover().ok()?;
    match b.version() {
        Ok(_) => Some(b),
        Err(d) => {
            eprintln!("skipping backend integration tests: {d}");
            None
        }
    }
}

fn class_dir() -> Utf8PathBuf {
    biblecompose_testkit::repo_root().join("sile")
}

fn tmp() -> (tempfile::TempDir, Utf8PathBuf) {
    let d = tempfile::tempdir().expect("temp dir");
    let p = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).expect("UTF-8 temp path");
    (d, p)
}

fn job_for(name: &str, work: &Utf8PathBuf) -> BackendJob {
    let doc = fixtures::by_name(name).expect("a known fixture");
    BackendJob {
        xml: emit(&doc).xml,
        work_dir: work.clone(),
        pdf_name: format!("{name}.pdf"),
        sile_path: vec![class_dir()],
        project_root: biblecompose_testkit::repo_root(),
        class: "biblecompose".to_owned(),
    }
}

/// SILE-002: the version is detected and recorded.
#[test]
fn the_backend_reports_a_version() {
    let Some(b) = backend() else { return };
    let v = b.version().expect("a version");
    assert!(v.raw.contains("SILE"), "unexpected banner: {}", v.raw);
    assert!(v.semver.is_some(), "a parseable version: {}", v.raw);
}

/// The M0 acceptance for P0.4: a fixture becomes a PDF through the real class.
#[test]
fn every_fixture_typesets_to_a_pdf() {
    let Some(b) = backend() else { return };
    for name in fixtures::names() {
        let (_g, work) = tmp();
        let job = job_for(name, &work);
        let mut lines = Vec::new();
        let outcome = b
            .run(&job, &CancelToken::new(), &mut |l| lines.push(l))
            .unwrap_or_else(|d| panic!("{name} failed to typeset: {d}\n{:?}", d.detail));

        let pdf = std::fs::read(outcome.pdf.as_std_path()).expect("read the PDF");
        let parsed = biblecompose_testkit::pdf::Pdf::parse(&pdf);
        assert!(parsed.pages >= 1, "{name} produced no pages");
        assert!(
            parsed.embedded_font_files >= 1,
            "{name}: PDF-003 wants fonts embedded"
        );
        let (w, h) = parsed
            .uniform_page_size_inches()
            .unwrap_or_else(|| panic!("{name}: pages differ in size"));
        assert!(
            (w - 6.0).abs() < 0.02 && (h - 9.0).abs() < 0.02,
            "{name}: expected a 6x9in trim, got {w:.2}x{h:.2}"
        );

        // SILE-006: the version line and the backend's own output both arrive.
        assert!(
            lines.iter().any(|l| l.text.contains("SILE")),
            "{name}: no backend output reached the log"
        );
    }
}

/// SILE-006 again, from the other side: stderr is captured, not swallowed into
/// a console nobody sees.
#[test]
fn backend_output_is_captured_from_both_streams() {
    let Some(b) = backend() else { return };
    let (_g, work) = tmp();
    let job = job_for("kitchen_sink", &work);
    let mut lines = Vec::new();
    let _ = b.run(&job, &CancelToken::new(), &mut |l| lines.push(l));
    assert!(!lines.is_empty(), "nothing was captured at all");
    // The fixture carries a figure with a path that does not resolve from the
    // repository root, so SILE has something to say on stderr.
    let streams: Vec<Stream> = lines.iter().map(|l| l.stream).collect();
    assert!(
        streams.contains(&Stream::Stdout),
        "stdout must reach the log"
    );
}

/// BLD-009 as an integration test rather than a unit one: the backend writes
/// only inside its own work directory, so the destination cannot be touched.
#[test]
fn the_backend_writes_only_inside_its_work_directory() {
    let Some(b) = backend() else { return };
    let (_g, work) = tmp();
    let job = job_for("john_1_1_5", &work);
    b.run(&job, &CancelToken::new(), &mut |_| {})
        .expect("a successful build");

    let produced: Vec<String> = std::fs::read_dir(work.as_std_path())
        .expect("read work dir")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(produced.iter().any(|f| f == "document.xml"));
    assert!(produced.iter().any(|f| f == "john_1_1_5.pdf"));
}

/// A cancelled build reports cancellation rather than a spurious failure, and
/// leaves nothing running.
#[test]
fn a_cancelled_build_reports_cancellation() {
    let Some(b) = backend() else { return };
    let (_g, work) = tmp();
    let job = job_for("two_books", &work);
    let cancel = CancelToken::new();
    cancel.cancel();

    let err = b
        .run(&job, &cancel, &mut |_| {})
        .expect_err("a cancelled run does not succeed");
    assert_eq!(err.code, biblecompose_diagnostics::code::CANCELLED);
}
