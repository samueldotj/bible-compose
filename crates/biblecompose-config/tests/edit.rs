//! P2.7 — CFG-005 to CFG-007: a GUI edit changes one key and nothing else,
//! and a reset restores the built-in value.

use biblecompose_config::edit::{SettingValue, TomlFile};
use biblecompose_config::settings::{self, Settings};
use biblecompose_config::ConfigDocument;

/// A file with everything a careless rewrite would destroy: comments above,
/// beside and between; an author's own key order; alignment; blank lines.
const HANDWRITTEN: &str = "\
# My Bible.
#
# The margins are wider on the inside because the binding eats 3mm.

schema_version = 1

[page]
size         = \"6x9in\"   # trade paperback
columns      = 2
margin_inner = \"0.70in\"  # binding
margin_outer = \"0.50in\"


# Typography last, because it changes least.
[typography]
font_family = 'Gentium Plus'
font_size   = \"11.5pt\"
";

fn open(source: &str) -> TomlFile {
    let doc = ConfigDocument::parse("biblecompose.toml", source.to_owned()).expect("valid fixture");
    TomlFile::new(doc)
}

fn resolve(source: &str) -> Settings {
    let doc = ConfigDocument::parse("biblecompose.toml", source.to_owned()).expect("valid fixture");
    settings::resolve(Some(&doc)).0
}

/// The acceptance criterion: one key changes, every comment, blank line and
/// key order stays.
#[test]
fn an_edit_to_one_key_leaves_the_rest_of_the_file_alone() {
    let mut file = open(HANDWRITTEN);
    file.set("typography.font_size", "12pt");
    let after = file.to_toml();

    // Not merely "the line was replaced": the key's own alignment padding
    // survives too, so the only thing that differs in the whole file is the
    // value between the quotes.
    assert_eq!(
        after,
        HANDWRITTEN.replace("\"11.5pt\"", "\"12pt\""),
        "only the value may differ"
    );
    assert!(after.contains("font_size   = \"12pt\""), "alignment kept");

    // Said again from the other side, because the assertion above would also
    // pass if the whole file had been rewritten to coincidentally match.
    assert!(after.contains("# The margins are wider on the inside because the binding eats 3mm."));
    assert!(after.contains("size         = \"6x9in\"   # trade paperback"));
    assert!(
        after.contains("font_family = 'Gentium Plus'"),
        "single quotes kept"
    );
    assert!(
        after.contains("\n\n\n# Typography last"),
        "blank lines kept"
    );
}

#[test]
fn an_edit_is_visible_to_the_next_resolution() {
    let mut file = open(HANDWRITTEN);
    file.set("page.columns", 1_i64);

    let s = resolve(&file.to_toml());
    assert_eq!(*s.page.columns, 1);
    assert!(s.page.columns.is_overridden());
}

/// A length goes back in the unit it is carried in. `39.6pt` is the same
/// measurement as `0.55in` and a worse thing to find in your own file.
#[test]
fn a_length_is_written_in_the_unit_it_is_carried_in() {
    let mut file = open(HANDWRITTEN);
    let margin = resolve(HANDWRITTEN).page.margin_inner;
    file.set("page.margin_outer", *margin);

    assert!(
        file.to_toml().contains("margin_outer = \"0.7in\""),
        "{}",
        file.to_toml()
    );
}

#[test]
fn setting_a_key_whose_table_is_missing_creates_a_real_header() {
    let mut file = open("schema_version = 1\n");
    file.set("notes.show_footnotes", false);

    let after = file.to_toml();
    assert!(
        after.contains("[notes]"),
        "a header, not an inline table: {after}"
    );
    assert!(after.contains("show_footnotes = false"));
    assert!(resolve(&after).notes.show_footnotes.is_overridden());
}

/// A dotted key already in the file is updated where it is, not duplicated
/// into a new table.
#[test]
fn a_dotted_key_is_updated_in_place() {
    let mut file = open("schema_version = 1\n[page]\nmargin.inner = \"0.5in\"\n");
    file.set("page.margin.inner", "0.9in");

    let after = file.to_toml();
    assert_eq!(
        after,
        "schema_version = 1\n[page]\nmargin.inner = \"0.9in\"\n"
    );
}

// ----------------------------------------------------------- CFG-007

/// Reset removes the key. Writing the built-in value instead would look
/// identical today and diverge silently the first time a release changes a
/// default.
#[test]
fn reset_removes_the_key_rather_than_writing_the_default_into_the_file() {
    let mut file = open(HANDWRITTEN);
    assert!(file.reset("page.columns"));

    let after = file.to_toml();
    assert!(!after.contains("columns"), "{after}");
    // The line above it and the line below it are untouched.
    assert!(after.contains("size         = \"6x9in\"   # trade paperback"));
    assert!(after.contains("margin_inner = \"0.70in\"  # binding"));

    // And the built-in value is back in force.
    let s = resolve(&after);
    assert_eq!(*s.page.columns, *Settings::builtin().page.columns);
    assert!(!s.page.columns.is_overridden());
}

#[test]
fn resetting_something_that_was_never_set_changes_nothing() {
    let mut file = open(HANDWRITTEN);
    assert!(
        !file.reset("page.header_gap"),
        "there was nothing to remove"
    );
    assert!(!file.reset("nothing.at.all"));
    assert_eq!(file.to_toml(), HANDWRITTEN);
}

/// An emptied table is left behind: `[page]` with nothing under it is inert,
/// and a publisher who wrote that header did not ask for it to disappear
/// because the last key inside it was reset.
#[test]
fn an_emptied_table_stays() {
    let mut file = open("schema_version = 1\n\n# The page.\n[page]\ncolumns = 1\n");
    assert!(file.reset("page.columns"));
    assert_eq!(
        file.to_toml(),
        "schema_version = 1\n\n# The page.\n[page]\n"
    );
}

/// Set then reset returns the file to what it was, byte for byte, when the key
/// was the only thing on its line.
#[test]
fn set_then_reset_is_the_identity() {
    let start = "schema_version = 1\n\n[page]\nsize = \"a5\"\n";
    let mut file = open(start);
    file.set("page.columns", 3_i64);
    assert!(file.to_toml().contains("columns = 3"));
    assert!(file.reset("page.columns"));
    assert_eq!(file.to_toml(), start);
}

// ------------------------------------------------------------- saving

#[test]
fn saving_replaces_the_file_and_leaves_no_temporary_behind() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("biblecompose.toml"))
        .expect("UTF-8 temp path");
    std::fs::write(path.as_std_path(), HANDWRITTEN).expect("write the fixture");

    let mut file = TomlFile::new(ConfigDocument::read(&path).expect("reads back"));
    file.set("page.columns", 1_i64);
    file.save().expect("saves");

    let written = std::fs::read_to_string(path.as_std_path()).expect("reads back");
    assert!(written.contains("columns      = 1"), "{written}");
    assert!(written.contains("# The margins are wider"));

    let leftovers: Vec<String> = std::fs::read_dir(dir.path())
        .expect("lists")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "biblecompose.toml")
        .collect();
    assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
}

/// CFG-005 end to end: the change is still there when the project is reopened.
#[test]
fn a_saved_change_survives_reopening() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("biblecompose.toml"))
        .expect("UTF-8 temp path");
    std::fs::write(path.as_std_path(), HANDWRITTEN).expect("write the fixture");

    let mut file = TomlFile::new(ConfigDocument::read(&path).expect("reads"));
    file.set("typography.font_family", "Noto Serif Tamil");
    file.save().expect("saves");

    let reopened = ConfigDocument::read(&path).expect("reopens");
    let (s, d) = settings::resolve(Some(&reopened));
    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(*s.typography.font_family, "Noto Serif Tamil");
    assert!(s.typography.font_family.is_overridden());
}

/// A project with no settings file at all gets one that says what it is.
#[test]
fn a_new_file_starts_with_a_version_and_an_explanation() {
    let mut file = TomlFile::create(
        "biblecompose.toml",
        &TomlFile::settings_header(settings::SCHEMA_VERSION),
    );
    file.set("page.size", "a5");

    let text = file.to_toml();
    assert!(text.starts_with("# BibleCompose settings."));
    assert!(text.contains("schema_version = 1"));

    let (s, d) = settings::resolve(Some(
        &ConfigDocument::parse("biblecompose.toml", text).expect("valid"),
    ));
    assert!(d.is_empty(), "a generated file must be clean");
    assert_eq!(s.page.size.to_string(), "148x210mm");
}

#[test]
fn a_list_is_written_as_an_array() {
    let mut file = open("schema_version = 1\n");
    file.set(
        "books.order",
        SettingValue::List(vec!["MAT".to_owned(), "JHN".to_owned()]),
    );
    assert!(file.to_toml().contains(r#"order = ["MAT", "JHN"]"#));
    assert_eq!(*resolve(&file.to_toml()).books.order, ["MAT", "JHN"]);
}
