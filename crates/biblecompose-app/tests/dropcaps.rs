//! Drop caps: the chapter's opening initial, set into the text.
//!
//! What is asserted is read off the page — the size of the initial, which
//! lines make room for it, where the chapter number went — because those are
//! the claims. Whether the initial is the right *run* of text is the emitter's
//! test, which is where a Tamil syllable is told apart from half of one.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_testkit::pdf::Line;
use common::{have_backend, typeset, BODY};

fn dropping(lines: u8) -> String {
    format!("[page]\ncolumns = 1\n[contents]\ndrop_caps = true\ndrop_cap_lines = {lines}\n")
}

/// The dropped initial: the one mark reading `I` set well above body size.
///
/// John 1 opens "In the beginning", and nothing else on the page is a capital
/// I at anything but body size — the chapter figure is a `1`.
fn the_initial(lines: &[Line]) -> f64 {
    let sizes: Vec<f64> = lines
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| m.text == "I" && m.size > BODY * 2.0)
        .map(|m| m.size)
        .collect();
    assert_eq!(sizes.len(), 1, "exactly one dropped initial: {sizes:?}");
    sizes[0]
}

/// Where a line's *text* starts — its verse number or its first word —
/// ignoring the dropped initial, which shares a baseline with one of them and
/// sits at the margin by design.
fn text_left(line: &Line) -> f64 {
    line.marks
        .iter()
        .filter(|m| m.size <= BODY + 0.1)
        .map(|m| m.x)
        .fold(f64::INFINITY, f64::min)
}

/// Body lines on page 1, top to bottom: anything carrying body text, whether
/// or not a verse number or the initial sits on the same baseline. The chapter
/// figure's own line is not text.
fn body_top_down(lines: &[Line]) -> Vec<&Line> {
    let mut body: Vec<&Line> = lines
        .iter()
        .filter(|l| {
            l.page == 1
                && l.sizes().iter().any(|s| (s - BODY).abs() < 0.01)
                && !l.sizes().contains(&21.0)
        })
        .collect();
    // SILE writes top-down as more negative, so the top of the page is the
    // largest y.
    body.sort_by(|a, b| b.y.partial_cmp(&a.y).expect("finite"));
    body
}

/// **The initial spans the lines it was told to**, and the text makes room.
#[test]
fn the_initial_drops_and_the_text_wraps_round_it() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("john_1_1_5", &dropping(3));

    // Three lines tall is well over three body sizes of type.
    let size = the_initial(&lines);
    assert!(size > BODY * 3.0, "a three-line initial is set at {size}pt");

    // The first three body lines are indented past the initial; the fourth
    // returns to the margin. That is the whole of what "drop" means.
    let body = body_top_down(&lines);
    assert!(
        body.len() >= 4,
        "John 1:1–5 should run to four lines at least"
    );
    let margin = body[3..]
        .iter()
        .map(|l| text_left(l))
        .fold(f64::INFINITY, f64::min);
    for (n, line) in body[..3].iter().enumerate() {
        assert!(
            text_left(line) > margin + 5.0,
            "line {} starts at {} and should be indented past the initial (margin {margin})",
            n + 1,
            text_left(line)
        );
    }
    let fourth = text_left(body[3]);
    assert!(
        (fourth - margin).abs() < 0.5,
        "the fourth line returns to the margin: {fourth} against {margin}"
    );
}

/// **The number of lines is the setting, not a constant.**
#[test]
fn the_span_follows_the_setting() {
    if !have_backend() {
        return;
    }
    let (_g2, two) = typeset("john_1_1_5", &dropping(2));
    let (_g3, three) = typeset("john_1_1_5", &dropping(3));
    let (a, b) = (the_initial(&two), the_initial(&three));
    assert!(
        b > a * 1.3,
        "three lines ({b}pt) should be well taller than two ({a}pt)"
    );

    // And two lines indent two, not three.
    let body = body_top_down(&two);
    let margin = body[3..]
        .iter()
        .map(|l| text_left(l))
        .fold(f64::INFINITY, f64::min);
    // A two-line initial is a narrower letter, so the indent is smaller too:
    // 9.7pt here against 15.8pt for three lines.
    assert!(text_left(body[0]) > margin + 5.0 && text_left(body[1]) > margin + 5.0);
    assert!(
        (text_left(body[2]) - margin).abs() < 0.5,
        "with two lines the third returns to the margin: {} against {margin}",
        text_left(body[2])
    );
}

/// **The chapter number moves to a line of its own**, and the first verse
/// goes unnumbered — two large things at one corner would fight, and a
/// superscript 1 wedged between them is a second marker for the same place.
#[test]
fn the_number_takes_its_own_line_and_verse_one_is_unnumbered() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("john_1_1_5", &dropping(3));

    let number = lines
        .iter()
        .find(|l| l.sizes().contains(&21.0))
        .expect("the chapter figure is still set at 21pt");
    assert_eq!(
        number.sizes(),
        vec![21.0],
        "the chapter number shares its line with nothing: {:?}",
        number.text()
    );
    assert_eq!(number.text(), "1");

    let verse_numbers: Vec<String> = lines
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| (m.size - 6.4).abs() < 0.01)
        .map(|m| m.text.clone())
        .collect();
    assert!(
        !verse_numbers.iter().any(|n| n == "1"),
        "verse 1 is unnumbered under a dropped initial: {verse_numbers:?}"
    );
    assert!(
        verse_numbers.iter().any(|n| n == "2"),
        "and the verses after it are still numbered: {verse_numbers:?}"
    );
}

/// **A paragraph shorter than its initial does not let what follows run
/// under it.** Psalm 3's first verse is one line; the initial spans three.
/// Without padding, the next heading set with its baseline 5.6pt *above* the
/// initial's — measured, not imagined.
#[test]
fn a_short_opening_paragraph_is_padded_to_clear_the_initial() {
    if !have_backend() {
        return;
    }
    let (_g, lines) = typeset("headings", &dropping(3));

    let initial = lines
        .iter()
        .flat_map(|l| &l.marks)
        .find(|m| m.text == "O" && m.size > BODY * 2.0)
        .expect("the psalm opens with a dropped O");
    let heading = lines
        .iter()
        .find(|l| l.sizes() == vec![9.6])
        .expect("the second-level heading follows the one-line paragraph");
    assert!(
        heading.y < initial.y - 2.0,
        "the heading (y={}) should sit below the initial's baseline (y={}), not through it",
        heading.y,
        initial.y
    );
}
