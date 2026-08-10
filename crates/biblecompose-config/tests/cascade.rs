//! P3.2 — the cascade, single-parent inheritance, and cycle detection.

use biblecompose_config::cascade::{self, ResolvedStyles};
use biblecompose_config::provenance::Origin;
use biblecompose_config::selector::StyleSelector;
use biblecompose_config::style::Align;
use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::{Diagnostics, Severity};
use biblecompose_scripture::{CharStyle, HeadingStyle, PoetryStyle};

fn resolve(source: &str) -> (ResolvedStyles, Diagnostics) {
    let doc = ConfigDocument::parse("styles.toml", source.to_owned()).expect("valid fixture");
    cascade::resolve(Some(&doc), false)
}

fn messages(d: &Diagnostics) -> Vec<String> {
    d.iter().map(|d| d.to_string()).collect()
}

const Q1: StyleSelector = StyleSelector::Poetry(PoetryStyle::Q, 1);
const Q2: StyleSelector = StyleSelector::Poetry(PoetryStyle::Q, 2);
const Q3: StyleSelector = StyleSelector::Poetry(PoetryStyle::Q, 3);

// ---------------------------------------------------------------- STY-002

/// The acceptance criterion: an override changes only what it names.
#[test]
fn an_override_changes_only_what_it_names() {
    let (styles, d) = resolve("[chapter]\nweight = 400\n");
    assert!(d.is_empty(), "{:?}", messages(&d));

    let chapter = styles.get(StyleSelector::Chapter);
    assert_eq!(chapter.style.weight, Some(400), "named, so changed");
    assert_eq!(
        chapter.style.font_size.map(|l| l.to_sile()),
        Some("21pt".to_owned()),
        "unnamed, so the built-in value stands"
    );

    // And nothing else moved.
    let builtin = cascade::resolve(None, false).0;
    assert_eq!(
        styles.get(StyleSelector::Verse),
        builtin.get(StyleSelector::Verse)
    );
}

#[test]
fn with_no_project_sheet_everything_is_built_in() {
    let (styles, d) = cascade::resolve(None, false);
    assert!(d.is_empty(), "{:?}", messages(&d));
    assert!(!styles.is_empty());

    let chapter = styles.get(StyleSelector::Chapter);
    assert_eq!(chapter.origin_of("weight"), Some(&Origin::Builtin));
}

#[test]
fn a_project_value_records_the_line_it_came_from() {
    let (styles, _) = resolve("[chapter]\n\nweight = 400\n");
    let chapter = styles.get(StyleSelector::Chapter);

    let Some(Origin::File(loc)) = chapter.origin_of("weight") else {
        panic!("an overridden property must know its file");
    };
    assert_eq!(loc.line, Some(3));
    // The property beside it is still the built-in one, and says so.
    assert_eq!(chapter.origin_of("font_size"), Some(&Origin::Builtin));
}

/// A property that was rejected is not in force, so nothing may point at the
/// line it was written on as though it were.
#[test]
fn a_rejected_property_does_not_claim_its_line() {
    let (styles, d) = resolve("[chapter]\nfont_size = \"enormous\"\n");
    assert_eq!(d.len(), 1, "{:?}", messages(&d));

    let chapter = styles.get(StyleSelector::Chapter);
    assert_eq!(
        chapter.style.font_size.map(|l| l.to_sile()),
        Some("21pt".to_owned()),
        "the built-in size stands"
    );
    assert_eq!(chapter.origin_of("font_size"), Some(&Origin::Builtin));
}

// ---------------------------------------------------------------- STY-007

/// The acceptance criterion: `q2` inherits poetry properties from `q1`.
#[test]
fn a_deeper_level_inherits_from_the_one_above() {
    let (styles, d) = resolve("[poetry.q1]\nitalic = true\n");
    assert!(d.is_empty(), "{:?}", messages(&d));

    let q2 = styles.get(Q2);
    assert_eq!(q2.style.italic, Some(true), "inherited from q1");
    assert_eq!(
        q2.origin_of("italic"),
        Some(&Origin::Inherited { from: Q1 })
    );

    // But its own indent wins over the inherited one.
    assert_eq!(q2.origin_of("indent"), Some(&Origin::Builtin));
    assert!(
        q2.style.indent.unwrap().points() > styles.get(Q1).style.indent.unwrap().points(),
        "q2 is indented further than q1"
    );
}

/// The inheritance is transitive, and each step says which one it came from.
#[test]
fn inheritance_walks_the_whole_chain() {
    let (styles, _) = resolve("[poetry.q1]\nsmallcaps = true\n");
    assert_eq!(styles.get(Q3).style.smallcaps, Some(true));
    assert_eq!(
        styles.get(Q3).origin_of("smallcaps"),
        Some(&Origin::Inherited { from: Q1 }),
        "named by where the value actually is, not by the first hop"
    );
}

/// What keeps the built-in table finite: a level deeper than it goes resolves
/// against the deepest one that exists.
#[test]
fn a_level_past_the_built_in_table_still_resolves() {
    let (styles, d) = resolve("");
    assert!(d.is_empty());

    let q7 = StyleSelector::parse("poetry.q7").expect("a deep level is a selector");
    let deep = styles.get(q7);
    assert!(
        deep.style.indent.is_some(),
        "a q7 line must still be indented"
    );
    assert_eq!(
        deep.style.indent,
        styles
            .get(StyleSelector::Poetry(PoetryStyle::Q, 4))
            .style
            .indent,
        "it takes the deepest level the table defines"
    );
}

/// A family whose deeper levels the built-in sheet never mentions still
/// follows a publisher's restyling of the first.
#[test]
fn restyling_a_first_level_carries_to_the_rest_of_its_family() {
    let (styles, _) = resolve("[heading.s1]\nweight = 400\nitalic = true\n");
    let s2 = styles.get(StyleSelector::Heading(HeadingStyle::S, 2));

    assert_eq!(s2.style.italic, Some(true));
    assert_eq!(
        s2.origin_of("italic"),
        Some(&Origin::Inherited {
            from: StyleSelector::Heading(HeadingStyle::S, 1)
        })
    );
}

/// STY-007's named parent, across families.
#[test]
fn a_style_may_name_its_parent() {
    let (styles, d) = resolve(
        "[character.bd]\nweight = 800\n\n\
         [character.sig]\ninherits = \"character.bd\"\n",
    );
    assert!(d.is_empty(), "{:?}", messages(&d));

    let sig = styles.get(StyleSelector::Character(CharStyle::Sig));
    assert_eq!(sig.style.weight, Some(800));
    assert_eq!(
        sig.origin_of("weight"),
        Some(&Origin::Inherited {
            from: StyleSelector::Character(CharStyle::Bd)
        })
    );
    // Its own built-in italic is nearer than the parent, so it survives.
    assert_eq!(sig.style.italic, Some(true));
    assert_eq!(sig.origin_of("italic"), Some(&Origin::Builtin));
}

/// A named parent replaces the implied one rather than adding to it — one
/// parent, so the chain is a walk and not a graph.
#[test]
fn a_named_parent_replaces_the_implied_one() {
    let (styles, _) = resolve(
        "[poetry.q1]\nsmallcaps = true\n\n\
         [character.bd]\nitalic = true\n\n\
         [poetry.q2]\ninherits = \"character.bd\"\n",
    );

    let q2 = styles.get(Q2);
    assert_eq!(q2.style.italic, Some(true), "from the named parent");
    assert_eq!(
        q2.style.smallcaps, None,
        "q1 is no longer in the chain, so its smallcaps is not"
    );
}

#[test]
fn inheriting_from_something_that_is_not_an_element_is_reported() {
    let (_, d) = resolve("[poetry.q2]\ninherits = \"poetry.gribble\"\n");
    assert_eq!(d.len(), 1, "{:?}", messages(&d));
    assert_eq!(d.iter().next().unwrap().code.as_str(), "STY-001");
}

// -------------------------------------------------------------- cycles

/// The acceptance criterion: one diagnostic naming the cycle, not a stack
/// overflow. Reaching this assertion at all is most of the test.
#[test]
fn a_cycle_is_one_diagnostic_naming_it() {
    let (styles, d) = resolve(
        "[poetry.q1]\ninherits = \"poetry.qr1\"\n\n\
         [poetry.qr1]\ninherits = \"poetry.q1\"\n",
    );

    // One per *cycle*. Four levels of `q` and four of `qr` all walk into this
    // same loop; eight identical complaints would bury the one fact.
    let cycles: Vec<&biblecompose_diagnostics::Diagnostic> =
        d.iter().filter(|d| d.code.as_str() == "STY-003").collect();
    assert_eq!(cycles.len(), 1, "{:?}", messages(&d));

    let first = cycles[0];
    assert_eq!(first.severity, Severity::Error);
    assert!(first.message.contains("poetry.q1"), "{}", first.message);
    assert!(first.message.contains("poetry.qr1"), "{}", first.message);
    assert!(
        first.message.contains('→'),
        "the loop is drawn: {}",
        first.message
    );
    assert!(
        first
            .message
            .ends_with(&first.message[first.message.rfind(':').unwrap() + 2..]),
        "and closed on the selector it came back to"
    );
    assert!(first.location.is_some(), "and pointed at");

    // Everything else still resolved.
    assert!(!styles.get(StyleSelector::Chapter).style.is_empty());
}

#[test]
fn a_style_inheriting_from_itself_is_caught_when_it_is_read() {
    let (_, d) = resolve("[chapter]\ninherits = \"chapter\"\n");
    assert_eq!(d.len(), 1, "{:?}", messages(&d));
    let only = d.iter().next().unwrap();
    assert_eq!(only.code.as_str(), "STY-003");
    assert!(only.message.contains("inherits from itself"));
}

/// A longer loop is still one pass and still named.
#[test]
fn a_three_step_cycle_terminates() {
    let (_, d) = resolve(
        "[poetry.q1]\ninherits = \"poetry.qr1\"\n\n\
         [poetry.qr1]\ninherits = \"poetry.qc1\"\n\n\
         [poetry.qc1]\ninherits = \"poetry.q1\"\n",
    );
    assert!(
        d.iter().any(|d| d.code.as_str() == "STY-003"),
        "{:?}",
        messages(&d)
    );
    assert!(
        d.iter().all(|d| d.message.len() < 400),
        "no runaway message"
    );
}

// -------------------------------------------------------------- coverage

/// Every selector resolves to something, so the emitter never has to ask what
/// to do about a marker the model produced.
#[test]
fn every_selector_resolves() {
    let (styles, _) = cascade::resolve(None, false);
    for selector in StyleSelector::all() {
        let _ = styles.get(selector);
    }
    assert!(styles.len() >= StyleSelector::all().len());
}

/// Every property that is set says where it came from — the exhaustiveness
/// check that stops a property being cascaded without an origin.
#[test]
fn every_resolved_property_has_an_origin() {
    let (styles, _) = resolve("[chapter]\nweight = 400\nalign = \"center\"\n");

    for (selector, resolved) in styles.iter() {
        let s = &resolved.style;
        for (name, set) in [
            ("font_size", s.font_size.is_some()),
            ("weight", s.weight.is_some()),
            ("italic", s.italic.is_some()),
            ("smallcaps", s.smallcaps.is_some()),
            ("space_above", s.space_above.is_some()),
            ("space_below", s.space_below.is_some()),
            ("indent", s.indent.is_some()),
            ("raise", s.raise.is_some()),
            ("align", s.align.is_some()),
        ] {
            assert_eq!(
                set,
                resolved.origin_of(name).is_some(),
                "`{selector}` property `{name}`: a value without an origin, or the reverse"
            );
        }
    }
}

#[test]
fn an_alignment_survives_the_cascade() {
    let (styles, _) = cascade::resolve(None, false);
    assert_eq!(
        styles
            .get(StyleSelector::Poetry(PoetryStyle::Qc, 1))
            .style
            .align,
        Some(Align::Center)
    );
}
