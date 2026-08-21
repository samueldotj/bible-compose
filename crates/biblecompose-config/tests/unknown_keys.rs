//! P2.5 — CFG-004: a key this release does not recognise, reported where it
//! was written, and promoted to an error in strict mode.

use biblecompose_config::settings::{self, Settings};
use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::{Diagnostic, Diagnostics, Severity};

fn resolve(body: &str) -> (Settings, Diagnostics) {
    let doc = ConfigDocument::parse("biblecompose.toml", body.to_owned()).expect("valid fixture");
    settings::resolve(Some(&doc))
}

fn unknown(d: &Diagnostics) -> Vec<&Diagnostic> {
    d.iter().filter(|d| d.code.as_str() == "CFG-002").collect()
}

/// The acceptance criterion, verbatim: `page.wdith` is reported at its own
/// line.
#[test]
fn a_misspelled_key_is_reported_at_its_own_line() {
    let (_, d) = resolve(
        "schema_version = 1\n\
         \n\
         [page]\n\
         columns = 1\n\
         wdith = \"6in\"\n",
    );

    let stray = unknown(&d);
    assert_eq!(stray.len(), 1, "{:?}", messages(&d));

    let only = stray[0];
    assert_eq!(only.severity, Severity::Warning);
    assert_eq!(
        only.message,
        "`page.wdith` is not a setting this release recognises"
    );
    let loc = only.location.as_ref().expect("CFG-004 needs a position");
    assert_eq!((loc.line, loc.column), (Some(5), Some(1)));
}

/// A near miss gets the whole dotted path suggested, not a bare key — the
/// answer to "what should I have written" is a line, not a word.
///
/// Note that SRS CFG-004's own example, `page.wdith`, gets no suggestion:
/// there is no `page.width` to suggest, because the schema spells the trim
/// size `page.size`. The example predates the schema, and inventing a key to
/// match it would be worse than not suggesting one.
#[test]
fn a_near_miss_is_suggested() {
    let (_, d) = resolve("schema_version = 1\n[page]\ncolums = 2\n");
    assert_eq!(
        unknown(&d)[0].help.as_deref(),
        Some("did you mean `page.columns`?")
    );
}

#[test]
fn a_key_nothing_resembles_gets_advice_rather_than_a_guess() {
    let (_, d) = resolve("schema_version = 1\n[page]\nbleed_marks = true\n");
    let help = unknown(&d)[0].help.as_deref().unwrap();
    assert!(help.contains("check the spelling"), "{help}");
}

/// A table nothing is known about is one complaint, at its header. Eight
/// warnings about the inside of `[gribble]` say less than one about
/// `[gribble]`.
#[test]
fn an_unknown_table_is_reported_once_rather_than_key_by_key() {
    let (_, d) = resolve(
        "schema_version = 1\n\
         [gribble]\n\
         one = 1\n\
         two = 2\n\
         three = 3\n",
    );

    let stray = unknown(&d);
    assert_eq!(stray.len(), 1, "{:?}", messages(&d));
    assert!(stray[0].message.contains("`gribble`"));
    assert_eq!(stray[0].location.as_ref().unwrap().line, Some(2));
}

/// But a table we know *part* of is descended into, so the complaint lands on
/// the stray key rather than on the whole of `[page]`.
#[test]
fn a_known_table_is_descended_into() {
    let (_, d) = resolve("schema_version = 1\n[page]\ncolumns = 2\nsplines = \"reticulated\"\n");
    assert_eq!(
        unknown(&d)[0].message,
        "`page.splines` is not a setting this release recognises"
    );
}

/// Everything the resolver reads is known — including the two keys that have
/// no built-in value and so do not appear in `defaults.toml`.
#[test]
fn no_supported_key_is_reported_as_unknown() {
    let (_, d) = resolve(
        "schema_version = 1\n\
         strict = false\n\
         \n\
         [project]\n\
         name = \"My Bible\"\n\
         language = \"ta\"\n\
         \n\
         [books]\n\
         order = [\"MAT\"]\n\
         include = [\"MAT\"]\n\
         \n\
         [page]\n\
         size = \"a5\"\n\
         columns = 1\n\
         margin_top = \"1in\"\n\
         margin_bottom = \"1in\"\n\
         margin_inner = \"1in\"\n\
         margin_outer = \"1in\"\n\
         column_gap = \"0.2in\"\n\
         header_gap = \"0.3in\"\n\
         footer_gap = \"0.3in\"\n\
         \n\
         [typography]\n\
         font_family = \"Gentium Plus\"\n\
         font_size = \"11pt\"\n\
         leading = \"13pt\"\n\
         hyphenation = false\n\
         \n\
         [numbering]\n\
         show_chapter_numbers = true\n\
         show_verse_numbers = true\n\
         \n\
         [notes]\n\
         show_footnotes = true\n\
         show_cross_references = false\n\
         \n\
         [headers]\n\
         enabled = true\n\
         show_book_name = true\n\
         show_reference_range = true\n\
         show_page_number = true\n\
         \n\
         [output]\n\
         file = \"out/bible.pdf\"\n\
         keep_intermediates = true\n",
    );

    assert!(
        d.is_empty(),
        "a file that sets every supported key must be clean: {:?}",
        messages(&d)
    );
}

/// A dotted key must be matched at its leaf, not reported because its head is
/// not a setting on its own.
#[test]
fn a_dotted_spelling_of_a_known_key_is_known() {
    let (s, d) = resolve("schema_version = 1\npage.columns = 1\n");
    assert!(d.is_empty(), "{:?}", messages(&d));
    assert_eq!(*s.page.columns, 1);
}

// --------------------------------------------------------------- strict

#[test]
fn strict_mode_promotes_it_to_an_error() {
    let (_, lenient) = resolve("schema_version = 1\n[page]\nwdith = \"6in\"\n");
    assert!(!lenient.has_blocking(), "the default is a warning");

    let (_, strict) = resolve("schema_version = 1\nstrict = true\n[page]\nwdith = \"6in\"\n");
    let stray = unknown(&strict);
    assert_eq!(stray.len(), 1);
    assert_eq!(stray[0].severity, Severity::Error);
    assert!(strict.has_blocking(), "strict mode stops the build");
}

/// Strict is off by default: a settings file written for a later release
/// should degrade rather than fail.
#[test]
fn strict_is_off_unless_asked_for() {
    assert!(!*Settings::builtin().strict);
}

/// `strict` itself is a setting, so it carries an origin like any other and a
/// GUI can show that the project turned it on.
#[test]
fn strict_records_where_it_was_turned_on() {
    let (s, _) = resolve("schema_version = 1\nstrict = true\n");
    assert!(*s.strict);
    assert!(s.strict.is_overridden());
    assert_eq!(s.strict.origin().location().unwrap().line, Some(2));
}

/// An unknown version already closed the file, so nothing inside it is read —
/// including for this check. One diagnostic, not one plus a list of keys that
/// are perfectly valid in the version the file was written for.
#[test]
fn an_unknown_schema_version_suppresses_the_key_check() {
    let (_, d) = resolve("schema_version = 99\n[gribble]\none = 1\n");
    assert_eq!(d.len(), 1, "{:?}", messages(&d));
    assert_eq!(d.iter().next().unwrap().code.as_str(), "CFG-004");
}

fn messages(d: &Diagnostics) -> Vec<String> {
    d.iter().map(|d| d.to_string()).collect()
}

/// A setting that used to exist is not a misspelling, and the help should not
/// pretend it is.
///
/// `books.exclude` was removed because two lists that can contradict each
/// other about the same book need a precedence rule, which is a rule a
/// publisher has to learn for no benefit. It is still reported — a setting
/// silently ignored is a publication quietly losing a book — but the message
/// says what happened and what to write instead.
#[test]
fn a_removed_setting_says_what_replaced_it() {
    let (_, d) = resolve("schema_version = 1\n[books]\nexclude = [\"JHN\"]\n");

    let stray = unknown(&d);
    assert_eq!(stray.len(), 1, "{:?}", messages(&d));
    assert!(stray[0].message.contains("books.exclude"), "{:?}", stray[0]);

    let help = stray[0].help.as_deref().expect("a removed key needs help");
    assert!(
        help.contains("books.include"),
        "the help must name the replacement rather than guess at a typo: {help}"
    );
    assert!(
        !help.contains("did you mean"),
        "they wrote exactly what the last release documented: {help}"
    );
}
