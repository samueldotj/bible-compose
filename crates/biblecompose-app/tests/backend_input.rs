//! P2.3 part 2 and P3.4 — resolved configuration reaching the page.
//!
//! The interesting failure here is not a wrong value, it is a value the class
//! has never heard of: SILE treats an undeclared class option as a hard error,
//! so an option the application sends and the class does not declare is a
//! build that dies at the typesetting step with a message about Lua. The last
//! test in this file is the one that stops that reaching a user.

use biblecompose_app::backend_input::class_options;
use biblecompose_config::{settings, ConfigDocument, Settings};
use biblecompose_testkit::repo_root;

fn options(toml: &str) -> Vec<(String, String)> {
    let doc = ConfigDocument::parse("biblecompose.toml", toml.to_owned()).expect("valid fixture");
    let (settings, diagnostics) = settings::resolve(Some(&doc));
    assert!(
        diagnostics.is_empty(),
        "the fixture should be clean: {:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    class_options(&settings)
}

fn get<'a>(options: &'a [(String, String)], key: &str) -> &'a str {
    options
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("no `{key}` among {options:?}"))
}

#[test]
fn the_built_in_settings_produce_the_page_the_class_produced_before_there_was_one() {
    let o = class_options(&Settings::builtin());

    assert_eq!(get(&o, "papersize"), "432pt x 648pt", "6x9in");
    assert_eq!(get(&o, "columns"), "2");
    // The class's own fallbacks are percentages of the page; these are those
    // percentages at 6x9in, so wiring settings through did not move any type.
    assert_eq!(get(&o, "margintop"), "58.32pt"); // 9%ph
    assert_eq!(get(&o, "marginbottom"), "77.76pt"); // 12%ph
    assert_eq!(get(&o, "margininner"), "47.52pt"); // 11%pw
    assert_eq!(get(&o, "marginouter"), "34.56pt"); // 8%pw
    assert_eq!(get(&o, "gutter"), "15.12pt"); // 3.5%pw
    assert_eq!(get(&o, "headsep"), "25.92pt"); // 4%ph
    assert_eq!(get(&o, "footsep"), "19.44pt"); // 3%ph

    assert_eq!(get(&o, "fontfamily"), "DejaVu Serif");
    assert_eq!(get(&o, "fontsize"), "9.2pt");
    assert_eq!(get(&o, "leading"), "11.2pt");
}

#[test]
fn a_setting_reaches_the_backend_as_points_however_it_was_written() {
    for written in ["0.55in", "39.6pt", "13.97mm"] {
        let o = options(&format!(
            "schema_version = 1\n[page]\nmargin_top = \"{written}\"\n"
        ));
        assert_eq!(
            get(&o, "margintop"),
            "39.6pt",
            "{written} must produce the same argument as every other spelling"
        );
    }
}

#[test]
fn booleans_cross_as_the_words_the_class_reads() {
    let o = options("schema_version = 1\n[numbering]\nshow_verse_numbers = false\n");
    assert_eq!(get(&o, "versenumbers"), "false");
    assert_eq!(get(&o, "chapternumbers"), "true");
}

#[test]
fn the_option_list_is_ordered_so_two_identical_builds_are_identical() {
    let a = class_options(&Settings::builtin());
    let b = class_options(&Settings::builtin());
    assert_eq!(a, b);
    assert_eq!(a.first().map(|(k, _)| k.as_str()), Some("papersize"));
}

/// ADR-005: nothing that reaches the backend can say where it came from.
///
/// The type system already guarantees this — `class_options` returns
/// `Vec<(String, String)>` and there is no way to put an `Origin` in one — so
/// what this checks is the thing a type cannot: that no value happens to
/// *contain* a file path.
#[test]
fn no_option_value_carries_a_file_path() {
    let o = options("schema_version = 1\n[page]\nsize = \"a5\"\n");
    for (key, value) in &o {
        assert!(
            !value.contains("biblecompose.toml") && !value.contains(':'),
            "`{key}` carries something that looks like a location: {value:?}"
        );
    }
}

/// The one that matters: every option the application sends is one the class
/// declares.
///
/// SILE errors on an undeclared class option, so a mismatch here is not a
/// wrong page — it is a build that fails inside Lua, at the last step, after
/// the publisher has waited for it. Read out of the class's own `OPTIONS`
/// table rather than a list kept here, because a second list is a second
/// thing to forget.
#[test]
fn every_option_the_application_sends_is_one_the_class_declares() {
    let class = repo_root().join("sile/classes/biblecompose.lua");
    let source = std::fs::read_to_string(class.as_std_path()).expect("the class is readable");

    let declared: Vec<&str> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("{ key = \""))
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert!(
        declared.len() > 10,
        "the OPTIONS table was not found — has the class changed shape? {declared:?}"
    );

    for (key, _) in class_options(&Settings::builtin()) {
        // `papersize` is the base class's, not ours.
        if key == "papersize" {
            continue;
        }
        assert!(
            declared.contains(&key.as_str()),
            "the application sends `-O {key}`, which sile/classes/biblecompose.lua \
             does not declare. SILE treats that as a hard error, so this would fail \
             the build at the typesetting step.\n  declared: {declared:?}"
        );
    }
}

/// And the other direction, which is a milder problem but still one: a class
/// option nothing sets is a setting a publisher cannot reach.
#[test]
fn every_option_the_class_declares_is_one_the_application_sends() {
    let class = repo_root().join("sile/classes/biblecompose.lua");
    let source = std::fs::read_to_string(class.as_std_path()).expect("the class is readable");
    let declared: Vec<&str> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("{ key = \""))
        .filter_map(|rest| rest.split('"').next())
        .collect();

    // Every option comes from the settings file: nothing describes *this run*
    // rather than the publication, since a build is always the whole
    // publication. The rule is that a knob with nothing attached to it is a
    // defect, and there are no exemptions to examine.
    let sent = class_options(&Settings::builtin());
    for key in declared {
        assert!(
            sent.iter().any(|(k, _)| k == key),
            "the class declares `{key}`, but no setting reaches it — it is a knob \
             with nothing attached to it"
        );
    }
}
