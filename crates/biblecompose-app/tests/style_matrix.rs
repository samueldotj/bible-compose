//! P3.8 — every selector class and every property, in a golden; and a style
//! change proven not to reach the selectors it should not.
//!
//! The second half is the one worth having. A cascade is a machine for making
//! one edit affect many things, and the failure it invites is an edit that
//! affects one thing *too many* — a change to the footnote size that also
//! moves the cross-references, which nobody notices until the proof copy.

use std::collections::BTreeSet;

use biblecompose_config::selector::StyleSelector;
use biblecompose_config::style::PROPERTIES;
use biblecompose_config::{cascade, ConfigDocument, ResolvedStyles};
use biblecompose_scripture::fixtures;
use biblecompose_testkit::{corpus, golden};
use camino::Utf8PathBuf;

/// One selector from every class, so the matrix covers the whole vocabulary.
///
/// Checked against the schema below rather than trusted: a class added to
/// `StyleSelector` and forgotten here would leave a whole kind of element
/// untested.
const ONE_PER_CLASS: [&str; 14] = [
    "paragraph.p",
    "poetry.q1",
    "heading.s1",
    "character.bd",
    "list.li1",
    "chapter",
    "verse",
    "note.f",
    "reference",
    "figure",
    "caption",
    "cell",
    "head",
    "folio",
];

/// Selectors nothing inherits from, so a change to one is a change to one.
///
/// The levelled families are deliberately absent: `poetry.q1` has three
/// descendants and changing it *should* move them, which is the test below
/// this one.
const LEAVES: [&str; 8] = [
    "paragraph.p",
    "character.bd",
    "chapter",
    "verse",
    "note.f",
    "reference",
    "head",
    "folio",
];

fn resolve(source: &str) -> ResolvedStyles {
    let doc = ConfigDocument::parse("styles.toml", source.to_owned()).expect("valid fixture");
    let (styles, d) = cascade::resolve(Some(&doc), false);
    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    styles
}

/// A value for every property, distinguishable from the built-in one.
fn every_property() -> String {
    // A font nobody has: this fixture never resolves a face, and naming a real
    // one would make the test depend on the machine running it.
    "font_family = \"Matrix Fixture Serif\"\n\
     font_size = \"13.5pt\"\n\
     weight = 500\n\
     italic = true\n\
     smallcaps = true\n\
     space_above = \"3.5pt\"\n\
     space_below = \"2.5pt\"\n\
     indent = \"7.5pt\"\n\
     raise = \"1.5pt\"\n\
     align = \"center\"\n\
     color = \"#c81414\"\n\
     border = true\n\
     border_width = \"0.75pt\"\n\
     own_line = true\n\
     gap_before = \"2pt\"\n\
     gap_after = \"6pt\"\n\
     new_column = false\n\
     new_page = \"next\"\n"
        .to_owned()
}

fn emit(styles: &ResolvedStyles) -> String {
    biblecompose_app::emit(&fixtures::john_1_1_5(), styles).xml
}

/// The `<style …/>` lines, which are the whole subject here.
fn style_lines(xml: &str) -> Vec<String> {
    xml.lines()
        .filter(|l| l.starts_with("<style "))
        .map(str::to_owned)
        .collect()
}

// ------------------------------------------------------------- the matrix

/// Every class named in the schema has a representative in the matrix.
#[test]
fn the_matrix_covers_every_selector_class() {
    let classes: BTreeSet<&'static str> = StyleSelector::all()
        .into_iter()
        .map(|s| s.class())
        .collect();

    let covered: BTreeSet<&'static str> = ONE_PER_CLASS
        .iter()
        .map(|key| {
            StyleSelector::parse(key)
                .unwrap_or_else(|| panic!("`{key}` is not a selector"))
                .class()
        })
        .collect();

    assert_eq!(
        covered, classes,
        "a selector class exists that the style matrix does not exercise"
    );
}

/// The golden: every class, every property, in the document.
#[test]
fn the_full_matrix_emits_a_stable_document() {
    let mut sheet = String::new();
    for selector in ONE_PER_CLASS {
        sheet.push_str(&format!("[{selector}]\n{}\n", every_property()));
    }

    let xml = emit(&resolve(&sheet));
    golden::assert_matches(&golden_path("style_matrix"), &xml);
}

/// Every property the schema lists reaches the document for every class —
/// asserted against `PROPERTIES` rather than against the golden's contents, so
/// a property added to the schema and dropped on the way out fails here.
#[test]
fn every_class_carries_every_property() {
    let mut sheet = String::new();
    for selector in ONE_PER_CLASS {
        sheet.push_str(&format!("[{selector}]\n{}\n", every_property()));
    }
    let xml = emit(&resolve(&sheet));

    for selector in ONE_PER_CLASS {
        let line = style_lines(&xml)
            .into_iter()
            .find(|l| l.contains(&format!("for=\"{selector}\"")))
            .unwrap_or_else(|| panic!("no rule for `{selector}`"));

        for property in PROPERTIES {
            assert!(
                line.contains(&format!("{property}=\"")),
                "`{selector}` lost `{property}` on the way out:\n  {line}"
            );
        }
    }
}

// ---------------------------------------------------------- isolation

/// The acceptance criterion: a change that should affect one selector affects
/// one selector.
#[test]
fn changing_one_selector_leaves_the_others_alone() {
    let baseline = style_lines(&emit(&cascade::resolve(None, false).0));

    for selector in LEAVES {
        let xml = emit(&resolve(&format!("[{selector}]\nspace_above = \"11pt\"\n")));
        let after = style_lines(&xml);

        let differing: Vec<&String> = after
            .iter()
            .filter(|line| !baseline.contains(line))
            .collect();

        assert_eq!(
            differing.len(),
            1,
            "changing `{selector}` moved {} rules, not one: {differing:?}",
            differing.len()
        );
        assert!(
            differing[0].contains(&format!("for=\"{selector}\"")),
            "changing `{selector}` moved `{}` instead",
            differing[0]
        );
        assert!(differing[0].contains("space_above=\"11pt\""));
    }
}

/// The isolation test above passes on the first run, which for a test whose
/// job is to catch a leak is worth being suspicious of. This is the control:
/// the same check applied to a selector that *does* reach further finds it.
///
/// Without this, "one rule differed" could equally mean the check cannot see
/// more than one.
#[test]
fn the_isolation_check_would_notice_a_leak() {
    let baseline = style_lines(&emit(&cascade::resolve(None, false).0));
    let after = style_lines(&emit(&resolve("[poetry.q1]\nspace_above = \"11pt\"\n")));

    let differing = after.iter().filter(|l| !baseline.contains(l)).count();
    assert_eq!(
        differing, 4,
        "q1 and the three levels under it, or the check is blind"
    );
}

/// And the other half of the same claim: a change that *should* reach further
/// does. Isolation is a property of leaves, not a rule the cascade breaks.
#[test]
fn changing_a_level_reaches_the_levels_below_it() {
    let xml = emit(&resolve("[poetry.q1]\nsmallcaps = true\n"));

    for level in 1..=4 {
        let key = format!("poetry.q{level}");
        let line = style_lines(&xml)
            .into_iter()
            .find(|l| l.contains(&format!("for=\"{key}\"")))
            .unwrap_or_else(|| panic!("no rule for `{key}`"));
        assert!(
            line.contains("smallcaps=\"true\""),
            "`{key}` did not inherit it:\n  {line}"
        );
    }

    // But not into another family.
    let qr = style_lines(&xml)
        .into_iter()
        .find(|l| l.contains("for=\"poetry.qr1\""))
        .expect("qr1 is styled");
    assert!(
        !qr.contains("smallcaps"),
        "`poetry.qr1` is not below `poetry.q1`:\n  {qr}"
    );
}

/// One property at a time: setting one leaves the rest of that selector's
/// rule as the cascade had it.
#[test]
fn changing_one_property_leaves_the_others_on_that_selector() {
    let before = style_lines(&emit(&cascade::resolve(None, false).0))
        .into_iter()
        .find(|l| l.contains("for=\"chapter\""))
        .expect("chapter is styled");
    assert!(before.contains("font_size=\"21pt\""));

    let after = style_lines(&emit(&resolve("[chapter]\nweight = 300\n")))
        .into_iter()
        .find(|l| l.contains("for=\"chapter\""))
        .expect("chapter is still styled");

    assert!(after.contains("weight=\"300\""), "{after}");
    assert!(
        after.contains("font_size=\"21pt\""),
        "the size was not named, so it should not have moved: {after}"
    );
}

fn golden_path(name: &str) -> Utf8PathBuf {
    corpus::root()
        .parent()
        .expect("workspace root")
        .join("tests/golden")
        .join(format!("{name}.xml"))
}
