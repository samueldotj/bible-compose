//! P5.5 — a build that has nothing to do does not do it, and knows when it has.
//!
//! The measurement that shaped this is in `cache.rs`: opening a 66-book project
//! and parsing every word of it takes 260 ms warm, so the acceptance bar was
//! already met and a parse cache would have bought a quarter of a second at the
//! price of a class of stale-read bug. The backend is what is slow — 7.9
//! seconds for one book of Mark, minutes for a Bible — so the cache is the one
//! that skips *that*, and it did: 7.9 s to 0.65 s, of which most is process
//! start.
//!
//! What is asserted here is the half that can go wrong silently. A cache that
//! misses is slow; a cache that hits when it should not have is a publisher
//! shipping yesterday's book.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_config::cascade;
use biblecompose_scripture::fixtures;
use camino::Utf8PathBuf;
use common::{have_backend, settings};

/// A project that can be built more than once, with each build's settings
/// chosen at the call.
struct Project {
    _guard: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Project {
    fn new() -> Project {
        let guard = tempfile::tempdir().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
        biblecompose_testkit::place_fixture_assets(&root);
        Project {
            _guard: guard,
            root,
        }
    }

    /// Build, and say whether the backend ran.
    ///
    /// `Typesetting` in the event stream is the only honest answer to that: a
    /// wall clock would be a flake, and asking the cache whether it hit would
    /// be asking the thing under test.
    fn run(&self, overrides: &str, clean: bool) -> bool {
        let doc = fixtures::by_name("john_1_1_5").expect("a known fixture");
        let mut request = BuildRequest::new(self.root.clone(), self.root.join("out.pdf"));
        request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
        request.settings = settings(overrides);
        request.styles = cascade::resolve(None, false).0;
        request.clean = clean;

        let (mut reporter, events) = BuildReporter::new();
        let report = build(&doc, &request, &CancelToken::new(), &mut reporter);
        drop(reporter);
        assert_eq!(
            report.state,
            BuildState::Succeeded,
            "{:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        );
        events.iter().any(|e| {
            matches!(
                e,
                biblecompose_app::BuildEvent::State(BuildState::Typesetting)
            )
        })
    }
}

/// The first build runs; the second, with nothing changed, does not.
#[test]
fn a_second_identical_build_does_not_run_the_backend() {
    if !have_backend() {
        return;
    }
    let project = Project::new();
    assert!(project.run("", false), "the first build has to do the work");
    assert!(
        !project.run("", false),
        "nothing changed, so there was nothing to typeset"
    );
}

/// **A changed setting is a changed build.**
///
/// The failure this guards against is the expensive one: a publisher changes
/// the column count, presses the button, and gets the two-column PDF back.
#[test]
fn a_changed_setting_runs_the_backend_again() {
    if !have_backend() {
        return;
    }
    let project = Project::new();
    project.run("", false);
    assert!(
        project.run("[page]\ncolumns = 1\n", false),
        "one column is a different book from two"
    );
    // And settles again on the new answer rather than flapping.
    assert!(!project.run("[page]\ncolumns = 1\n", false));
}

/// A changed style is a changed build, and the emitted document is what says
/// so — the styles are resolved into it, so nothing has to remember to hash
/// them separately.
#[test]
fn a_changed_style_runs_the_backend_again() {
    if !have_backend() {
        return;
    }
    let project = Project::new();
    project.run("", false);
    assert!(project.run("[typography]\nfont_size = \"11pt\"\n", false));
}

/// `Clean build` overrides the answer, which is what it is for: the
/// fingerprint is a promise about this project's inputs, and a build also
/// reads a system font and artwork that may live anywhere (BLD-007).
#[test]
fn a_clean_build_ignores_the_fingerprint() {
    if !have_backend() {
        return;
    }
    let project = Project::new();
    project.run("", false);
    assert!(!project.run("", false), "the cache is warm");
    assert!(project.run("", true), "clean means run it anyway");
}

/// **A missing PDF is a rebuild**, however good the stamp looks. The stamp
/// records what was built, not that it is still there.
#[test]
fn a_deleted_pdf_is_rebuilt() {
    if !have_backend() {
        return;
    }
    let project = Project::new();
    project.run("", false);
    std::fs::remove_file(project.root.join("out.pdf").as_std_path()).expect("delete the PDF");
    assert!(project.run("", false));
}
