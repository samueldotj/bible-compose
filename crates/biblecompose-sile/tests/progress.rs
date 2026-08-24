//! Progress reaches the caller *while* the backend runs, not at the end of it.
//!
//! The distinction is the whole feature. SILE writes `[12] ` for each finished
//! page to stderr **without a newline**, so a line-buffered reader delivers a
//! long document's worth in one go when the run ends — which, to someone
//! watching a bar, is the same as never.

use std::time::{Duration, Instant};

use biblecompose_scripture::fixtures;
use biblecompose_sile::{emit, Backend, BackendEvent, BackendJob, CancelToken, SileBackend};
use camino::Utf8PathBuf;

fn backend() -> Option<SileBackend> {
    let b = SileBackend::discover().ok()?;
    b.version().ok().map(|_| b)
}

fn job(work: &Utf8PathBuf) -> BackendJob {
    // The fixture names a figure, and since P4.3 the class no longer swallows
    // a draw that fails — so the artwork has to be where a relative `src`
    // resolves from, which is the folder the backend runs in.
    biblecompose_testkit::place_fixture_assets(work);
    BackendJob {
        xml: emit(&fixtures::kitchen_sink(), &[]).xml,
        work_dir: work.clone(),
        pdf_name: "progress.pdf".to_owned(),
        sile_path: vec![biblecompose_sile::class_dir(
            &biblecompose_testkit::repo_root(),
        )],
        project_root: work.clone(),
        class: "biblecompose".to_owned(),
        class_options: Vec::new(),
    }
}

/// Every page is reported, in order, and before the run is over.
#[test]
fn pages_arrive_while_the_backend_is_still_running() {
    let Some(backend) = backend() else {
        eprintln!("no backend installed; skipping");
        return;
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let work = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");

    let started = Instant::now();
    let mut pages: Vec<(u32, Duration)> = Vec::new();
    let outcome = backend
        .run(&job(&work), &CancelToken::new(), &mut |event| {
            if let BackendEvent::Page(n) = event {
                pages.push((n, started.elapsed()));
            }
        })
        .expect("the fixture typesets");
    let total = started.elapsed();

    assert!(!pages.is_empty(), "no page was ever reported");
    assert!(outcome.pdf.exists());

    // Ascending, and each one seen before the process finished. The second is
    // the property that a line-buffered reader would fail: it would report
    // every page at the same instant, at the end.
    let numbers: Vec<u32> = pages.iter().map(|(n, _)| *n).collect();
    let mut sorted = numbers.clone();
    sorted.sort_unstable();
    assert_eq!(numbers, sorted, "pages arrived out of order: {numbers:?}");

    let first = pages[0].1;
    assert!(
        first < total,
        "the first page arrived at {first:?}, the run took {total:?} — that is the \
         end of the run, not progress during it"
    );
}
