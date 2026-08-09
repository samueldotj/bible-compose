//! P1.2 — the composition corpus verifies against itself.
//!
//! Unlike the smoke run in `normalize.rs`, this needs no environment variable
//! and no external checkout: the books are committed, so this is a CI gate.

use biblecompose_testkit::corpus;
use std::collections::BTreeSet;

#[test]
fn the_corpus_verifies() {
    let problems = corpus::verify();
    assert!(
        problems.is_empty(),
        "{} problem(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

/// The property that makes this corpus different from `usfm-core`'s: every
/// file is a complete book. Column balancing, running heads and note placement
/// only misbehave over pages of continuous text, so a fragment cannot exercise
/// them.
#[test]
fn every_file_is_a_whole_book() {
    for e in corpus::books() {
        assert!(
            e.chapters >= 1 && e.verses >= 20,
            "{} has {} chapters and {} verses",
            e.path,
            e.chapters,
            e.verses
        );
    }
}

#[test]
fn every_required_script_and_feature_is_covered() {
    let mut scripts = BTreeSet::new();
    let mut features = BTreeSet::new();
    for e in corpus::books() {
        let text = corpus::read(&e);
        scripts.extend(corpus::detect_scripts(&text));
        features.extend(corpus::detect_features(&text));
    }

    let missing_scripts: Vec<_> = corpus::REQUIRED_SCRIPTS
        .iter()
        .filter(|s| !scripts.contains(**s))
        .collect();
    let missing_features: Vec<_> = corpus::REQUIRED_FEATURES
        .iter()
        .filter(|f| !features.contains(**f))
        .collect();

    assert!(
        missing_scripts.is_empty(),
        "uncovered scripts: {missing_scripts:?}"
    );
    assert!(
        missing_features.is_empty(),
        "uncovered feature classes: {missing_features:?}"
    );
}

/// Every file must be redistributable, and say on whose authority.
///
/// Most of these books are under copyright rather than in the public domain;
/// they are here because their distributor marks them redistributable. That is
/// evidence rather than interpretation, but it only works if it is recorded
/// per file — so an entry without it is a build failure, not an untidiness.
#[test]
fn every_book_records_its_terms() {
    for e in corpus::books() {
        assert!(!e.source.is_empty(), "{}: no source", e.path);
        assert!(!e.copyright.is_empty(), "{}: no copyright line", e.path);
        assert!(
            e.redistributable.eq_ignore_ascii_case("true"),
            "{}: redistributable is {:?}",
            e.path,
            e.redistributable
        );
    }
}

/// The corpus stays small enough that cloning is not a decision.
///
/// Not an arbitrary limit: it is the reason the selection is a set cover
/// rather than "take the first 50 files", and without a test the next person
/// to add a book has no idea a budget exists.
#[test]
fn the_corpus_stays_small() {
    let total: u64 = corpus::books().iter().map(|e| e.bytes).sum();
    let mb = total as f64 / (1024.0 * 1024.0);
    assert!(mb < 4.0, "the corpus has grown to {mb:.1} MB");
}
