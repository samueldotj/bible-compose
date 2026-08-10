//! P2.8's half of the settings form: the schema describing itself, and an
//! edit that is only kept if the file still resolves.

use std::collections::BTreeSet;

use biblecompose_config::edit::{self, TomlFile};
use biblecompose_config::form::Kind;
use biblecompose_config::settings::{self, known_keys, Settings};
use biblecompose_config::ConfigDocument;

fn open(source: &str) -> TomlFile {
    TomlFile::new(
        ConfigDocument::parse("biblecompose.toml", source.to_owned()).expect("valid fixture"),
    )
}

fn resolve(source: &str) -> Settings {
    settings::resolve(Some(
        &ConfigDocument::parse("biblecompose.toml", source.to_owned()).expect("valid fixture"),
    ))
    .0
}

/// The form is generated from the schema, so a setting added to resolution
/// appears in it without anyone remembering to add a row. This is the test
/// that makes that true rather than hoped.
#[test]
fn the_form_has_a_row_for_every_setting() {
    let fields: BTreeSet<String> = Settings::builtin()
        .fields()
        .into_iter()
        .map(|f| f.key.to_owned())
        .collect();

    let mut expected = known_keys();
    // A declaration about which settings these are, not a setting.
    expected.remove("schema_version");

    assert_eq!(fields, expected);
}

#[test]
fn a_row_shows_the_value_and_where_it_came_from() {
    let s = resolve("schema_version = 1\n\n[page]\nsize = \"a5\"\n");
    let fields = s.fields();

    let size = fields.iter().find(|f| f.key == "page.size").unwrap();
    assert_eq!(size.kind, Kind::PageSize);
    assert_eq!(size.value, "148x210mm");
    assert_eq!(size.origin.location().unwrap().line, Some(4));

    let columns = fields.iter().find(|f| f.key == "page.columns").unwrap();
    assert_eq!(columns.kind, Kind::Integer);
    assert_eq!(columns.value, "2");
    assert!(columns.origin.is_builtin());
}

/// A length is shown in the unit it was written in, not normalised — a form
/// that answers `39.6pt` to someone who typed `0.55in` has changed the subject.
#[test]
fn a_length_row_shows_what_was_typed() {
    let s = resolve("schema_version = 1\n[page]\nmargin_top = \"0.55in\"\n");
    let row = s
        .fields()
        .into_iter()
        .find(|f| f.key == "page.margin_top")
        .unwrap();
    assert_eq!(row.value, "0.55in");
    assert_eq!(row.kind, Kind::Length);
}

/// An unset optional key is an empty field rather than a missing row: the
/// publisher needs somewhere to type the name they have not set yet.
#[test]
fn an_unset_optional_key_is_an_empty_row() {
    let row = Settings::builtin()
        .fields()
        .into_iter()
        .find(|f| f.key == "project.name")
        .unwrap();
    assert_eq!(row.value, "");
    assert!(row.origin.is_builtin());
}

#[test]
fn a_list_row_is_comma_separated_both_ways() {
    let s = resolve("schema_version = 1\n[books]\norder = [\"MAT\", \"JHN\"]\n");
    let row = s
        .fields()
        .into_iter()
        .find(|f| f.key == "books.order")
        .unwrap();
    assert_eq!(row.value, "MAT, JHN");

    let read = Kind::List.read(" MAT ,JHN , ");
    assert_eq!(
        read,
        edit::SettingValue::List(vec!["MAT".to_owned(), "JHN".to_owned()])
    );
}

#[test]
fn reading_a_field_back_matches_the_control_it_came_from() {
    assert_eq!(Kind::Integer.read(" 3 "), edit::SettingValue::Int(3));
    assert_eq!(Kind::Boolean.read("true"), edit::SettingValue::Bool(true));
    assert_eq!(Kind::Boolean.read("no"), edit::SettingValue::Bool(false));
    assert_eq!(
        Kind::Length.read(" 0.55in "),
        edit::SettingValue::Str("0.55in".to_owned())
    );
    // A number field that somehow holds text stays text, so the *settings*
    // reader reports it rather than this one guessing a number.
    assert_eq!(
        Kind::Integer.read("two"),
        edit::SettingValue::Str("two".to_owned())
    );
}

// ------------------------------------------------- validated writes

#[test]
fn a_good_value_is_written() {
    let mut file = open("schema_version = 1\n[page]\nsize = \"6x9in\"\n");
    edit::set_validated(&mut file, "page.size", "a5".into(), &edit::settings_check)
        .expect("a5 is a page size");
    assert!(file.to_toml().contains("size = \"a5\""));
}

/// The form field is text, and text can say "quarto". The edit is made on a
/// copy, the copy is resolved by the same reader that resolves a hand-written
/// file, and a new complaint means the file is left alone.
#[test]
fn a_value_the_reader_rejects_leaves_the_file_untouched() {
    let before = "schema_version = 1\n[page]\nsize = \"6x9in\"\n";
    let mut file = open(before);

    let refused = edit::set_validated(
        &mut file,
        "page.size",
        "quarto".into(),
        &edit::settings_check,
    )
    .expect_err("quarto is not a page size");

    assert_eq!(refused.len(), 1);
    assert_eq!(refused.iter().next().unwrap().code.as_str(), "CFG-003");
    assert_eq!(file.to_toml(), before, "nothing may have been written");
}

/// A file that already has a problem elsewhere must still be editable — the
/// test is "did this edit make it worse", not "is it perfect".
#[test]
fn an_existing_problem_does_not_block_editing_a_different_key() {
    let mut file = open("schema_version = 1\n[page]\ncolumns = 99\n");
    edit::set_validated(&mut file, "page.size", "a5".into(), &edit::settings_check)
        .expect("the bad columns value is not this edit's fault");
    assert!(file.to_toml().contains("size = \"a5\""));
    assert!(file.to_toml().contains("columns = 99"), "left as it was");
}

/// An unknown key is a warning, not an error — but it is still a new
/// complaint, so writing one is refused rather than quietly accepted.
#[test]
fn writing_a_key_the_schema_does_not_know_is_refused() {
    let before = "schema_version = 1\n";
    let mut file = open(before);
    let refused = edit::set_validated(&mut file, "page.wdith", "6in".into(), &edit::settings_check)
        .expect_err("not a setting");
    assert_eq!(refused.iter().next().unwrap().code.as_str(), "CFG-002");
    assert_eq!(file.to_toml(), before);
}
