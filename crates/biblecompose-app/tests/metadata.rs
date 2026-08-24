//! P4.8 — what the file says about itself, and where it says a place is.
//!
//! Two requirements meet here and neither is about the page. PDF-005 wants the
//! publication's own names in the document's properties; SCR-008 wants every
//! reference to be somewhere a link could later be pointed at. Both are read
//! back out of the finished PDF — the information dictionary, the named
//! destination tree, and the outline.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_testkit::pdf::{Bookmark, Pdf};
use common::{have_backend, raw_pdf};

/// A project that has said who it is.
const NAMED: &str = "\
[project]
name = \"The Holy Scriptures\"
author = \"An Example Bible Society\"
subject = \"New Testament\"
";

/// The three properties a publisher gave reach the file.
#[test]
fn the_document_carries_the_names_it_was_given() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", NAMED);
    let info = Pdf::info(&raw);

    assert_eq!(
        info.get("Title").map(String::as_str),
        Some("The Holy Scriptures")
    );
    assert_eq!(
        info.get("Author").map(String::as_str),
        Some("An Example Bible Society")
    );
    assert_eq!(
        info.get("Subject").map(String::as_str),
        Some("New Testament")
    );
}

/// **A property nobody set is absent, not empty.**
///
/// The distinction is the whole of the decision. A properties panel showing
/// `Title:` with nothing after it reads as an answer, and the honest answer
/// when a project has not been named is that the document has no title. It
/// specifically is not the name of the folder it was built in: that is a path
/// reaching the output, and a path that reaches the output reaches a golden
/// file.
#[test]
fn an_unset_property_is_absent_rather_than_empty() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "");
    let info = Pdf::info(&raw);

    for key in ["Title", "Author", "Subject"] {
        assert!(
            !info.contains_key(key),
            "nothing was said about {key}, so the file should not claim one: {info:?}"
        );
    }
    // The dictionary still exists — this is a claim about three keys, not
    // about the file having no properties at all.
    assert!(
        info.contains_key("Producer") || info.contains_key("Creator"),
        "the information dictionary is still there: {info:?}"
    );
}

/// A name outside Latin-1 survives the round trip.
///
/// PDF writes a string either literally or as hex, and a writer carrying
/// anything beyond Latin-1 has to choose hex with a byte-order mark. A Tamil
/// title is the case this application exists for, so it is the case the
/// assertion uses.
#[test]
fn a_title_in_another_script_survives() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "[project]\nname = \"திருவிவிலியம்\"\n");
    assert_eq!(
        Pdf::info(&raw).get("Title").map(String::as_str),
        Some("திருவிவிலியம்")
    );
}

/// **Every book and chapter is a place the file can name**, by default.
///
/// `JHN.3.16` is not an invention: it is the reference form Paratext and every
/// reference parser already speak, so a cross-reference turned into a link in
/// a later release needs nothing from the class that is not already there. A
/// prefix of a verse's name is the name of the thing containing it, which is
/// what makes a book and a chapter navigable by the same scheme.
#[test]
fn books_and_chapters_are_named_out_of_the_box() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "");
    let names = Pdf::destinations(&raw);

    assert_eq!(names, vec!["GEN", "GEN.1", "JHN", "JHN.1"]);
}

/// **And every verse, when a project asks for verses.**
///
/// Opt-in because it was measured, not because it is doubtful: one
/// destination per verse cost 15% of the build time and 14% of the file size
/// on a 4,950-verse document, and nothing in this release points at one.
#[test]
fn verses_are_named_when_asked_for() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "[output]\nanchors = \"verse\"\n");
    let names = Pdf::destinations(&raw);

    for wanted in [
        "GEN", "GEN.1", "GEN.1.1", "JHN", "JHN.1", "JHN.1.1", "JHN.1.5",
    ] {
        assert!(
            names.iter().any(|n| n == wanted),
            "{wanted} should be a named destination; the file has {names:?}"
        );
    }

    // No two destinations share a name. A PDF with a duplicate resolves to
    // whichever came last, so a collision is a link that goes to the wrong
    // verse rather than an error anybody would see.
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        names.len(),
        "duplicate destinations in {names:?}"
    );
}

/// And none at all for a publication that is only going to be printed.
#[test]
fn a_print_only_publication_can_have_no_anchors() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "[output]\nanchors = \"none\"\n");
    assert!(Pdf::destinations(&raw).is_empty());
    assert!(
        Pdf::bookmarks(&raw).is_empty(),
        "an outline whose entries point nowhere is worse than no outline"
    );
}

/// **Hiding a number does not remove its anchor** (SCR-001).
///
/// The same rule P4.5 asserts through the running head, asserted here where it
/// now also matters: an edition set without chapter or verse numbers is still
/// a document a reader can be sent to a verse in.
#[test]
fn an_anchor_outlives_the_number_it_belongs_to() {
    if !have_backend() {
        return;
    }
    let (_g, shown) = raw_pdf("two_books", "");
    let (_g2, hidden) = raw_pdf(
        "two_books",
        "[numbering]\nshow_chapter_numbers = false\nshow_verse_numbers = false\n",
    );

    assert_eq!(
        Pdf::destinations(&shown),
        Pdf::destinations(&hidden),
        "the same places are named whether or not their numbers are printed"
    );
}

/// The outline is a book with its chapters under it.
#[test]
fn the_outline_nests_chapters_under_their_books() {
    if !have_backend() {
        return;
    }
    let (_g, raw) = raw_pdf("two_books", "");
    let marks = Pdf::bookmarks(&raw);

    let named = |title: &str| -> &Bookmark {
        marks
            .iter()
            .find(|b| b.title == title)
            .unwrap_or_else(|| panic!("no bookmark titled {title:?} in {marks:?}"))
    };

    // Books at the top, by the name a reader sees rather than by their code.
    assert_eq!(named("Genesis").level, 1);
    assert_eq!(named("John").level, 1);
    // Chapters under them, and pointing at the chapter rather than the book.
    assert_eq!(named("Genesis 1").level, 2);
    assert_eq!(named("John 1").dest, "JHN.1");

    // In reading order: Genesis and everything under it, then John.
    let order: Vec<&str> = marks.iter().map(|b| b.title.as_str()).collect();
    assert_eq!(
        order,
        vec!["Genesis", "Genesis 1", "John", "John 1"],
        "the outline reads in the order the books are printed"
    );
}
