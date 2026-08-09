//! P2.3 — embedded defaults, field-by-field merge, and the version gate.

use biblecompose_config::provenance::Origin;
use biblecompose_config::settings::{self, Settings, SCHEMA_VERSION};
use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::{Diagnostics, Severity};

fn project(body: &str) -> ConfigDocument {
    ConfigDocument::parse("biblecompose.toml", body.to_owned()).expect("the fixture parses")
}

fn resolve(body: &str) -> (Settings, Diagnostics) {
    let doc = project(body);
    let (settings, diagnostics) = settings::resolve(Some(&doc));
    (settings, diagnostics)
}

fn messages(d: &Diagnostics) -> Vec<String> {
    d.iter().map(|d| d.to_string()).collect()
}

// -------------------------------------------------------------- CFG-001

/// The one test that makes the panics in the resolver legitimate: the file
/// compiled into the executable parses, and every key the schema asks for is
/// present and valid. If this fails, no release can reach a user.
#[test]
fn the_embedded_defaults_are_valid() {
    let (_, diagnostics) = settings::resolve(None);
    assert!(
        diagnostics.is_empty(),
        "the built-in defaults are not clean: {:?}",
        messages(&diagnostics)
    );
}

/// CFG-001: a USFM-only folder builds, because every value has an answer.
#[test]
fn a_folder_with_no_settings_file_gets_a_complete_set() {
    let s = Settings::builtin();

    assert_eq!(s.page.size.to_string(), "6x9in");
    assert_eq!(*s.page.columns, 2);
    assert_eq!(*s.typography.font_family, "DejaVu Serif");
    assert_eq!(s.typography.font_size.to_sile(), "9.2pt");
    assert!(*s.numbering.show_verse_numbers);
    assert_eq!(s.output.file.as_str(), "output/bible.pdf");

    // There is no publication name that is right for everyone, so there is
    // none — the caller uses the folder's.
    assert!(s.project.name.is_none());
    // And no book selection, which is different from an empty one.
    assert!(s.books.include.is_none());
    assert!(s.books.order.is_empty());
}

/// ADR-005: every value says where it came from, and a built-in one says so
/// rather than pointing at a file that does not mention it.
#[test]
fn every_built_in_value_is_marked_built_in() {
    let s = Settings::builtin();
    assert_eq!(*s.page.size.origin(), Origin::Builtin);
    assert!(!s.page.size.is_overridden());
    assert_eq!(s.page.size.origin().to_string(), "built-in default");
    assert_eq!(s.page.size.origin().location(), None);
}

// -------------------------------------------------------------- CFG-002

/// The acceptance criterion: changing one key changes one key.
#[test]
fn one_override_leaves_every_other_default_intact() {
    let (s, d) = resolve("schema_version = 1\n[page]\nsize = \"a5\"\n");
    assert!(d.is_empty(), "{:?}", messages(&d));

    assert_eq!(s.page.size.to_string(), "148x210mm");
    assert!(s.page.size.is_overridden());

    // Everything else is untouched, including the rest of `[page]`.
    let builtin = Settings::builtin();
    assert_eq!(s.page.columns, builtin.page.columns);
    assert_eq!(s.page.margin_inner, builtin.page.margin_inner);
    assert_eq!(s.typography, builtin.typography);
    assert_eq!(s.notes, builtin.notes);
    assert_eq!(s.output, builtin.output);
}

#[test]
fn an_override_carries_the_line_it_was_written_on() {
    let (s, _) = resolve("schema_version = 1\n\n[typography]\nfont_family = \"Gentium Plus\"\n");

    assert_eq!(*s.typography.font_family, "Gentium Plus");
    let Origin::File(loc) = s.typography.font_family.origin() else {
        panic!("an overridden value must know its file");
    };
    assert_eq!(loc.path, "biblecompose.toml");
    assert_eq!(loc.line, Some(4));
}

/// The asymmetry, stated: a bad project value costs its own setting and
/// nothing else. `page.size = "quarto"` is not a reason to lose the margins.
#[test]
fn a_bad_value_is_reported_and_the_built_in_one_is_kept() {
    let (s, d) = resolve(
        "schema_version = 1\n\
         [page]\n\
         size = \"quarto\"\n\
         columns = 1\n",
    );

    assert_eq!(d.len(), 1, "{:?}", messages(&d));
    assert_eq!(d.iter().next().unwrap().code.as_str(), "CFG-003");

    assert_eq!(s.page.size.to_string(), "6x9in", "the default stands");
    assert!(!s.page.size.is_overridden());
    // And the valid key beside it still took effect.
    assert_eq!(*s.page.columns, 1);
    assert!(s.page.columns.is_overridden());
}

#[test]
fn several_bad_values_are_all_reported_at_once() {
    let (_, d) = resolve(
        "schema_version = 1\n\
         [page]\n\
         size = \"quarto\"\n\
         columns = 9\n\
         margin_top = \"3furlongs\"\n",
    );
    assert_eq!(d.len(), 3, "DIA-002: {:?}", messages(&d));
    assert!(d.iter().all(|d| d.location.is_some()));
}

#[test]
fn a_range_is_enforced_on_columns() {
    let (s, d) = resolve("schema_version = 1\n[page]\ncolumns = 0\n");
    assert_eq!(d.iter().next().unwrap().code.as_str(), "CFG-007");
    assert_eq!(*s.page.columns, 2);
}

// ------------------------------------------------------------ book lists

#[test]
fn a_book_list_is_taken_from_the_file_with_its_position() {
    let (s, d) = resolve("schema_version = 1\n[books]\norder = [\"JHN\", \"MAT\"]\n");
    assert!(d.is_empty(), "{:?}", messages(&d));
    assert_eq!(*s.books.order, ["JHN".to_owned(), "MAT".to_owned()]);
    assert!(s.books.order.is_overridden());
}

/// An empty `include` is a project that has selected nothing, which is not the
/// same as not having said anything.
#[test]
fn an_empty_include_is_different_from_an_absent_one() {
    let (absent, _) = resolve("schema_version = 1\n[books]\norder = []\n");
    assert!(absent.books.include.is_none());

    let (empty, _) = resolve("schema_version = 1\n[books]\ninclude = []\n");
    let include = empty.books.include.expect("the key was written");
    assert!(include.is_empty());
    assert!(include.is_overridden());
}

/// One bad element is not one bad list.
#[test]
fn a_bad_element_is_reported_and_the_rest_of_the_list_survives() {
    let (s, d) = resolve("schema_version = 1\n[books]\norder = [\"MAT\", 3, \"JHN\", true]\n");

    assert_eq!(*s.books.order, ["MAT".to_owned(), "JHN".to_owned()]);
    assert_eq!(
        d.len(),
        2,
        "every bad element, not the first: {:?}",
        messages(&d)
    );
    assert!(d.iter().all(|d| d.code.as_str() == "CFG-006"));
}

/// A list that is not a list at all is one problem with the setting, so the
/// built-in one stands.
#[test]
fn a_list_written_as_a_string_falls_back_to_the_default() {
    let (s, d) = resolve("schema_version = 1\n[books]\norder = \"MAT\"\n");
    assert_eq!(d.len(), 1);
    assert!(s.books.order.is_empty());
    assert!(!s.books.order.is_overridden());
}

// -------------------------------------------------------------- CFG-008

#[test]
fn the_current_version_is_accepted_silently() {
    let (_, d) = resolve(&format!("schema_version = {SCHEMA_VERSION}\n"));
    assert!(d.is_empty(), "{:?}", messages(&d));
}

/// A version from the future is one clear diagnostic — and, crucially, the
/// only one. Reading a file written for a schema we do not know produces a
/// cascade of complaints about keys that are perfectly correct in their own
/// version.
#[test]
fn an_unknown_version_produces_exactly_one_diagnostic() {
    let (s, d) = resolve(
        "schema_version = 99\n\
         [page]\n\
         size = \"a5\"\n\
         columns = 1\n\
         gribble = \"nonsense\"\n",
    );

    assert_eq!(d.len(), 1, "{:?}", messages(&d));
    let only = d.iter().next().unwrap();
    assert_eq!(only.severity, Severity::Error);
    assert_eq!(only.code.as_str(), "CFG-004");
    assert!(only.message.contains("version 99"));
    assert!(only
        .help
        .as_deref()
        .unwrap()
        .contains("update the application"));
    assert_eq!(only.location.as_ref().unwrap().line, Some(1));

    // Nothing in the file was used — not even the keys that would have been
    // valid, because we cannot know they mean here what they meant there.
    assert_eq!(s, Settings::builtin());
}

#[test]
fn a_version_from_the_past_says_so_differently() {
    let (s, d) = resolve("schema_version = 0\n[page]\ncolumns = 1\n");
    let only = d.iter().next().unwrap();
    assert!(
        only.message.contains("no longer understood"),
        "{}",
        only.message
    );
    assert_eq!(s, Settings::builtin());
}

/// Missing is a warning, not an error. There is one version, so assuming it is
/// safe, and refusing every file written before the key existed would punish
/// publishers for a problem versioning exists to prevent later.
#[test]
fn a_missing_version_warns_and_the_file_is_still_read() {
    let (s, d) = resolve("[page]\ncolumns = 1\n");

    assert_eq!(d.len(), 1);
    let only = d.iter().next().unwrap();
    assert_eq!(only.severity, Severity::Warning);
    assert!(!d.has_blocking());
    assert!(
        only.help.as_deref().unwrap().contains("schema_version = 1"),
        "the help must name the line to add: {only:?}"
    );

    assert_eq!(*s.page.columns, 1, "the rest of the file was still read");
}

#[test]
fn a_version_that_is_not_a_number_closes_the_file() {
    let (s, d) = resolve("schema_version = \"one\"\n[page]\ncolumns = 1\n");
    assert_eq!(d.iter().next().unwrap().code.as_str(), "CFG-006");
    assert_eq!(s, Settings::builtin());
}
