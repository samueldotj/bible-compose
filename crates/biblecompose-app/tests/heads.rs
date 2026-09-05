//! P4.4 — the running head and the folio, read off a real page.
//!
//! The mechanism came out of S0.4 and has worked since; what it never had was
//! a test, and the two defects below were both live until this file was
//! written. Neither was visible from the code: one needed a page whose head
//! carried two slots at once, the other needed a verse long enough to own a
//! page by itself.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use common::{folio, have_backend, head, pages, typeset, ONE_COLUMN};

/// Each of the seven things a slot used to be able to hold, in a slot of its
/// own — written in the old names, which still read as the template each one
/// meant, so a file from before templates sets the page it always set.
///
/// Three at a time, because a head has three places and the point is that each
/// one shows what it was assigned rather than what its neighbour was.
#[test]
fn every_slot_shows_what_it_names() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "john_1_1_5",
        "[page]
columns = 1
[headers.left_page]
header_left = \"book_name\"
header_center = \"alt_book_name\"
header_right = \"reference_range\"
footer_left = \"first_reference\"
footer_center = \"page_number\"
footer_right = \"last_reference\"
[headers.right_page]
header_left = \"book_name\"
header_center = \"alt_book_name\"
header_right = \"reference_range\"
footer_left = \"first_reference\"
footer_center = \"page_number\"
footer_right = \"last_reference\"
",
    );

    let top = head(&lines, 1);
    assert!(top.contains("John"), "the book name: {top:?}");
    assert!(
        top.contains("1:1"),
        "the range starts at the first verse: {top:?}"
    );
    assert!(top.contains("1:5"), "and ends at the last: {top:?}");

    let bottom = folio(&lines, 1);
    assert!(bottom.contains('1'), "the page number: {bottom:?}");
    assert!(
        bottom.contains("1:1") && bottom.contains("1:5"),
        "{bottom:?}"
    );
}

/// A slot set to `empty` puts nothing there, and a head of three empties is
/// not drawn at all.
#[test]
fn an_empty_head_is_absent_rather_than_blank() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "john_1_1_5",
        "[page]
columns = 1
[headers.left_page]
header_left = \"empty\"
header_center = \"empty\"
header_right = \"empty\"
[headers.right_page]
header_left = \"empty\"
header_center = \"empty\"
header_right = \"empty\"
",
    );
    assert_eq!(head(&lines, 1), "", "nothing was asked for");
    assert_ne!(
        folio(&lines, 1),
        "",
        "the foot still carries the page number"
    );
}

/// **The defect this file found first.** Upstream's `first-reference` takes
/// options and discards them, so `showbook=false` was honoured at one end of
/// the range and ignored at the other — and the default head read
/// `John        John 1:1–1:5`, with the book name twice.
#[test]
fn the_book_name_appears_once_in_the_default_head() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("john_1_1_5", ONE_COLUMN);
    let top = head(&lines, 1);
    assert_eq!(
        top.matches("John").count(),
        1,
        "the book name belongs to its own slot: {top:?}"
    );
}

/// **The second.** `chapterverse` records a reference where a verse *number* is
/// typeset, so a page wholly inside one long verse collected none and the head
/// went blank — the page furniture saying there is no Scripture here while a
/// reader is looking at Scripture.
///
/// P4.4, verbatim: "the range is correct on a page whose first verse started
/// earlier".
#[test]
fn a_page_inside_one_verse_still_names_it() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("long_verse", ONE_COLUMN);
    let all = pages(&lines);
    assert!(all.len() >= 3, "the fixture should run to several pages");

    for page in &all {
        let top = head(&lines, *page);
        assert!(
            top.contains("1:"),
            "page {page} has no reference in its head: {top:?}"
        );
    }

    // The middle of the long verse: no verse begins there, so the head names
    // the one still in progress rather than a range.
    let middle = all[all.len() / 2];
    assert!(
        head(&lines, middle).contains("1:2"),
        "page {middle}: {:?}",
        head(&lines, middle)
    );
}

/// A range whose ends are the same place is not a range. A page holding one
/// verse used to read `1:2–1:2`.
#[test]
fn one_verse_is_not_written_as_a_range() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("long_verse", ONE_COLUMN);
    let all = pages(&lines);
    let middle = head(&lines, all[all.len() / 2]);
    assert!(!middle.contains('–'), "{middle:?}");
}

/// The head is a page's, not a column's: in two columns the range has to cover
/// verses from both, because infonode collects per page and the head is set
/// once the whole page is closed.
#[test]
fn the_range_covers_both_columns() {
    if !have_backend() {
        return;
    }
    let (_g, one) = typeset("john_1_1_5", ONE_COLUMN);
    let (_g2, two) = typeset("john_1_1_5", "[page]\ncolumns = 2\n");

    // The same five verses either way, so the same range — which is the claim:
    // a column break does not truncate it.
    assert!(head(&one, 1).contains("1:1") && head(&one, 1).contains("1:5"));
    assert_eq!(head(&one, 1), head(&two, 1));
}

/// Two books, and the head follows the book rather than the document: nothing
/// is carried across the break.
#[test]
fn the_head_belongs_to_the_book_it_is_on() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("two_books", ONE_COLUMN);
    let all = pages(&lines);

    let first = head(&lines, all[0]);
    let last = head(&lines, *all.last().expect("at least one page"));
    assert!(first.contains("Genesis"), "{first:?}");
    assert!(last.contains("John"), "{last:?}");
    assert!(!last.contains("Genesis"), "{last:?}");
}

/// GUI-006's other half: the folio is a setting like any other, and a project
/// that puts the page number in the head gets it there.
#[test]
fn the_page_number_goes_where_it_is_put() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "long_verse",
        "[page]
columns = 1
[headers.left_page]
header_left = \"page_number\"
header_center = \"empty\"
header_right = \"empty\"
footer_left = \"empty\"
footer_center = \"empty\"
footer_right = \"empty\"
[headers.right_page]
header_left = \"page_number\"
header_center = \"empty\"
header_right = \"empty\"
footer_left = \"empty\"
footer_center = \"empty\"
footer_right = \"empty\"
",
    );
    let all = pages(&lines);
    assert!(all.len() >= 2);
    assert_eq!(head(&lines, all[0]), "1");
    assert_eq!(head(&lines, all[1]), "2");
    assert_eq!(
        folio(&lines, all[0]),
        "",
        "nothing was asked for at the foot"
    );
}

/// **The two sides of a spread are set separately.** The page number at the
/// outer edge — left on a left-hand page, right on a right-hand one — is the
/// commonest reason to want that, and it cannot be said with one arrangement
/// for every page.
#[test]
fn each_side_of_the_spread_has_its_own_head() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "long_verse",
        "[page]
columns = 1
[headers.left_page]
header_left = \"page_number\"
header_center = \"empty\"
header_right = \"empty\"
footer_left = \"empty\"
footer_center = \"empty\"
footer_right = \"empty\"
[headers.right_page]
header_left = \"empty\"
header_center = \"empty\"
header_right = \"page_number\"
footer_left = \"empty\"
footer_center = \"empty\"
footer_right = \"empty\"
",
    );
    let all = pages(&lines);
    assert!(all.len() >= 2, "the fixture runs to a second page");

    // Page 1 is a recto: its number is at the right. Page 2 is a verso: at
    // the left. Both heads carry the number and nothing else.
    let x_of = |page: usize| -> f64 {
        lines
            .iter()
            .filter(|l| l.page == page && l.sizes() == vec![common::HEAD] && l.y > -60.0)
            .map(|l| l.left())
            .next()
            .unwrap_or_else(|| panic!("page {page} has a head"))
    };
    assert_eq!(head(&lines, all[0]), "1");
    assert_eq!(head(&lines, all[1]), "2");
    let (recto, verso) = (x_of(all[0]), x_of(all[1]));
    assert!(
        recto > verso + 100.0,
        "the recto's number ({recto}) sits to the right of the verso's ({verso})"
    );
}

/// **A slot is a template.** Text stays as written and every field reads what
/// it names — all ten of them here, which is what keeps the class's list of
/// field names in step with the resolver's.
#[test]
fn a_slot_is_a_template_of_fields() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "john_1_1_5",
        "[page]
columns = 1
[headers.right_page]
header_left = \"{Book}:{FirstChapter}-{FirstVerse}\"
header_center = \"p. {Page}\"
header_right = \"{first_reference} to {LAST_REFERENCE}\"
footer_left = \"{Range}\"
footer_center = \"{{{LastChapter}:{LastVerse}}}\"
footer_right = \"{AltBook}\"
",
    );
    // `head` joins the marks without their spaces, so "p. 1" reads "p.1".
    let top = head(&lines, 1);
    assert!(
        top.contains("John:1-1"),
        "text and fields as written: {top:?}"
    );
    assert!(
        top.contains("p.1"),
        "the page number in a sentence: {top:?}"
    );
    assert!(
        top.contains("1:1to1:5"),
        "names read without regard to case or underscores: {top:?}"
    );
    let bottom = folio(&lines, 1);
    assert!(bottom.contains("1:1–1:5"), "the range: {bottom:?}");
    assert!(
        bottom.contains("{1:5}"),
        "a doubled brace is a brace of the publisher's own: {bottom:?}"
    );
}

/// **Text around a field stays as written**, brackets included — and the
/// alternate book name of a book that has none is the book's own name, which
/// is the class's rule and worth pinning: a head asking for the fuller name
/// never goes blank.
#[test]
fn text_around_a_field_stays_and_the_alternate_name_falls_back() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "john_1_1_5",
        "[page]
columns = 1
[headers.right_page]
header_left = \"[{AltBook}]\"
header_center = \"\"
header_right = \"{Book} {AltBook}\"
footer_left = \"\"
footer_center = \"\"
footer_right = \"\"
",
    );
    let top = head(&lines, 1);
    assert!(top.contains("[John]"), "the brackets stay: {top:?}");
    assert!(
        top.contains("JohnJohn"),
        "both fields read the book: {top:?}"
    );
}
