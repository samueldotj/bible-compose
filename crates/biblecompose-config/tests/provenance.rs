//! P2.6 — every resolved value reports where it came from (ADR-005).
//!
//! The typed fields carried an origin from P2.3, because ADR-005's whole
//! argument is that retrofitting provenance is what makes it get dropped. What
//! this adds is the string-keyed index over them and, more importantly, the
//! test that the index is *complete* — a field added to resolution without an
//! origin fails here rather than showing "default" in an inspector for a value
//! the publisher wrote themselves.

use std::collections::BTreeSet;

use biblecompose_config::provenance::Origin;
use biblecompose_config::settings::{self, known_keys, Settings};
use biblecompose_config::ConfigDocument;

/// Sets every key the release understands, so a resolution of it should have
/// an origin in a file for every single one.
const EVERYTHING: &str = "\
schema_version = 1
strict = true

[project]
name = \"My Bible\"
language = \"ta\"
author = \"A Bible Society\"
subject = \"New Testament\"

[books]
order = [\"MAT\"]
include = [\"MAT\"]

[page]
size = \"a5\"
columns = 1
margin_top = \"1in\"
margin_bottom = \"1in\"
margin_inner = \"1in\"
margin_outer = \"1in\"
column_gap = \"0.2in\"
header_gap = \"0.3in\"
footer_gap = \"0.3in\"

[typography]
font_family = \"Gentium Plus\"
font_size = \"11pt\"
leading = \"13pt\"
hyphenation = false
justify = false
keep_poetry_indentation = false

[contents]
show_book_introductions = false
show_introductory_outlines = false
show_section_headings = false

[numbering]
show_chapter_numbers = false
show_verse_numbers = false
hide_first_verse_number = true
show_chapter_labels = false

[notes]
show_footnotes = false
show_cross_references = false
footnote_callers = \"symbols\"
cross_reference_callers = \"none\"
restart_numbering = \"per_book\"
cross_reference_placement = \"inline\"

[headers]
header_left = \"alt_book_name\"
header_center = \"empty\"
header_right = \"empty\"
footer_left = \"first_reference\"
footer_center = \"empty\"
footer_right = \"last_reference\"

[assets]
missing_figure = \"omit\"

[output]
keep_intermediates = true
anchors = \"verse\"
name = \"Proof copy\"
";

fn resolve(body: &str) -> Settings {
    let doc = ConfigDocument::parse("biblecompose.toml", body.to_owned()).expect("valid fixture");
    let (settings, diagnostics) = settings::resolve(Some(&doc));
    assert!(
        diagnostics.is_empty(),
        "fixture should be clean: {:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    settings
}

/// The exhaustiveness test. A field read during resolution but missing from
/// the index would show as "built-in" for a value the publisher wrote.
#[test]
fn the_index_covers_every_key_that_has_a_value() {
    let s = resolve(EVERYTHING);

    let indexed: BTreeSet<String> = s.provenance.iter().map(|(k, _)| k.to_owned()).collect();
    let mut expected = known_keys();
    // Not a setting — a declaration about which settings these are, read
    // before resolution and not resolved into anything.
    expected.remove("schema_version");

    assert_eq!(
        indexed, expected,
        "every key resolution asks for must appear in the provenance index"
    );
}

/// The two keys with no built-in value are absent from the index when unset,
/// rather than reported as `Builtin`. Nothing chose them, and saying `Builtin`
/// would be a claim that something did.
#[test]
fn an_unset_optional_key_has_no_origin_at_all() {
    let s = Settings::builtin();
    for key in [
        "project.name",
        "project.author",
        "project.subject",
        "books.include",
        "output.name",
    ] {
        assert_eq!(s.provenance.get(key), None, "{key} was never chosen");
    }
    // The five above, and `schema_version`, which is a declaration about the
    // settings rather than one of them.
    assert_eq!(s.provenance.len(), known_keys().len() - 6);
}

#[test]
fn with_no_project_file_everything_is_built_in() {
    let s = Settings::builtin();
    assert!(
        s.provenance.iter().all(|(_, o)| o.is_builtin()),
        "some value claims a file: {:?}",
        s.provenance
            .iter()
            .filter(|(_, o)| !o.is_builtin())
            .collect::<Vec<_>>()
    );
    assert_eq!(s.provenance.overridden().count(), 0);
}

#[test]
fn a_file_that_sets_everything_leaves_nothing_built_in() {
    let s = resolve(EVERYTHING);
    let still_builtin: Vec<&str> = s
        .provenance
        .iter()
        .filter(|(_, o)| o.is_builtin())
        .map(|(k, _)| k)
        .collect();
    assert!(
        still_builtin.is_empty(),
        "these keys were written in the file but report as built-in: {still_builtin:?}"
    );

    for (key, origin) in s.provenance.iter() {
        let loc = origin
            .location()
            .unwrap_or_else(|| panic!("`{key}` has no location"));
        assert_eq!(loc.path, "biblecompose.toml");
        assert!(loc.line.is_some(), "`{key}` has a file but no line");
    }
}

/// CFG-007's input: which keys the publisher actually wrote, so the GUI knows
/// what it can offer to reset.
#[test]
fn overridden_lists_exactly_what_the_file_set() {
    let s = resolve("schema_version = 1\n[page]\nsize = \"a5\"\ncolumns = 1\n");
    assert_eq!(
        s.provenance.overridden().collect::<Vec<_>>(),
        ["page.columns", "page.size"],
        "in key order"
    );
}

/// The index and the typed field are written by the same expression, so they
/// cannot disagree. Checked anyway, because "cannot" is a claim about code
/// that will be edited.
#[test]
fn the_index_and_the_typed_field_agree() {
    let s = resolve("schema_version = 1\n\n[page]\nsize = \"a5\"\n");

    assert_eq!(s.provenance.get("page.size"), Some(s.page.size.origin()));
    let Some(Origin::File(loc)) = s.provenance.get("page.size") else {
        panic!("page.size was written in the file");
    };
    assert_eq!(loc.line, Some(4));

    assert_eq!(s.provenance.get("page.columns"), Some(&Origin::Builtin));
    assert!(!s.page.columns.is_overridden());
}

/// A value the file set badly falls back to the built-in one — and the index
/// says so, rather than pointing at the line that was rejected.
#[test]
fn a_rejected_value_reports_the_origin_of_what_was_actually_used() {
    let doc = ConfigDocument::parse(
        "biblecompose.toml",
        "schema_version = 1\n[page]\nsize = \"quarto\"\n".to_owned(),
    )
    .unwrap();
    let (s, d) = settings::resolve(Some(&doc));

    assert_eq!(d.len(), 1);
    assert_eq!(s.provenance.get("page.size"), Some(&Origin::Builtin));
    assert_eq!(
        s.provenance.overridden().count(),
        0,
        "nothing was successfully overridden, so nothing can be reset"
    );
}

/// `Builtin` renders as words rather than as a fake location — ADR-005 is
/// explicit that a fabricated `file:0:0` reads as a defect in the file.
#[test]
fn a_built_in_origin_shows_as_words() {
    let s = Settings::builtin();
    assert_eq!(
        s.provenance.get("page.size").unwrap().to_string(),
        "built-in default"
    );
}
