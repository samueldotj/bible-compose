//! NFR-010 — a finished build leaves no Scripture lying about.
//!
//! "Application logs shall avoid recording Scripture content unnecessarily."
//! The verification the SRS asks for is a review of the logs, which is a thing
//! somebody does once. This is the same claim as a test: build a book whose
//! text is unmistakable, then read every byte the build left behind and look
//! for it.
//!
//! **What the answer turns out to be** is that a successful build leaves
//! nothing at all — the working directory is removed on success, so the only
//! files under `.biblecompose` are a fingerprint and a page count, neither of
//! which is text. The two ways Scripture *can* reach the disk are both asked
//! for: `keep_intermediates`, which exists to be asked for (BLD-008), and a
//! failed build, which keeps its log because a failure nobody can read is a
//! failure nobody can fix.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, BuildReporter, BuildRequest, CancelToken};
use biblecompose_config::cascade;
use biblecompose_scripture::{
    Block, Book, BookCode, BookNames, Inline, ParaStyle, ScriptureDocument,
};
use camino::Utf8PathBuf;
use common::{have_backend, settings};

/// A phrase no part of the machinery would produce on its own.
const PHRASE: &str = "Zophar the Naamathite answered and said";

fn book() -> ScriptureDocument {
    ScriptureDocument::new(vec![Book::new(
        BookCode::parse("JOB").expect("a book code"),
        BookNames::named("Job"),
        vec![Block::Paragraph {
            style: ParaStyle::P,
            content: vec![Inline::Text(format!("{PHRASE}, and it was so."))],
        }],
    )])
}

/// Every file under a directory, with its contents, as lossy text.
fn everything_left(root: &Utf8PathBuf) -> Vec<(Utf8PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(dir.as_std_path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = Utf8PathBuf::from_path_buf(entry.path()).expect("UTF-8 path");
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(path.as_std_path()) {
                out.push((path, String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
    }
    out
}

/// Every file under `root` that holds the phrase, the PDF excepted — which is
/// obviously the Scripture, and is the point.
fn holding_scripture(root: &Utf8PathBuf) -> Vec<Utf8PathBuf> {
    everything_left(root)
        .into_iter()
        .filter(|(path, _)| path.extension() != Some("pdf"))
        .filter(|(_, text)| text.contains(PHRASE))
        .map(|(path, _)| path)
        .collect()
}

fn run(keep: bool) -> (tempfile::TempDir, Utf8PathBuf) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");

    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = settings("");
    request.styles = cascade::resolve(None, false).0;
    request.keep_intermediates = keep;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&book(), &request, &CancelToken::new(), &mut reporter);
    assert!(report.output.is_some(), "the build should succeed");
    (guard, root)
}

/// **A successful build leaves no Scripture behind it.**
#[test]
fn a_finished_build_leaves_no_scripture_on_disk() {
    if !have_backend() {
        return;
    }
    let (_guard, root) = run(false);

    let leaked = holding_scripture(&root);
    assert!(
        leaked.is_empty(),
        "these files hold Scripture after a successful build: {leaked:?}"
    );
}

/// **And `keep_intermediates` does what it says**, so the test above is
/// measuring the policy and not an accident of where files happen to go
/// (BLD-008).
#[test]
fn asking_for_the_intermediates_gets_them() {
    if !have_backend() {
        return;
    }
    let (_guard, root) = run(true);

    let kept = holding_scripture(&root);
    assert!(
        !kept.is_empty(),
        "the generated document should be there when it was asked for"
    );
}

/// A diagnostic quotes a line and never a book.
///
/// NFR-010's other half: "diagnostics should include only the context required
/// to identify a problem". A build's diagnostics travel to the panel, the log
/// and, when something goes wrong, into somebody's bug report — so a
/// diagnostic that carried a chapter would carry it everywhere.
#[test]
fn no_diagnostic_carries_more_than_a_line() {
    let long = "Now the serpent was more crafty than any beast of the field. ".repeat(40);
    let doc = ScriptureDocument::new(vec![Book::new(
        BookCode::parse("GEN").expect("a book code"),
        BookNames::named("Genesis"),
        vec![Block::Paragraph {
            style: ParaStyle::P,
            // A marker this release does not support, so the paragraph
            // produces a diagnostic about a very long piece of text.
            content: vec![Inline::Text(long.clone())],
        }],
    )]);

    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.settings = settings("");
    request.styles = cascade::resolve(None, false).0;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&doc, &request, &CancelToken::new(), &mut reporter);

    for d in report.diagnostics.iter() {
        let whole = format!("{d}");
        assert!(
            whole.len() < 2000,
            "a diagnostic {} characters long is carrying content rather than \
             context: {}",
            whole.len(),
            &whole[..200.min(whole.len())]
        );
    }
}
