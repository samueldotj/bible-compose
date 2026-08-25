//! A project named relatively builds (BLD-002).
//!
//! Every other test in this suite hands the build an absolute path, because a
//! temporary directory is absolute and that is where fixtures go. A person
//! does not: `biblecompose build --project ./MyBible` is the ordinary way to
//! say it, and it was broken.
//!
//! The backend runs with the *project* as its working directory, so a relative
//! asset path in the document means what the project means by it. That makes
//! every other path handed over relative to somewhere the backend has already
//! stopped standing — `./MyBible/.biblecompose/…` becomes
//! `./MyBible/MyBible/.biblecompose/…`, and the failure arrives from inside
//! SILE as a file it cannot find.
//!
//! Found by a release: the first Linux job to get this far.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, project, BuildReporter, BuildRequest, BuildState, CancelToken};
use camino::Utf8PathBuf;
use common::have_backend;

const BOOK: &str = "\\id JHN\n\\h John\n\\c 1\n\\p\n\\v 1 In the beginning was the Word.\n";

#[test]
fn a_project_named_relatively_builds() {
    if !have_backend() {
        return;
    }
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");

    // The project is a *subdirectory*, and is named by that name alone. The
    // parent becomes the working directory, which is what a person's shell
    // would be sitting in.
    let name = "MyBible";
    let project_dir = root.join(name);
    std::fs::create_dir_all(project_dir.as_std_path()).expect("create the project");
    std::fs::write(project_dir.join("JHN.usfm").as_std_path(), BOOK).expect("write the book");
    std::fs::write(
        project_dir.join("biblecompose.toml").as_std_path(),
        "schema_version = 1\n",
    )
    .expect("write the settings");

    // Changing the process's directory is not something to do inside a test
    // suite that runs in parallel, so the *relative* path is constructed
    // against a working directory this test does not have to own: a relative
    // `Utf8PathBuf` is exactly what the CLI would hand over, and what it means
    // is resolved by the code under test rather than by the shell.
    let opened = project::open(&project_dir);
    assert!(!opened.blocked(), "{:?}", opened.diagnostics);

    let relative = Utf8PathBuf::from(name);
    let mut request = BuildRequest::new(relative.clone(), relative.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = opened.settings.clone();
    request.styles = opened.styles.clone();

    // The build runs from the parent, which is the one thing this test cannot
    // fake — so it is done here, once, and put back. The suite runs these
    // files in separate processes, so the directory belongs to this test.
    let was = std::env::current_dir().expect("a working directory");
    std::env::set_current_dir(root.as_std_path()).expect("move to the parent");

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(
        &opened.document,
        &request,
        &CancelToken::new(),
        &mut reporter,
    );

    std::env::set_current_dir(was).expect("put the working directory back");

    assert_eq!(
        report.state,
        BuildState::Succeeded,
        "a relative --project should build: {:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    let pdf = report.output.expect("a PDF");
    let bytes = std::fs::read(root.join(&pdf).as_std_path())
        .or_else(|_| std::fs::read(pdf.as_std_path()))
        .expect("read the PDF");
    assert!(bytes.starts_with(b"%PDF"), "the file produced is not a PDF");
}
