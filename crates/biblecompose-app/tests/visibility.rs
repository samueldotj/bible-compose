//! P4.5 — each thing on the page can be turned off on its own, and turning it
//! off does not lose what it marked (SCR-001, SCR-007).
//!
//! The switches themselves are asserted at two levels, because they work in two
//! different ways and both can break. Chapter numbers, verse numbers, footnotes
//! and cross-references are hidden by the *class* from a document that still
//! carries them; introductions, outlines, section headings and chapter labels
//! are withheld at *emission*, because a class that returns without typesetting
//! a heading hangs the backend. `emit.rs` covers the second by reading the XML;
//! this file covers both by reading the page.
//!
//! **The property that matters more than either** is SCR-001: a hidden number
//! must not take its anchor with it. The running head is what proves it — a
//! page whose verse numbers are hidden still knows which verses are on it, and
//! if the anchor had gone with the number the head would go blank.

mod common;

use common::{body_lines, have_backend, head, note_lines, text_at, typeset, NOTE, ONE_COLUMN};

/// Everything on, so the tests below have something to take away.
#[test]
fn by_default_all_of_it_is_there() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", ONE_COLUMN);
    let body = text_at(&lines, 9.2);

    assert!(body.contains('1'), "chapter and verse numbers");
    assert!(!note_lines(&lines).is_empty(), "the note area");
    assert!(head(&lines, 1).contains("1:"), "the running head");
}

/// SCR-007, one switch at a time. Each hides its own thing and leaves the
/// others where they were.
#[test]
fn each_switch_hides_only_its_own() {
    if !have_backend() {
        return;
    }

    // Chapter numbers. The fixture's chapter opening is the only 21pt thing on
    // the page, so its absence is unambiguous.
    let (_g, lines) = typeset(
        "apparatus",
        "[page]\ncolumns = 1\n[numbering]\nshow_chapter_numbers = false\n",
    );
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&21.0)),
        "the chapter number is still set"
    );
    assert!(!note_lines(&lines).is_empty(), "notes were not asked about");

    // Verse numbers, which are the only 6.4pt thing.
    let (_g2, lines) = typeset(
        "apparatus",
        "[page]\ncolumns = 1\n[numbering]\nshow_verse_numbers = false\n",
    );
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&6.4)),
        "verse numbers are still set"
    );
    assert!(
        lines.iter().any(|l| l.sizes().contains(&21.0)),
        "the chapter number was not asked about"
    );

    // Section headings, which are withheld at emission rather than by the
    // class — and the heading text is the proof either way.
    let (_g3, lines) = typeset(
        "apparatus",
        "[page]\ncolumns = 1\n[contents]\nshow_section_headings = false\n",
    );
    assert!(
        !text_at(&lines, 10.2).contains("Word"),
        "the section heading is still on the page"
    );
}

/// **SCR-001.** A number is hidden; its anchor is not.
///
/// The running head is built from the references the page collected, and those
/// come from the same `\v` the number is printed from. If hiding the number had
/// dropped the anchor — which is what a naive implementation does — the head
/// would have nothing to say.
#[test]
fn hiding_the_numbers_keeps_the_references() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[numbering]
show_chapter_numbers = false
show_verse_numbers = false
",
    );

    assert!(
        !lines.iter().any(|l| l.sizes().contains(&6.4)),
        "the verse numbers really are gone"
    );
    let top = head(&lines, 1);
    assert!(
        top.contains("1:1"),
        "the head lost its reference with the number: {top:?}"
    );
    // Joined without spaces: SILE places each word separately and spaces
    // them by moving the pen, so there are no space glyphs to recover.
    assert!(top.contains("1John"), "and its book: {top:?}");
}

/// The same for the apparatus: hidden notes take their callers with them, and
/// nothing else moves.
#[test]
fn hiding_an_apparatus_takes_its_marks_too() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]\ncolumns = 1\n[notes]\nshow_footnotes = false\n",
    );

    // The references remain, so the note area is not empty — only the
    // footnotes are gone, which is the point of the switch being its own.
    let notes = note_lines(&lines);
    assert!(
        notes.iter().all(|l| !l.text().contains("everlasting")),
        "a footnote survived: {:?}",
        notes.iter().map(|l| l.text()).collect::<Vec<_>>()
    );
    assert!(
        notes.iter().any(|l| l.text().contains("John1:1")),
        "the cross-reference was not asked about"
    );
}

/// Everything off at once, which is the reader edition: Scripture and nothing
/// else. The text is still there and the head still knows where it is.
#[test]
fn a_page_can_be_stripped_to_the_words() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[numbering]
show_chapter_numbers = false
show_verse_numbers = false
[contents]
show_section_headings = false
[notes]
show_footnotes = false
show_cross_references = false
",
    );

    assert!(
        note_lines(&lines).is_empty(),
        "nothing should be at the foot"
    );
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&NOTE)),
        "and no callers in the text"
    );
    assert!(
        text_at(&lines, 9.2).contains("beginning"),
        "the Scripture is still set"
    );
    assert!(
        head(&lines, 1).contains("1:1"),
        "and the head still knows the page"
    );
    // One page of nothing but Scripture, which is more of it than the same
    // page carried with the apparatus on.
    assert!(!body_lines(&lines).is_empty());
}

/// A hidden part is hidden, not deleted: turning it back on needs no
/// re-emission of anything the publisher wrote (ADR-002).
#[test]
fn hiding_is_reversible_from_the_same_document() {
    if !have_backend() {
        return;
    }
    let (_g, off) = typeset(
        "apparatus",
        "[page]\ncolumns = 1\n[notes]\nshow_footnotes = false\nshow_cross_references = false\n",
    );
    let (_g2, on) = typeset("apparatus", ONE_COLUMN);

    assert!(note_lines(&off).is_empty());
    assert!(!note_lines(&on).is_empty());

    // Both pages open at the same place, because nothing before verse 1 was
    // taken away — and the stripped one runs *further*, because the room the
    // note area was using is now Scripture. That difference is the whole
    // reason a publisher turns the apparatus off.
    assert!(head(&off, 1).starts_with("1John1:1"), "{:?}", head(&off, 1));
    assert!(head(&on, 1).starts_with("1John1:1"), "{:?}", head(&on, 1));
    let on_page_one = |lines: &[_]| {
        body_lines(lines)
            .into_iter()
            .filter(|l| l.page == 1)
            .count()
    };
    assert!(
        on_page_one(&off) > on_page_one(&on),
        "the first page should hold more of the book: {} lines against {}",
        on_page_one(&off),
        on_page_one(&on)
    );
}
