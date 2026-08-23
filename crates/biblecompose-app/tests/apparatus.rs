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

use biblecompose_app::{build, BuildReporter, BuildRequest, CancelToken};
use biblecompose_config::{cascade, settings, ConfigDocument, Settings};
use biblecompose_scripture::fixtures;
use biblecompose_testkit::pdf::{Line, Pdf};
use camino::Utf8PathBuf;

/// The sizes the built-in style sheet sets the body and the notes at.
///
/// **Through the whole application and not through the backend alone**, which
/// is why these tests live in this crate. The apparatus is styled by
/// `styles.toml` and configured by `defaults.toml`, and a test that ran the
/// emitter and the class on their own would be reading a page set entirely in
/// the body size — every note the same size as the Scripture, and every
/// assertion below about which is which vacuously true.
const BODY: f64 = 9.2;
const NOTE: f64 = 7.4;

fn have_backend() -> bool {
    match biblecompose_app::backend_version() {
        Ok(_) => true,
        Err(d) => {
            eprintln!("skipping apparatus tests: {d}");
            false
        }
    }
}

/// The built-in settings with a few keys overridden, resolved the way a project
/// file is — so a spelling these tests use and the resolver rejects fails here
/// rather than being quietly ignored by the class.
fn settings(body: &str) -> Settings {
    let toml = format!("schema_version = 1\n{body}\n");
    let doc = ConfigDocument::parse("apparatus-test.toml", toml).expect("valid TOML");
    let (resolved, diagnostics) = settings::resolve(Some(&doc));
    assert!(
        diagnostics.is_empty(),
        "the test's own settings are not valid: {:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    resolved
}

/// Build one fixture and read the page back.
fn typeset(fixture: &str, overrides: &str) -> (tempfile::TempDir, Vec<Line>) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    let doc = fixtures::by_name(fixture).expect("a known fixture");

    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = settings(overrides);
    request.styles = cascade::resolve(None, false).0;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&doc, &request, &CancelToken::new(), &mut reporter);
    let pdf = report.output.unwrap_or_else(|| {
        panic!(
            "{fixture} failed to build: {:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        )
    });
    let raw = std::fs::read(pdf.as_std_path()).expect("read the PDF");
    (guard, Pdf::lines(&raw))
}

/// A single column, so "the note area" and "under the column" mean one thing.
const ONE_COLUMN: &str = "[page]\ncolumns = 1\n";

/// Lines that are wholly note-sized and long enough to be a line of a note
/// rather than a caller.
///
/// The callers in the body are note-sized too — they are set from the note's
/// own style — so length is what separates them. A caller is one or two glyphs;
/// a line of a note is a line.
fn note_lines(lines: &[Line]) -> Vec<&Line> {
    lines
        .iter()
        .filter(|l| l.sizes() == vec![NOTE] && l.text().chars().count() > 3)
        .collect()
}

fn body_lines(lines: &[Line]) -> Vec<&Line> {
    lines.iter().filter(|l| l.sizes().contains(&BODY)).collect()
}

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
    let pages: Vec<usize> = {
        let mut seen: Vec<usize> = lines.iter().map(|l| l.page).collect();
        seen.dedup();
        seen
    };
    for page in pages {
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
    let pages: Vec<usize> = {
        let mut seen: Vec<usize> = notes.iter().map(|l| l.page).collect();
        seen.dedup();
        seen
    };
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
