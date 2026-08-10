//! P3.4 — the resolved style map, as data in the emitted document.
//!
//! ADR-002's rule is that Scripture reaches SILE as a text node and never as
//! syntax. Style values are held to the same line: they arrive as attributes,
//! so a style file cannot carry a command — and style files travel with
//! projects, by email, from third parties.

use biblecompose_app::backend_input::style_rules;
use biblecompose_config::{cascade, ConfigDocument, ResolvedStyles};
use biblecompose_scripture::fixtures;
use biblecompose_testkit::{corpus, golden};
use camino::Utf8PathBuf;

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

/// The golden the roadmap asks for: every property, in the document.
///
/// A sheet that sets all nine on one selector, so a property added to the
/// schema and forgotten in the translation shows up here as a missing
/// attribute rather than as a style the page ignores.
#[test]
fn the_full_property_set_reaches_the_document() {
    let styles = resolve(
        "[chapter]\n\
         font_size = \"18pt\"\n\
         weight = 700\n\
         italic = true\n\
         smallcaps = true\n\
         space_above = \"6pt\"\n\
         space_below = \"3pt\"\n\
         indent = \"0.5in\"\n\
         raise = \"2pt\"\n\
         align = \"center\"\n",
    );

    let emitted = biblecompose_app::emit(&fixtures::john_1_1_5(), &styles);
    golden::assert_matches(&golden_path("style_full_set"), &emitted.xml);
}

/// Every property crosses, and crosses in points where it is a length.
#[test]
fn every_property_appears_as_an_attribute() {
    let styles = resolve(
        "[chapter]\n\
         font_size = \"18pt\"\n\
         weight = 700\n\
         italic = true\n\
         smallcaps = false\n\
         space_above = \"6pt\"\n\
         space_below = \"3pt\"\n\
         indent = \"0.5in\"\n\
         raise = \"2pt\"\n\
         align = \"center\"\n",
    );

    let rules = style_rules(&styles);
    let chapter = rules
        .iter()
        .find(|r| r.selector == "chapter")
        .expect("chapter is styled");

    let names: Vec<&str> = chapter.properties.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "font_size",
            "weight",
            "italic",
            "smallcaps",
            "space_above",
            "space_below",
            "indent",
            "raise",
            "align"
        ],
        "in the schema's order, so the bytes are a function of the values"
    );

    let value = |name: &str| {
        chapter
            .properties
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .unwrap()
    };
    assert_eq!(value("indent"), "36pt", "0.5in, in points");
    assert_eq!(value("italic"), "true");
    assert_eq!(value("smallcaps"), "false", "false is said, not omitted");
    assert_eq!(value("align"), "center");
}

/// The names are one word from the TOML key to the Lua field, so there is no
/// translation table for the file and the class to disagree through.
#[test]
fn a_property_is_called_the_same_thing_everywhere() {
    let styles = resolve("[poetry.q1]\nspace_above = \"4pt\"\n");
    let rules = style_rules(&styles);
    let q1 = rules.iter().find(|r| r.selector == "poetry.q1").unwrap();

    assert!(q1.properties.iter().any(|(n, _)| n == "space_above"));
    assert!(
        biblecompose_config::style::PROPERTIES.contains(&"space_above"),
        "the same name the schema uses"
    );
}

/// A selector with nothing to say is not written. An empty entry means "renders
/// as body text", and a line of XML per paragraph marker saying nothing is
/// eighty lines of nothing.
#[test]
fn a_selector_with_no_properties_is_left_out() {
    let rules = style_rules(&cascade::resolve(None, false).0);
    assert!(
        !rules.iter().any(|r| r.selector == "paragraph.p"),
        "an unstyled paragraph should not be in the document"
    );
    assert!(rules.iter().any(|r| r.selector == "chapter"));
    assert!(!rules.is_empty());
}

/// SILE-005: the same styles produce the same bytes.
#[test]
fn the_styles_block_is_byte_stable() {
    let styles = cascade::resolve(None, false).0;
    let first = biblecompose_app::emit(&fixtures::john_1_1_5(), &styles).xml;
    for _ in 0..5 {
        assert_eq!(
            biblecompose_app::emit(&fixtures::john_1_1_5(), &styles).xml,
            first
        );
    }
}

/// ADR-002 in the case that matters: a style value that looks like markup is
/// escaped by the serializer, not by care.
#[test]
fn a_style_value_cannot_carry_syntax() {
    // `font_family` is a setting rather than a style, so the closest a sheet
    // can come is an alignment or a unit — both are validated. What this
    // asserts is the property the *emitter* provides regardless: whatever a
    // rule holds, it lands as an attribute value.
    let rules = vec![biblecompose_sile::StyleRule {
        selector: "chapter".to_owned(),
        properties: vec![(
            "font_size".to_owned(),
            "\"/><script>oops</script><x a=\"".to_owned(),
        )],
    }];

    let xml = biblecompose_sile::emit(&fixtures::john_1_1_5(), &rules).xml;
    assert!(!xml.contains("<script>"), "{xml}");
    assert!(xml.contains("&quot;"), "the quotes are escaped: {xml}");
}

fn golden_path(name: &str) -> Utf8PathBuf {
    corpus::root()
        .parent()
        .expect("workspace root")
        .join("tests/golden")
        .join(format!("{name}.xml"))
}
