//! P4.1 and P4.2, asserted against a real page.
//!
//! Everything here is about placement and marks, not about the model — the
//! golden files already say what is emitted, and the note and cross-reference
//! types have been distinct since P1.5. What only a typeset page can show is
//! whether the note landed where its caller is, whether the mark is the one
//! USFM asked for, whether the sequence restarts, and whether the note area
//! stayed out of the column above it.
//!
//! Skipped, loudly, when no backend is installed: see `backend.rs`.

mod common;

use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::{BookCode, ScriptureDocument};
use biblecompose_testkit::corpus;
use biblecompose_testkit::pdf::Line;
use camino::Utf8Path;

use common::{
    attempt, body_lines, have_backend, note_lines, pages, typeset, BODY, NOTE, ONE_COLUMN,
};

/// Every caller in the body, in reading order.
///
/// Note-sized, short, and *raised*: the class lifts a caller off the baseline,
/// so it never shares a `y` with the line it belongs to and always arrives as a
/// line of its own.
fn callers(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| l.sizes() == vec![NOTE] && l.text().chars().count() <= 3)
        .map(|l| l.text())
        .collect()
}

/// **The defect this file exists for** (spike F-10, P4.1).
///
/// No line of Scripture may sit at or below the first line of the note area on
/// its own page. Before the frame work in this milestone, 22 of the 47 pages of
/// a two-column Mark failed this, by up to 186pt.
fn assert_notes_clear_the_body(lines: &[Line], what: &str) {
    for page in pages(lines) {
        let lowest = body_lines(lines)
            .into_iter()
            .filter(|l| l.page == page)
            .map(|l| l.y)
            .reduce(f64::min);
        let highest = note_lines(lines)
            .into_iter()
            .filter(|l| l.page == page)
            .map(|l| l.y)
            .reduce(f64::max);
        let (Some(lowest), Some(highest)) = (lowest, highest) else {
            continue;
        };
        assert!(
            highest < lowest,
            "{what}: on page {page} a note line at {highest} is above the last \
             line of Scripture at {lowest} — the note area is printing over the column"
        );
    }
}

#[test]
fn notes_never_overlap_the_column_above_them() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", ONE_COLUMN);
    assert_notes_clear_the_body(&lines, "one column");
}

/// The two-column case is where this went wrong, and it went wrong differently:
/// the notes called in the second column steal from the first column after the
/// first column has already been set.
#[test]
fn notes_never_overlap_either_column() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", "");
    assert_notes_clear_the_body(&lines, "two columns");
}

/// SCR-003 and P4.2: two apparatus, two sequences, neither counting the other's
/// marks — and an editor's own caller printed as written and skipped over.
///
/// The fixture calls, in order: a reference, a note, a reference, a note whose
/// caller is `*`, and a note. With numbers for footnotes and letters for
/// references that is `a 1 b * 2`, and any interleaving of one sequence into
/// the other would show up here.
#[test]
fn the_two_sequences_run_independently() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", ONE_COLUMN);
    let first_chapter: Vec<Line> = lines.iter().filter(|l| l.page == 1).cloned().collect();
    let marks = callers(&first_chapter);
    assert_eq!(
        marks,
        vec!["a", "1", "b", "*", "2"],
        "callers on page 1 of {:?}",
        first_chapter.iter().map(Line::text).collect::<Vec<_>>()
    );
}

/// The caller sequence is a setting, and `symbols` is the one an edition with a
/// handful of notes to a page uses.
#[test]
fn the_caller_sequence_is_configurable() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[notes]
footnote_callers = \"symbols\"
cross_reference_callers = \"numbers\"
",
    );
    let marks = callers(
        &lines
            .iter()
            .filter(|l| l.page == 1)
            .cloned()
            .collect::<Vec<_>>(),
    );
    assert_eq!(marks, vec!["1", "*", "2", "*", "†"]);
}

/// P4.1: "numbering restarts per the configured policy rather than running
/// continuously through the book".
///
/// The fixture's second chapter carries one note and one reference. Under the
/// default policy their marks are `1` and `a` again; with the policy off they
/// continue from the first chapter.
#[test]
fn the_sequence_restarts_where_the_policy_says() {
    if !have_backend() {
        return;
    }

    let (_g, restarted) = typeset("apparatus", ONE_COLUMN);
    let last = restarted.iter().map(|l| l.page).max().expect("pages");
    let second_chapter: Vec<Line> = restarted
        .iter()
        .filter(|l| l.page == last)
        .cloned()
        .collect();
    assert_eq!(
        callers(&second_chapter),
        vec!["1", "a"],
        "the second chapter starts its sequences again"
    );

    let (_g2, continuous) = typeset(
        "apparatus",
        "[page]
columns = 1
[notes]
restart_numbering = \"never\"
",
    );
    let last = continuous.iter().map(|l| l.page).max().expect("pages");
    let second_chapter: Vec<Line> = continuous
        .iter()
        .filter(|l| l.page == last)
        .cloned()
        .collect();
    assert_eq!(
        callers(&second_chapter),
        vec!["3", "c"],
        "with no restart the sequences carry on: two notes and two references \
         were called in the first chapter"
    );
}

/// P4.1: a note too long for the note area splits, and the rest goes to the
/// next page.
///
/// The fixture's fifth note is deliberately several pages long. What proves the
/// split is that a sentence which is one sentence in the source is broken
/// across two pages' note areas, with the note continuing at the top of the
/// second one.
#[test]
fn a_note_too_long_for_the_page_splits() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", ONE_COLUMN);

    let notes = note_lines(&lines);
    let mut pages: Vec<usize> = notes.iter().map(|l| l.page).collect();
    pages.dedup();
    assert!(
        pages.len() >= 2,
        "the long note should reach a second page; note lines are on {pages:?}"
    );

    // The continuation is note text and not a new note: a note begins with its
    // caller and its origin reference, and this one begins mid-sentence.
    let second = notes
        .iter()
        .find(|l| l.page == pages[1])
        .expect("a note line on the second page");
    assert!(
        second.text().starts_with("Repeated") || second.text().starts_with("carries"),
        "the second page should continue the long note, not start one: {:?}",
        second.text()
    );
}

/// USFM's `\fr` and `\xo`. The model has carried them since P1.5 and the page
/// never showed them, which made a note keyed by a symbol impossible to place.
#[test]
fn a_note_carries_the_reference_it_is_about() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("apparatus", ONE_COLUMN);
    let first = note_lines(&lines)
        .into_iter()
        .find(|l| l.page == 1)
        .expect("a note on page 1");
    assert!(
        first.text().contains("1:1"),
        "the first note in the area is the reference at 1:1: {:?}",
        first.text()
    );
}

/// SCR-005 and P4.2: the placement is configurable, and the two other answers
/// take the reference out of the note area entirely.
#[test]
fn cross_references_can_be_set_in_the_text() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[notes]
cross_reference_placement = \"inline\"
",
    );

    // In the text, in brackets, at the size the reference style asks for — and
    // with no caller, because the reference is standing where its caller would.
    let inline: Vec<String> = lines
        .iter()
        .filter(|l| l.page == 1 && l.sizes().contains(&NOTE))
        .map(Line::text)
        .filter(|t| t.contains("John1:1"))
        .collect();
    assert!(
        inline.iter().any(|t| t.contains("[John1:1")),
        "the reference should be set inline in brackets: {inline:?}"
    );

    // And nothing in the note area is a reference any more.
    assert!(
        note_lines(&lines)
            .iter()
            .all(|l| !l.text().contains("Acts4:20")),
        "an inline reference must not also be in the note area"
    );
}

#[test]
fn cross_references_can_be_gathered_under_their_paragraph() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[notes]
cross_reference_placement = \"end_of_paragraph\"
",
    );

    let gathered = lines
        .iter()
        .find(|l| l.text().contains("John1:1") && l.text().contains("Acts4:20"))
        .unwrap_or_else(|| {
            panic!(
                "both of the paragraph's references should be on one line under it: {:?}",
                lines.iter().map(Line::text).collect::<Vec<_>>()
            )
        });

    // Under the paragraph, not at the foot: there is Scripture below it.
    assert!(
        body_lines(&lines)
            .iter()
            .any(|l| l.page == gathered.page && l.y < gathered.y),
        "the gathered references should sit inside the text, with Scripture after them"
    );
}

/// Hiding an apparatus hides its marks too. Numbering the notes that remain
/// 3, 7, 11 would be a stranger reading of "hidden" than any other switch in
/// this class takes.
#[test]
fn hiding_the_notes_hides_their_callers() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset(
        "apparatus",
        "[page]
columns = 1
[notes]
show_footnotes = false
show_cross_references = false
",
    );
    assert!(
        note_lines(&lines).is_empty(),
        "no note area: {:?}",
        note_lines(&lines)
            .iter()
            .map(|l| l.text())
            .collect::<Vec<_>>()
    );
    assert!(
        callers(&lines).is_empty(),
        "no callers either: {:?}",
        callers(&lines)
    );
}

/// **A note the first column could not reserve for still composes.**
///
/// The defect that stopped every full-Bible build. A note called in the last
/// lines of column A is reached by the page builder only after column A has
/// been broken and output, so it arrives at the head of column B with the line
/// that called it already committed to the page — and the note area has already
/// been capped at what that column left beneath it, which is nothing. The
/// typesetter's answer to a note it cannot place is to carry it onto the next
/// page together with the line that called it, and there is no such line to
/// carry: the walk that looks for it ran off the start of the page's vertical
/// list and died with `attempt to index a nil value`, naming no book, chapter
/// or page. A sixty-six-book build always found a page like this; a handful of
/// books usually did not, which is what made it look like a limit on size.
///
/// Mark at 11pt on a 6x9 page in two columns is where the corpus produces one,
/// on page 7, in the backend's own font — so this is the whole defect in a
/// book already committed here. What is asserted is that the book composes;
/// `notes_never_overlap_the_column_above_them` is what says the note that
/// moved did not land on the column it moved off.
#[test]
fn a_note_the_first_column_could_not_reserve_for_does_not_stop_the_build() {
    if !have_backend() {
        return;
    }
    let entry = corpus::books()
        .into_iter()
        .find(|e| e.book == "MRK")
        .expect("Mark is in the corpus");
    let source = corpus::read(&entry);
    let code = BookCode::parse(&entry.book).expect("a book code");
    let parsed = biblecompose_scripture::usfm::parse(entry.path.as_str(), source);
    let (book, _) = normalize(code, Utf8Path::new(entry.path.as_str()), &parsed.document);
    let doc = ScriptureDocument::new(vec![book]);

    let (_g, report) = attempt(
        &doc,
        "[page]
columns = 2
size = \"6x9in\"
[typography]
font_family = \"DejaVu Serif\"
font_size = \"11pt\"
leading = \"13pt\"
",
        &[],
    );
    assert!(
        report.output.is_some(),
        "Mark did not compose: {:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
}

/// **A note called low in the second column does not end that column.**
///
/// Below a committed first column the note area holds only what that column
/// left beneath it, so a note called in the second column rarely fits. The
/// class used to answer by ending the page there, which left the second
/// column a few lines deep wherever that happened — on most pages of Mark.
/// The note now goes to the next page instead and the text fills the column.
///
/// Measured as: on every page but the book's last, the second column's last
/// line is within three lines of the first column's. Three, not one, because
/// a heading is kept with what follows it and may leave a column short by
/// the heading and its space. A page before one that opens a chapter is let
/// off: a chapter is not begun in a column's last lines, on purpose, and the
/// space that leaves is that rule's and not a note's.
#[test]
fn a_note_called_in_the_second_column_does_not_end_it() {
    if !have_backend() {
        return;
    }
    let entry = corpus::books()
        .into_iter()
        .find(|e| e.book == "MRK")
        .expect("Mark is in the corpus");
    let source = corpus::read(&entry);
    let code = BookCode::parse(&entry.book).expect("a book code");
    let parsed = biblecompose_scripture::usfm::parse(entry.path.as_str(), source);
    let (book, _) = normalize(code, Utf8Path::new(entry.path.as_str()), &parsed.document);
    let doc = ScriptureDocument::new(vec![book]);

    let (_g, report) = attempt(&doc, "[page]\ncolumns = 2\n", &[]);
    let pdf = report.output.expect("Mark composes");
    let raw = std::fs::read(pdf.as_std_path()).expect("the PDF");
    let lines = biblecompose_testkit::pdf::Pdf::lines(&raw);
    let width = biblecompose_testkit::pdf::Pdf::parse(&raw)
        .page_sizes
        .first()
        .map(|s| s.0)
        .expect("a page size");

    let all = pages(&lines);
    let last = *all.last().expect("pages");
    let tolerance = 3.0 * 11.2 + 1.0;
    let mut short = Vec::new();
    // A chapter opens a page when its figure, many times the body size, is
    // in the first lines of the page's text.
    let opens_chapter = |page: usize| {
        let top = lines
            .iter()
            .filter(|l| l.page == page && l.sizes().contains(&BODY))
            .map(|l| l.y)
            .fold(f64::MIN, f64::max);
        lines.iter().any(|l| {
            l.page == page && l.y > top - 40.0 && l.sizes().iter().any(|s| *s > BODY * 2.0)
        })
    };
    for page in all
        .into_iter()
        .filter(|p| *p != last && !opens_chapter(p + 1))
    {
        // By mark, not by line: where the two columns share a baseline the
        // page's lines run across both of them.
        let body: Vec<_> = lines
            .iter()
            .filter(|l| l.page == page)
            .flat_map(|l| &l.marks)
            .filter(|m| (m.size - BODY).abs() < 0.05)
            .collect();
        let lowest = |right: bool| {
            body.iter()
                .filter(|m| (m.x > width / 2.0) == right)
                .map(|m| m.y)
                .reduce(f64::min)
        };
        if let (Some(a), Some(b)) = (lowest(false), lowest(true)) {
            if b - a > tolerance {
                short.push((page, (b - a).round()));
            }
        }
    }
    assert!(
        short.is_empty(),
        "the second column ends this many points above the first on these \
         pages: {short:?}"
    );
    assert_notes_clear_the_body(&lines, "Mark in two columns");
}
