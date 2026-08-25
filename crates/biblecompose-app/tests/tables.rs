//! P4.7 — lists that stay indented, and columns that line up.
//!
//! Both halves are claims about horizontal position and nothing else, so both
//! are read as `x` coordinates out of the PDF.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_testkit::pdf::Line;
use common::{have_backend, typeset, unligature, ONE_COLUMN};

/// The one line whose text begins with this.
///
/// Spaces are dropped from both sides before comparing. A PDF records where
/// each run of glyphs was placed and nothing about the gaps between them, so
/// a line reads back as `Arah775` — the spacing is real on the page and
/// simply is not text.
fn line_at<'a>(lines: &'a [Line], prefix: &str) -> &'a Line {
    let squash = |s: &str| unligature(s).replace(' ', "");
    let prefix = squash(prefix);
    let found: Vec<&Line> = lines
        .iter()
        .filter(|l| squash(&l.text()).starts_with(&prefix))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one line starting {prefix:?}, found {:?}",
        found.iter().map(|l| l.text()).collect::<Vec<_>>()
    );
    found[0]
}

/// The leftmost point anything on the page was set at — the text block's own
/// left edge, which is what an indent is measured from.
fn margin(lines: &[Line]) -> f64 {
    lines.iter().map(Line::left).fold(f64::INFINITY, f64::min)
}

/// Where the mark containing this text starts.
fn mark_x(line: &Line, text: &str) -> f64 {
    line.marks
        .iter()
        .find(|m| m.text.contains(text))
        .unwrap_or_else(|| panic!("no mark holding {text:?} in {:?}", line.text()))
        .x
}

/// Every level of both list families is indented, by its own amount.
///
/// `\lim` is a list inside a paragraph, so it starts one step further in than
/// a free-standing list of the same depth — which is the whole of the
/// difference between the two families, and was not expressible at all before
/// this: `\lim1` was an unsupported marker and its text came out as prose.
#[test]
fn each_list_level_has_its_own_indent() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("lists_and_tables", ONE_COLUMN);

    let margin = margin(&lines);
    let li = |n: &str| line_at(&lines, &format!("the descendants of {n}")).left();

    // Nine points a level, measured from the margin rather than asserted as
    // absolute coordinates, so the test survives a change of page size.
    for (name, steps) in [
        ("Parosh", 1.0),
        ("Shephatiah", 2.0),
        ("Arah", 3.0),
        ("Pahath-Moab", 4.0),
    ] {
        assert!(
            (li(name) - (margin + 9.0 * steps)).abs() < 0.5,
            "level {steps} should be {steps} steps in from {margin}, and starts at {}",
            li(name)
        );
    }

    // Embedded items, one step deeper than the free-standing level they name.
    let embedded = line_at(&lines, "an item embedded").left();
    let deeper = line_at(&lines, "and one a level deeper").left();
    assert!(
        (embedded - li("Shephatiah")).abs() < 0.5,
        "\\lim1 sits where \\li2 does: {embedded} against {}",
        li("Shephatiah")
    );
    assert!(
        (deeper - li("Arah")).abs() < 0.5,
        "\\lim2 sits where \\li3 does: {deeper} against {}",
        li("Arah")
    );
}

/// **An item that wraps keeps its indent on every line.**
///
/// The distinction that matters, and the one the old layout got wrong: an
/// indent applied as leading glue moves the first line only, so a wrapped item
/// returned to the margin and stopped looking like an item at all. It is a
/// left skip now, which indents the block.
#[test]
fn a_wrapped_list_item_stays_indented() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("lists_and_tables", ONE_COLUMN);

    let first = line_at(&lines, "the descendants of Elam");
    let rest: Vec<&Line> = lines
        .iter()
        .filter(|l| {
            let t = unligature(&l.text()).replace(' ', "");
            t.starts_with("Bebai") || t.starts_with("toneedasecond")
        })
        .collect();
    assert_eq!(rest.len(), 2, "the long item should run to three lines");

    for line in rest {
        assert!(
            (line.left() - first.left()).abs() < 0.5,
            "continuation line {:?} starts at {} and the item starts at {}",
            line.text(),
            line.left(),
            first.left()
        );
    }
}

/// **A column is a column: its position does not depend on the row above it.**
///
/// This is the assertion the old layout could not pass and the reason P4.7
/// exists. Cells used to be set one after another with a fixed gap between
/// them, so the second column began wherever the first column's text happened
/// to end — `Arah` and `Shephatiah` differ by half an inch, and their numbers
/// differed by the same. Measuring the column first is what makes the two
/// numbers share an edge.
#[test]
fn the_second_column_starts_in_the_same_place_whatever_is_beside_it() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("lists_and_tables", ONE_COLUMN);

    let short = line_at(&lines, "Arah775");
    let long = line_at(&lines, "Shephatiah372");

    // The two names differ by a wide margin, which is what gives the test its
    // teeth: without measurement the numbers would differ by the same amount.
    assert!(
        long.left() < mark_x(long, "372") - 60.0,
        "the fixture's two names should differ enough in width to matter"
    );

    // The numbers are the same width, so right alignment puts them at the same
    // x — and they only get there if the column was measured.
    assert!(
        (mark_x(short, "775") - mark_x(long, "372")).abs() < 0.5,
        "two three-digit numbers should share an edge: {} against {}",
        mark_x(short, "775"),
        mark_x(long, "372")
    );

    // Every row's first column begins at the same place.
    let firsts: Vec<f64> = ["Family", "Parosh", "Shephatiah", "Arah", "and the rest"]
        .iter()
        .map(|p| line_at(&lines, p).left())
        .collect();
    for x in &firsts {
        assert!(
            (x - firsts[0]).abs() < 0.01,
            "first column starts at {firsts:?}"
        );
    }
}

/// A right-aligned cell is set from its right edge, so a wider number reaches
/// further left rather than pushing the column over.
#[test]
fn a_right_aligned_cell_grows_leftwards() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("lists_and_tables", ONE_COLUMN);

    let wide = mark_x(line_at(&lines, "Parosh2,172"), "2,172");
    let narrow = mark_x(line_at(&lines, "Arah775"), "775");
    assert!(
        wide < narrow,
        "the five-character number should start left of the three-character one: \
         {wide} against {narrow}"
    );
}

/// A header row is set apart from the body, and a spanning cell is not shut
/// into the first column.
#[test]
fn a_header_is_marked_and_a_span_runs_across() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("lists_and_tables", ONE_COLUMN);

    let header = line_at(&lines, "Family");
    assert!(
        header.faces().iter().all(|f| f.contains("Bold")),
        "the header row is set apart from the body: {:?}",
        header.faces()
    );
    assert_eq!(
        line_at(&lines, "Parosh2,172").faces(),
        vec!["DejaVuSerif".to_owned()],
        "and a body row is not"
    );

    // `\tc1-2` covers both columns, so its text passes the point where the
    // second column begins. Flattened to one cell — which is what happened
    // before the span was carried — it would stop short of it.
    let span = line_at(&lines, "and the rest");
    let column_two = mark_x(header, "Number");
    let reach = span.marks.last().expect("marks").x;
    assert!(
        reach > column_two,
        "a two-column cell should run past {column_two}, and reaches {reach}"
    );
}
