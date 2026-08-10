//! P3.3 — STY-004: what a style sheet says that this release cannot use,
//! reported where it was written.
//!
//! The two shapes are a selector for an element that does not exist and a
//! property that does not exist. Both are silent failures otherwise: the page
//! comes out looking wrong and nothing in the build says why, which is the
//! same category as the tofu page (spike F-12).

use biblecompose_config::cascade;
use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::{Diagnostic, Diagnostics, Severity};

fn resolve(source: &str) -> Diagnostics {
    let doc = ConfigDocument::parse("styles.toml", source.to_owned()).expect("valid fixture");
    cascade::resolve(Some(&doc), false).1
}

fn strict(source: &str) -> Diagnostics {
    let doc = ConfigDocument::parse("styles.toml", source.to_owned()).expect("valid fixture");
    cascade::resolve(Some(&doc), true).1
}

fn only(d: &Diagnostics) -> &Diagnostic {
    assert_eq!(
        d.len(),
        1,
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    d.iter().next().unwrap()
}

// ------------------------------------------------------------- properties

/// The acceptance criterion: a misspelled property is reported at its line
/// rather than silently ignored.
#[test]
fn a_misspelled_property_is_reported_at_its_line() {
    let d = resolve("[chapter]\nweight = 700\n\nitallic = true\n");
    let only = only(&d);

    assert_eq!(only.code.as_str(), "STY-002");
    assert_eq!(only.severity, Severity::Warning);
    assert_eq!(only.message, "`chapter.itallic` is not a style property");

    let loc = only.location.as_ref().expect("STY-004 requires a location");
    assert_eq!(loc.path, "styles.toml");
    assert_eq!((loc.line, loc.column), (Some(4), Some(1)));
}

#[test]
fn a_near_miss_property_is_suggested() {
    assert_eq!(
        only(&resolve("[chapter]\nitallic = true\n"))
            .help
            .as_deref(),
        Some("did you mean `italic`?")
    );
    assert_eq!(
        only(&resolve("[poetry.q1]\nident = \"3pt\"\n"))
            .help
            .as_deref(),
        Some("did you mean `indent`?")
    );
    // `inherits` is not a property, but it is a legal key, so a slip towards
    // it is worth catching too.
    assert_eq!(
        only(&resolve("[poetry.q1]\ninherit = \"poetry.q1\"\n"))
            .help
            .as_deref(),
        Some("did you mean `inherits`?")
    );
}

#[test]
fn a_property_nothing_resembles_lists_the_ones_that_exist() {
    let d = resolve("[chapter]\nletterspacing = \"2pt\"\n");
    let help = only(&d).help.as_deref().unwrap();
    assert!(help.contains("the properties are:"), "{help}");
    assert!(help.contains("font_size"), "{help}");
}

/// Reported, and the rest of the entry still applies — a typo in one property
/// is not a reason to lose the others beside it.
#[test]
fn the_rest_of_the_entry_survives_a_bad_property() {
    let doc = ConfigDocument::parse(
        "styles.toml",
        "[chapter]\nweight = 400\nitallic = true\n".to_owned(),
    )
    .unwrap();
    let (styles, d) = cascade::resolve(Some(&doc), false);

    assert_eq!(d.len(), 1);
    assert_eq!(
        styles
            .get(biblecompose_config::StyleSelector::Chapter)
            .style
            .weight,
        Some(400)
    );
}

// -------------------------------------------------------------- selectors

#[test]
fn an_unknown_selector_is_reported_at_its_line() {
    let d = resolve("[chapter]\nweight = 700\n\n[paragraph.gribble]\nitalic = true\n");
    let only = only(&d);

    assert_eq!(only.code.as_str(), "STY-001");
    assert_eq!(
        only.message,
        "`paragraph.gribble` is not an element this release can style"
    );
    assert_eq!(only.location.as_ref().unwrap().line, Some(4));
}

#[test]
fn a_near_miss_selector_is_suggested() {
    // A level that does not exist in a family that does.
    assert_eq!(
        only(&resolve("[poetry.q0]\nindent = \"3pt\"\n"))
            .help
            .as_deref(),
        Some("did you mean `poetry.q1`?")
    );
    // A misspelled class.
    assert_eq!(
        only(&resolve("[chaptr]\nweight = 700\n")).help.as_deref(),
        Some("did you mean `chapter`?")
    );
}

#[test]
fn a_selector_nothing_resembles_explains_the_shape() {
    let d = resolve("[nonsense.entirely]\nitalic = true\n");
    let help = only(&d).help.as_deref().unwrap();
    assert!(help.contains("[class.marker]"), "{help}");
}

#[test]
fn inheriting_from_a_misspelled_selector_is_suggested_too() {
    let d = resolve("[poetry.q2]\ninherits = \"poetry.q0\"\n");
    let only = only(&d);
    assert_eq!(only.code.as_str(), "STY-001");
    assert!(
        only.message.contains("poetry.q2"),
        "names the style: {}",
        only.message
    );
    assert_eq!(only.help.as_deref(), Some("did you mean `poetry.q1`?"));
    assert!(only.location.is_some());
}

/// A style that inherits from itself is a cycle of one, and is caught when the
/// key is read rather than when the walk runs into it.
#[test]
fn a_style_inheriting_from_itself_is_an_error() {
    let only = only(&resolve("[chapter]\ninherits = \"chapter\"\n")).clone();
    assert_eq!(only.code.as_str(), "STY-003");
    assert_eq!(only.severity, Severity::Error, "a loop cannot be resolved");
    assert_eq!(only.help.as_deref(), Some("remove the `inherits` key"));
}

// ----------------------------------------------------------------- strict

/// CFG-004's `strict` covers styles too. A publisher who asked to be stopped
/// by a settings key this release does not recognise did not mean "except for
/// the ones that decide what the page looks like".
#[test]
fn strict_mode_promotes_both_shapes_to_errors() {
    let lenient = resolve("[paragraph.gribble]\nitallic = true\n");
    assert!(!lenient.has_blocking(), "the default is a warning");

    let strict = strict("[paragraph.gribble]\nitallic = true\n");
    assert!(strict.has_blocking(), "strict mode stops the build");
    assert!(strict.iter().all(|d| d.severity == Severity::Error));
}

/// Every complaint has somewhere to go. A diagnostic about a style that cannot
/// say which line it is about is one a publisher cannot act on.
#[test]
fn every_style_diagnostic_carries_a_position() {
    let d = resolve(
        "[paragraph.gribble]\nitalic = true\n\n\
         [chapter]\nitallic = true\nfont_size = \"enormous\"\n\n\
         [poetry.q2]\ninherits = \"nowhere\"\n",
    );
    assert!(
        d.len() >= 4,
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );

    for diagnostic in d.iter() {
        let loc = diagnostic
            .location
            .as_ref()
            .unwrap_or_else(|| panic!("no position on {diagnostic}"));
        assert_eq!(loc.path, "styles.toml");
        assert!(loc.line.is_some(), "no line on {diagnostic}");
    }
}

/// The built-in sheet must not produce any of these, or every project would
/// start with a panel full of complaints about a file the publisher cannot see.
#[test]
fn the_built_in_sheet_produces_none_of_them() {
    let (_, d) = cascade::resolve(None, true);
    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
}
