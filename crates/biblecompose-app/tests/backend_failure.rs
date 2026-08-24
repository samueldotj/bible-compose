//! P5.8 — a backend failure arrives as something a person can act on.
//!
//! The mapping table itself is unit-tested next to the patterns it holds. What
//! this covers is the plumbing between them: that the backend's output is
//! actually kept, that the tail reaches the classifier, and that a mapped
//! diagnostic comes back out of `build` rather than an exit code.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_config::cascade;
use biblecompose_scripture::fixtures;
use camino::Utf8PathBuf;
use common::{have_backend, settings};

/// Build against a class that fails in a stated way, and return the report.
///
/// A whole class rather than a patch of one: `SILE_PATH` resolves
/// `classes/biblecompose.lua` from the first directory that has it, so the
/// shortest way to make the backend fail predictably is to be that directory.
fn build_against(class_body: &str) -> biblecompose_app::BuildReport {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    let classes = root.join("sile/classes");
    std::fs::create_dir_all(classes.as_std_path()).expect("class directory");
    std::fs::write(classes.join("biblecompose.lua").as_std_path(), class_body)
        .expect("write the class");

    let doc = fixtures::by_name("john_1_1_5").expect("a known fixture");
    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![root.join("sile")];
    request.settings = settings("");
    request.styles = cascade::resolve(None, false).0;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&doc, &request, &CancelToken::new(), &mut reporter);
    // The guard has to outlive the build and not the assertions, and the
    // report holds nothing borrowed from it.
    drop(guard);
    report
}

/// A recognised failure comes back as itself, not as an exit code.
#[test]
fn a_known_failure_is_named() {
    if !have_backend() {
        return;
    }
    let report = build_against("SU.error(\"Could not find requested font Nonesuch\")\n");
    assert_eq!(report.state, BuildState::Failed);

    let d = report
        .diagnostics
        .iter()
        .find(|d| d.code.to_string().starts_with("SILE-"))
        .expect("a backend diagnostic");
    assert_eq!(
        d.code.to_string(),
        "SILE-009",
        "expected the font mapping, got {}: {}",
        d.code,
        d.message
    );
    assert!(d.message.contains("font"), "{}", d.message);

    // **And the evidence came with it** (DIA-005). A mapping that hid what the
    // backend said would be worse than no mapping.
    let detail = d.detail.as_deref().unwrap_or_default();
    assert!(
        detail.contains("Nonesuch"),
        "the raw text should still be there: {detail:?}"
    );
}

/// **An unmapped failure still surfaces**, with the same evidence attached.
/// The table lists what has been seen; it does not decide what is worth
/// reporting.
#[test]
fn an_unknown_failure_is_not_swallowed() {
    if !have_backend() {
        return;
    }
    let report = build_against("SU.error(\"A thing nobody has written a pattern for yet\")\n");
    assert_eq!(report.state, BuildState::Failed);

    let d = report
        .diagnostics
        .iter()
        .find(|d| d.code.to_string().starts_with("SILE-"))
        .expect("a backend diagnostic");
    // A Lua error is a Lua error, so the general class-defect entry claims it
    // — which is honest: something inside the typesetter failed. What matters
    // is that it is reported at all and carries what the backend said.
    let detail = d.detail.as_deref().unwrap_or_default();
    assert!(
        detail.contains("nobody has written a pattern"),
        "the raw text should still be there: {detail:?}"
    );
}
