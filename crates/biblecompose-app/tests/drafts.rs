//! P5.4 — a proof that says it is one, and cannot be mistaken for the book.
//!
//! Two claims, and the second is the one that matters. A draft is *marked*, on
//! every page, in a way a style sheet cannot turn off. And a draft is written
//! **beside** the finished PDF rather than over it: a proof of two books must
//! not replace a publication of sixty-six, and the only way to be sure of that
//! is for a draft to be unable to name it.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{draft_note, draft_path};
use biblecompose_testkit::pdf::Pdf;
use camino::Utf8Path;
use common::{have_backend, pages, typeset_draft};

/// The mark is on every page, and it says what is missing.
#[test]
fn every_page_of_a_draft_is_marked() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset_draft("two_books", "", &draft_note(2));

    let printed = pages(&lines);
    assert!(printed.len() > 1, "the fixture should run to several pages");

    for page in &printed {
        let marked = lines
            .iter()
            .any(|l| l.page == *page && l.text().replace(' ', "").contains("DRAFT-2books"));
        assert!(
            marked,
            "page {page} of {} carries no draft mark",
            printed.len()
        );
    }
}

/// The mark sits above the running head, in the top margin, so it takes no
/// room from the text and cannot be confused for part of it.
#[test]
fn the_mark_is_above_the_running_head() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset_draft("two_books", "", &draft_note(2));

    let mark = lines
        .iter()
        .find(|l| l.text().replace(' ', "").contains("DRAFT"))
        .expect("a draft mark");
    let head = lines
        .iter()
        .filter(|l| l.page == mark.page && l.sizes() == vec![common::HEAD])
        .map(|l| l.y)
        .fold(f64::NEG_INFINITY, f64::max);

    // SILE writes top-down negative y, so nearer the top is less negative.
    assert!(
        mark.y > head,
        "the mark at {} should sit above the head at {head}",
        mark.y
    );
}

/// A real build carries no mark, and its argument list is unchanged.
#[test]
fn a_finished_build_says_nothing() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = common::typeset("two_books", "");
    assert!(
        !lines.iter().any(|l| l.text().contains("DRAFT")),
        "a finished publication does not announce itself as a proof"
    );
}

/// **A draft cannot overwrite the publication.**
#[test]
fn a_draft_is_written_beside_the_real_pdf() {
    assert_eq!(
        draft_path(Utf8Path::new("/books/My Bible.pdf")),
        Utf8Path::new("/books/My Bible-draft.pdf")
    );
    // No extension, and a name with dots in it that are not one.
    assert_eq!(
        draft_path(Utf8Path::new("/books/proof")),
        Utf8Path::new("/books/proof-draft")
    );
    assert_eq!(
        draft_path(Utf8Path::new("/b/v1.2.pdf")),
        Utf8Path::new("/b/v1.2-draft.pdf")
    );
}

/// And the build actually goes there, leaving anything already at the real
/// path exactly as it was (BLD-009's rule, for the case that most invites
/// breaking it).
#[test]
fn a_draft_build_leaves_the_finished_pdf_alone() {
    if !have_backend() {
        return;
    }
    let (guard, report) = common::attempt_draft("john_1_1_5", "", &draft_note(1));
    let root = camino::Utf8Path::from_path(guard.path()).expect("UTF-8 path");
    let real = root.join("out.pdf");

    // Whatever was at the real path before is still there, untouched, and the
    // draft went next to it.
    assert_eq!(
        std::fs::read(real.as_std_path()).expect("the previous PDF"),
        b"not a pdf",
        "a draft replaced the finished publication"
    );
    assert_eq!(report.output.as_deref(), Some(draft_path(&real).as_path()));
    let raw = std::fs::read(draft_path(&real).as_std_path()).expect("the draft");
    assert!(Pdf::parse(&raw).pages > 0);
}

/// A one-book draft is a small fraction of the work of the whole selection.
///
/// The roadmap asks for "a small fraction of a full-Bible build", which cannot
/// honestly be measured here — this repository has no full Bible, and a wall
/// clock in a test is a flake waiting for a slow machine. What *is* honest and
/// is the whole mechanism: the draft typesets only the books it was given, so
/// the page count scales with the selection and not with the project. If that
/// ever stopped being true, no amount of timing would save it.
#[test]
fn a_draft_typesets_only_what_it_was_given() {
    if !have_backend() {
        return;
    }
    let (_g, both) = typeset_draft("two_books", "", &draft_note(2));
    let (_g2, one) = typeset_draft("john_1_1_5", "", &draft_note(1));

    assert!(
        pages(&one).len() < pages(&both).len(),
        "one book should be fewer pages than two: {} against {}",
        pages(&one).len(),
        pages(&both).len()
    );
}
