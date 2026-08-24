//! P4.6 — the headings, and the one rule about where they may fall.
//!
//! Six kinds share the `heading` element and are told apart only by their
//! resolved style, so "renders with its own style" is a claim about the page
//! and nowhere else: the XML for `\s3` and `\s4` differs by one digit, and
//! whether a reader can see the difference is settled by `styles.toml` and the
//! cascade together.
//!
//! **These assertions read size and weight, never italic**, and that is a
//! finding rather than an oversight. The font this application ships with has
//! a regular and a bold face and nothing else, and a request for italic that
//! a font cannot fill is answered with the regular face and no complaint. So
//! `italic = true` on a style is, on the default configuration, invisible —
//! which is exactly why the heading hierarchy is not built on it.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use common::{
    body_lines, have_backend, lines_set_in, text_at, typeset, unligature, BODY, ONE_COLUMN,
};

/// The four section levels, told apart.
///
/// The cascade gives a level nobody defined whatever the level above has
/// (P3.2), which is the right answer for `\s7` and the wrong one for a level
/// USFM names: before this, `s3` and `s4` both came out at `s2`'s 9.6pt, so a
/// translation marking four levels of structure was printed with two.
#[test]
fn the_four_section_levels_are_told_apart() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("headings", ONE_COLUMN);

    let bold = "DejaVuSerif-Bold";
    let each = [
        (10.2, "first"),
        (9.6, "second"),
        (BODY, "third"),
        (8.8, "fourth"),
    ];

    for (size, ordinal) in each {
        let found = lines_set_in(&lines, size, bold);
        assert_eq!(
            found.len(),
            1,
            "exactly one line is set in {size}pt {bold}, so the {ordinal} level \
             has a rendering of its own"
        );
        assert!(
            unligature(&found[0].text()).contains(ordinal),
            "{size}pt {bold} should be the {ordinal}-level heading, and is {:?}",
            found[0].text()
        );
    }
}

/// `\d`, `\sp`, `\r` and `\sr` are not section headings and are not set like
/// them: the first two are part of the text, the last two are apparatus.
#[test]
fn the_other_heading_kinds_have_their_own_styles() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("headings", ONE_COLUMN);

    // The superscription and the speaker are set in the text face at the text
    // size. The visible claim is the negative one — neither is bold — because
    // what separates them from a section heading is that they do not announce
    // themselves as one. (The sheet also asks for italic; see the module note
    // on why no page here can show that.)
    //
    // Read per mark rather than per line: a superscription is the first thing
    // under `\c`, so it shares a baseline with the 21pt chapter figure and no
    // line-level filter can see it on its own.
    let set_as_text = |word: &str| {
        lines
            .iter()
            .flat_map(|l| &l.marks)
            .filter(|m| unligature(&m.text).contains(word))
            .inspect(|m| {
                assert_eq!(
                    (m.size, m.face.as_str()),
                    (BODY, "DejaVuSerif"),
                    "{word:?} is set as running text, not as a heading"
                );
            })
            .count()
    };
    assert_eq!(set_as_text("Absalom"), 1, "the \\d superscription");
    assert_eq!(set_as_text("Beloved"), 1, "the \\sp speaker");

    // Parallel references and the reference range are smaller than the text,
    // which is what marks them as apparatus rather than Scripture.
    let small = text_at(&lines, 7.6);
    assert!(small.contains("Samuel"), "the \\r parallels: {small:?}");
    assert!(small.contains("3:1"), "the \\sr range: {small:?}");
}

/// **A heading never sits alone at the foot of a column.**
///
/// The fixture is built so that, laid out naively, a section heading falls on
/// the last line a column can hold. The class forbids the break after a
/// heading, so it has to move up and take the heading with it.
///
/// Run in one column and in two, because the two have different failure
/// modes: in one column an orphaned heading is the last line of a page, and in
/// two it is the last line of the left column with its text in the right.
#[test]
fn a_heading_is_never_the_last_line_of_a_column() {
    if !have_backend() {
        return;
    }
    for columns in ["1", "2"] {
        let (_g, lines) = typeset("orphan_heading", &format!("[page]\ncolumns = {columns}\n"));

        let headings: Vec<_> = lines.iter().filter(|l| l.sizes() == vec![10.2]).collect();
        assert!(!headings.is_empty(), "the fixture should have headings");

        for heading in headings {
            let followed = body_lines(&lines).into_iter().any(|l| {
                l.page == heading.page
                    && l.y < heading.y
                    // In the same column: a line starting well to the right of
                    // the heading is in the *next* one, and does not count as
                    // following it.
                    && (l.left() - heading.left()).abs() < 40.0
            });
            assert!(
                followed,
                "in {columns} column(s), the heading {:?} on page {} has no \
                 Scripture under it in its own column",
                heading.text(),
                heading.page
            );
        }
    }
}

/// A chapter opening is a chapter opening: the number is set large and sits in
/// the text, so the text that follows runs on from it rather than starting
/// below it.
#[test]
fn a_chapter_opens_with_its_number_in_the_text() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("headings", ONE_COLUMN);

    let opening = lines
        .iter()
        .find(|l| l.sizes().contains(&21.0))
        .expect("the chapter number is set at 21pt");

    // The number itself, first on the line and alone in its size.
    let number = opening
        .marks
        .iter()
        .find(|m| m.size == 21.0)
        .expect("a 21pt mark");
    assert_eq!(number.text, "3", "the chapter's own number");

    // And text on the same baseline, which is what makes it a drop figure the
    // Scripture runs into rather than a line of its own.
    assert!(
        opening.sizes().len() > 1,
        "the chapter number shares its line with the text it opens: {:?}",
        opening.sizes()
    );
    assert!(
        opening
            .marks
            .iter()
            .any(|m| m.x > number.x && m.size != 21.0),
        "the text sits to the right of the number, not under it"
    );
}
