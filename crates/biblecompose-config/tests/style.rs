//! P3.1 — typed selectors, and a built-in style for every supported marker.

use std::collections::BTreeSet;

use biblecompose_config::selector::{StyleSelector, MAX_LEVEL};
use biblecompose_config::style::{self, Align, Style, PROPERTIES};
use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::Severity;
use biblecompose_scripture::{CharStyle, HeadingStyle, NoteKind, ParaStyle, PoetryStyle};

fn sheet(source: &str) -> (biblecompose_config::StyleSheet, Vec<String>) {
    let doc = ConfigDocument::parse("styles.toml", source.to_owned()).expect("valid fixture");
    let (sheet, diagnostics) = style::read(&doc, Severity::Warning);
    (sheet, diagnostics.iter().map(|d| d.to_string()).collect())
}

// ---------------------------------------------------------------- STY-001

/// The built-in file is compiled in, so it has to be clean before release —
/// the same bargain `defaults.toml` makes.
#[test]
fn the_built_in_sheet_is_clean() {
    let doc = ConfigDocument::parse("styles.toml", style::BUILTIN_STYLES_TOML.to_owned())
        .expect("the built-in styles parse");
    let (sheet, diagnostics) = style::read(&doc, Severity::Warning);
    assert!(
        diagnostics.is_empty(),
        "{:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(!sheet.is_empty());
}

/// STY-001, the acceptance criterion: every marker the model supports has a
/// selector, and the built-in sheet answers for it.
///
/// "Answers" rather than "sets something" — most paragraph markers correctly
/// have nothing to say, and render as body text.
#[test]
fn every_supported_marker_has_a_style() {
    let builtin = style::builtin();
    let selectors: BTreeSet<String> = StyleSelector::all().into_iter().map(|s| s.key()).collect();

    for marker in ParaStyle::all() {
        let key = StyleSelector::Paragraph(*marker).key();
        assert!(selectors.contains(&key), "no selector for \\{marker}");
        let _ = builtin.get(StyleSelector::Paragraph(*marker));
    }
    for marker in CharStyle::all() {
        let key = StyleSelector::Character(*marker).key();
        assert!(selectors.contains(&key), "no selector for \\{marker}");
    }
    for marker in PoetryStyle::all() {
        for level in 1..=MAX_LEVEL {
            let key = StyleSelector::Poetry(*marker, level).key();
            assert!(
                selectors.contains(&key),
                "no selector for \\{marker}{level}"
            );
        }
    }
    for marker in HeadingStyle::all() {
        for level in 1..=MAX_LEVEL {
            let key = StyleSelector::Heading(*marker, level).key();
            assert!(
                selectors.contains(&key),
                "no selector for \\{marker}{level}"
            );
        }
    }
}

/// The ones a reader would notice immediately if they were missing.
#[test]
fn the_built_in_sheet_sets_what_a_bible_needs() {
    let s = style::builtin();

    let chapter = s.get(StyleSelector::Chapter);
    assert_eq!(
        chapter.font_size.map(|l| l.to_sile()),
        Some("21pt".to_owned())
    );
    assert_eq!(chapter.weight, Some(700));

    let verse = s.get(StyleSelector::Verse);
    assert_eq!(verse.weight, Some(600));
    assert!(verse.raise.is_some(), "a verse number is a superior figure");

    assert_eq!(
        s.get(StyleSelector::Character(CharStyle::Bd)).weight,
        Some(700)
    );
    assert_eq!(
        s.get(StyleSelector::Character(CharStyle::Nd)).smallcaps,
        Some(true)
    );
    assert!(s
        .get(StyleSelector::Note(NoteKind::Footnote))
        .font_size
        .is_some());
    assert_eq!(s.get(StyleSelector::Reference).italic, Some(true));

    // Poetry indents step, and each level is written out so one can be moved
    // without moving the others.
    let indent = |level| s.get(StyleSelector::Poetry(PoetryStyle::Q, level)).indent;
    assert!(indent(1).unwrap().points() < indent(2).unwrap().points());
    assert!(indent(2).unwrap().points() < indent(3).unwrap().points());
}

/// A marker with nothing to say is supported, not missing.
#[test]
fn an_ordinary_paragraph_has_an_empty_style() {
    let s = style::builtin();
    assert!(s.get(StyleSelector::Paragraph(ParaStyle::P)).is_empty());
    assert!(s.get(StyleSelector::Paragraph(ParaStyle::M)).is_empty());
}

// ---------------------------------------------------------------- STY-003

/// The acceptance criterion: a same-named selector in another class cannot
/// collide. Class-prefixing is what guarantees it, and this asserts the
/// property that follows — every selector has a distinct key.
#[test]
fn no_two_selectors_share_a_key() {
    let all = StyleSelector::all();
    let keys: BTreeSet<String> = all.iter().map(|s| s.key()).collect();
    assert_eq!(keys.len(), all.len(), "two selectors produce the same key");
}

#[test]
fn the_class_is_part_of_the_identity() {
    // `d` is a heading in the model. If a character style of the same name is
    // ever added, these stay different elements without anything changing.
    assert_ne!(
        StyleSelector::Heading(HeadingStyle::D, 1).key(),
        "character.d"
    );
    assert_eq!(
        StyleSelector::Heading(HeadingStyle::D, 1).key(),
        "heading.d1"
    );
    assert_eq!(
        StyleSelector::parse("character.d"),
        None,
        "not a character style"
    );
}

/// A key written by hand comes back as the same selector it was printed from.
#[test]
fn every_selector_round_trips_through_its_key() {
    for selector in StyleSelector::all() {
        let key = selector.key();
        assert_eq!(
            StyleSelector::parse(&key),
            Some(selector),
            "`{key}` did not round-trip"
        );
    }
}

/// `\q` is `q1`: normalization gives an unnumbered marker level 1, so a style
/// written for `q1` has to match it or the commonest poetry line in the corpus
/// would be unstyled.
#[test]
fn an_unnumbered_marker_is_level_one() {
    assert_eq!(
        StyleSelector::parse("poetry.q"),
        Some(StyleSelector::Poetry(PoetryStyle::Q, 1))
    );
    assert_eq!(
        StyleSelector::parse("heading.s"),
        Some(StyleSelector::Heading(HeadingStyle::S, 1))
    );
}

/// STY-007's chain, which P3.2 walks.
#[test]
fn a_deeper_level_knows_what_it_inherits_from() {
    assert_eq!(
        StyleSelector::Poetry(PoetryStyle::Q, 3).shallower(),
        Some(StyleSelector::Poetry(PoetryStyle::Q, 2))
    );
    assert_eq!(StyleSelector::Poetry(PoetryStyle::Q, 1).shallower(), None);
    assert_eq!(StyleSelector::Chapter.shallower(), None);
}

// ---------------------------------------------------------------- reading

#[test]
fn a_project_sheet_is_read_into_typed_values() {
    let (s, d) = sheet(
        "[chapter]\nfont_size = \"18pt\"\nweight = 700\n\n\
         [poetry.q2]\nindent = \"1cm\"\nalign = \"center\"\n",
    );
    assert!(d.is_empty(), "{d:?}");

    assert_eq!(
        s.get(StyleSelector::Chapter).font_size.map(|l| l.to_sile()),
        Some("18pt".to_owned())
    );
    let q2 = s.get(StyleSelector::Poetry(PoetryStyle::Q, 2));
    assert_eq!(q2.align, Some(Align::Center));
    assert!(
        (q2.indent.unwrap().points() - 28.3465).abs() < 0.001,
        "1cm in points"
    );
}

/// STY-004: a selector this release has no element for is reported where it
/// was written, and the rest of the sheet is still read.
#[test]
fn an_unknown_selector_is_reported_at_its_line() {
    let (s, d) = sheet("[chapter]\nweight = 700\n\n[paragraph.gribble]\nitalic = true\n");
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("STY-001"), "{d:?}");
    assert!(d[0].contains("styles.toml:4"), "{d:?}");
    assert_eq!(
        s.get(StyleSelector::Chapter).weight,
        Some(700),
        "the rest is read"
    );
}

/// STY-004 again, for a property rather than a selector.
#[test]
fn a_misspelled_property_is_reported_rather_than_ignored() {
    let (s, d) = sheet("[chapter]\nweight = 700\nitallic = true\n");
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("STY-002"), "{d:?}");
    assert!(d[0].contains("chapter.itallic"), "{d:?}");
    assert_eq!(s.get(StyleSelector::Chapter).weight, Some(700));
}

#[test]
fn a_property_of_the_wrong_type_is_reported_with_its_position() {
    let (_, d) = sheet("[chapter]\nfont_size = \"enormous\"\n");
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("CFG-003"), "an invalid unit: {d:?}");
}

#[test]
fn every_property_the_schema_lists_can_actually_be_set() {
    let body: String = PROPERTIES
        .iter()
        .map(|p| match *p {
            "weight" => "weight = 700\n".to_owned(),
            "italic" | "smallcaps" | "border" | "own_line" | "drop_cap" | "new_column" => {
                format!("{p} = true\n")
            }
            "align" => "align = \"end\"\n".to_owned(),
            "new_page" => "new_page = \"right\"\n".to_owned(),
            "color" => "color = \"#c81414\"\n".to_owned(),
            "font_family" => "font_family = \"Some Serif\"\n".to_owned(),
            _ => format!("{p} = \"3pt\"\n"),
        })
        .collect();

    let (s, d) = sheet(&format!("[chapter]\n{body}"));
    assert!(d.is_empty(), "a property list nothing can read: {d:?}");

    let chapter = s.get(StyleSelector::Chapter);
    assert!(!chapter.is_empty());
    assert_eq!(chapter.align, Some(Align::End));
    assert_eq!(chapter.italic, Some(true));
    assert_eq!(
        chapter.new_page,
        Some(biblecompose_config::style::NewPage::Right)
    );
    assert_eq!(chapter.border, Some(true));
    assert_eq!(chapter.gap_after.map(|l| l.points()), Some(3.0));
    assert_eq!(chapter.font_family.as_deref(), Some("Some Serif"));
    assert_eq!(
        chapter.color.map(|c| c.to_string()).as_deref(),
        Some("#c81414")
    );
}

// ---------------------------------------------------------------- cascade

/// STY-002 in miniature: an override changes only what it names.
#[test]
fn overlaying_changes_only_what_the_other_names() {
    let base = style::builtin().get(StyleSelector::Chapter);
    let over = Style {
        weight: Some(400),
        ..Style::default()
    };

    let result = base.clone().overlaid_with(over);
    assert_eq!(result.weight, Some(400), "named, so changed");
    assert_eq!(result.font_size, base.font_size, "unnamed, so untouched");
}

#[test]
fn overlaying_nothing_changes_nothing() {
    let base = style::builtin().get(StyleSelector::Verse);
    assert_eq!(base.clone().overlaid_with(Style::default()), base);
}
