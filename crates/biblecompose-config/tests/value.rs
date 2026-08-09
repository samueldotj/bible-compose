//! P2.4 — units and values become typed things at the configuration
//! boundary, or a diagnostic with a position. Never a string passed through.

use biblecompose_config::value::{self, Unit};
use biblecompose_config::{ConfigDocument, Node};

fn doc(body: &str) -> ConfigDocument {
    ConfigDocument::parse("biblecompose.toml", format!("[t]\n{body}\n"))
        .expect("the fixture parses")
}

/// Reads the one key of a one-key fixture, so each case below is the value
/// under test and nothing else.
fn with<R>(body: &str, f: impl FnOnce(&Node<'_>) -> R) -> R {
    let d = doc(body);
    let node = d.find("t.v").expect("the fixture has one key");
    f(&node)
}

// ------------------------------------------------------------- lengths

#[test]
fn every_supported_unit_converts_to_points() {
    let cases = [
        ("\"72pt\"", 72.0, Unit::Pt),
        ("\"6pc\"", 72.0, Unit::Pc),
        ("\"1in\"", 72.0, Unit::In),
        ("\"25.4mm\"", 72.0, Unit::Mm),
        ("\"2.54cm\"", 72.0, Unit::Cm),
    ];
    for (written, points, unit) in cases {
        let got = with(&format!("v = {written}"), |n| value::length(n).unwrap());
        assert!(
            (got.points() - points).abs() < 1e-9,
            "{written} should be {points}pt, got {}",
            got.points()
        );
        assert_eq!(got.unit(), unit, "the written unit is remembered");
    }
}

/// A form field must show what the publisher typed, not the normalised form.
#[test]
fn a_length_displays_in_the_unit_it_was_written_in() {
    let l = with("v = \"0.55in\"", |n| value::length(n).unwrap());
    assert_eq!(l.to_string(), "0.55in");
    assert!((l.points() - 39.6).abs() < 1e-9);
    // What reaches the backend is always points, always formatted the same.
    assert_eq!(l.to_sile(), "39.6pt");
}

/// DET-001: the text handed to SILE is a function of the value alone.
#[test]
fn the_backend_form_is_identical_for_identical_lengths() {
    let a = with("v = \"1in\"", |n| value::length(n).unwrap());
    let b = with("v = \"72pt\"", |n| value::length(n).unwrap());
    let c = with("v = \"25.4mm\"", |n| value::length(n).unwrap());
    assert_eq!(a.to_sile(), b.to_sile());
    assert_eq!(b.to_sile(), c.to_sile());
    assert_eq!(a.to_sile(), "72pt");
}

#[test]
fn case_and_surrounding_space_do_not_matter() {
    for written in ["\"11.5PT\"", "\"  11.5pt \"", "\"11.5 pt\""] {
        let l = with(&format!("v = {written}"), |n| {
            value::length(n).unwrap_or_else(|d| panic!("{written} should parse: {d}"))
        });
        assert_eq!(l.to_sile(), "11.5pt");
    }
}

/// The acceptance criterion for P2.4: an invalid unit is diagnosed here, with
/// a location, and never reaches SILE.
#[test]
fn an_unknown_unit_is_diagnosed_with_a_position() {
    let err = with("v = \"12furlongs\"", |n| value::length(n).unwrap_err());
    assert_eq!(err.code.as_str(), "CFG-003");
    let loc = err.location.as_ref().unwrap();
    assert_eq!((loc.line, loc.column), (Some(2), Some(1)));
    assert!(err.message.contains("`t.v` is not a length"));
}

/// Two mistakes worth naming, because neither is one the author will spot by
/// rereading their own file.
#[test]
fn the_help_names_the_mistake_that_was_made() {
    let bare = with("v = \"12\"", |n| value::length(n).unwrap_err());
    assert!(
        bare.help.as_deref().unwrap().contains("needs a unit"),
        "{bare:?}"
    );

    let percent = with("v = \"9%ph\"", |n| value::length(n).unwrap_err());
    assert!(
        percent
            .help
            .as_deref()
            .unwrap()
            .contains("percentages of the page are not supported"),
        "{percent:?}"
    );
}

#[test]
fn a_length_that_is_not_a_string_is_a_type_error_not_a_unit_error() {
    let err = with("v = 12", |n| value::length(n).unwrap_err());
    assert_eq!(
        err.code.as_str(),
        "CFG-006",
        "a bare integer is not a string"
    );
}

#[test]
fn a_body_size_must_be_positive_but_a_margin_may_be_zero() {
    let err = with("v = \"0pt\"", |n| value::length(n).unwrap_err());
    assert_eq!(err.code.as_str(), "CFG-007");
    assert!(err.message.contains("greater than zero"));

    let zero = with("v = \"0pt\"", |n| value::length_or_zero(n).unwrap());
    assert_eq!(zero.points(), 0.0);

    let negative = with("v = \"-1pt\"", |n| value::length_or_zero(n).unwrap_err());
    assert!(negative.message.contains("cannot be negative"));
}

// ---------------------------------------------------------- page sizes

#[test]
fn a_page_size_may_be_written_three_ways() {
    let compact = with("v = \"6x9in\"", |n| value::page_size(n).unwrap());
    let spelled = with("v = \"6in x 9in\"", |n| value::page_size(n).unwrap());
    let named = with("v = \"trade\"", |n| value::page_size(n).unwrap());

    assert_eq!(compact.to_sile(), "432pt x 648pt");
    assert_eq!(spelled.to_sile(), compact.to_sile());
    // 6x9in is 152.4 x 228.6mm, so the named size is the same page.
    assert_eq!(named.to_sile(), compact.to_sile());
}

#[test]
fn a_metric_page_size_keeps_its_units_for_display() {
    let a5 = with("v = \"148x210mm\"", |n| value::page_size(n).unwrap());
    assert_eq!(a5.to_string(), "148x210mm");
    assert_eq!(
        with("v = \"a5\"", |n| value::page_size(n).unwrap()).to_string(),
        "148x210mm"
    );
}

#[test]
fn a_named_size_is_case_insensitive() {
    let a = with("v = \"A4\"", |n| value::page_size(n).unwrap());
    let b = with("v = \"a4\"", |n| value::page_size(n).unwrap());
    assert_eq!(a.to_sile(), b.to_sile());
}

/// SILE will accept a 3pt page and then fail in frame solving with a message
/// about glue. Better here, in words about the page.
#[test]
fn an_unlayoutable_page_is_refused_here_rather_than_by_sile() {
    let tiny = with("v = \"3x9pt\"", |n| value::page_size(n).unwrap_err());
    assert_eq!(tiny.code.as_str(), "CFG-007");
    assert!(tiny.message.contains("width of 3pt"), "{}", tiny.message);
    assert!(tiny.message.contains("1in to 48in"));

    let huge = with("v = \"6x600in\"", |n| value::page_size(n).unwrap_err());
    assert!(huge.message.contains("height of 600in"), "{}", huge.message);
}

#[test]
fn an_unrecognised_page_size_lists_the_names_that_work() {
    let err = with("v = \"quarto\"", |n| value::page_size(n).unwrap_err());
    assert_eq!(err.code.as_str(), "CFG-003");
    let help = err.help.as_deref().unwrap();
    assert!(help.contains("6x9in"));
    assert!(help.contains("letter"), "{help}");
}

// -------------------------------------------------------------- choices

const PLACEMENTS: [(&str, u8); 3] = [
    ("footnote-area", 0),
    ("column-bottom", 1),
    ("end-of-book", 2),
];

#[test]
fn a_choice_matches_regardless_of_case() {
    assert_eq!(
        *with("v = \"Footnote-Area\"", |n| value::choice(n, &PLACEMENTS)
            .unwrap()),
        0
    );
}

#[test]
fn an_unknown_choice_lists_the_allowed_ones() {
    let err = with("v = \"margin\"", |n| {
        value::choice(n, &PLACEMENTS).unwrap_err()
    });
    assert_eq!(err.code.as_str(), "CFG-007");
    assert!(err
        .message
        .contains("\"footnote-area\", \"column-bottom\", \"end-of-book\""));
    assert!(err.location.is_some());
}

/// A near miss gets a suggestion; a different word does not get a guess.
#[test]
fn a_typo_is_suggested_but_a_different_word_is_not() {
    let typo = with("v = \"end-of-bok\"", |n| {
        value::choice(n, &PLACEMENTS).unwrap_err()
    });
    assert_eq!(typo.help.as_deref(), Some("did you mean \"end-of-book\"?"));

    let unrelated = with("v = \"inline\"", |n| {
        value::choice(n, &PLACEMENTS).unwrap_err()
    });
    assert_eq!(unrelated.help, None, "no guess is better than a bad guess");
}

// --------------------------------------------------------------- ranges

#[test]
fn a_number_outside_its_range_says_what_the_range_is() {
    assert_eq!(
        *with("v = 1.15", |n| value::number_in(n, 0.8, 3.0).unwrap()),
        1.15
    );

    let err = with("v = 12.0", |n| value::number_in(n, 0.8, 3.0).unwrap_err());
    assert_eq!(err.code.as_str(), "CFG-007");
    assert_eq!(err.message, "`t.v` is 12, but it must be between 0.8 and 3");
}

#[test]
fn columns_are_bounded() {
    assert_eq!(*with("v = 2", |n| value::integer_in(n, 1, 4).unwrap()), 2);
    let err = with("v = 0", |n| value::integer_in(n, 1, 4).unwrap_err());
    assert_eq!(err.message, "`t.v` is 0, but it must be between 1 and 4");
}
