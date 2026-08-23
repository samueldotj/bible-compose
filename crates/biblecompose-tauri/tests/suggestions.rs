//! What the window offers must be what the schema accepts.
//!
//! The page tab suggests a dozen Bible trim sizes and the new-project dialog
//! suggests four dozen languages. Both are lists of *strings the application
//! will write into a settings file*, and a suggestion the schema then refuses
//! is worse than no suggestion at all — it is the application disagreeing with
//! itself in front of somebody who did what it said.
//!
//! Read out of the TypeScript rather than mirrored here, because a copy is a
//! thing to keep in step and this is the test that exists to stop that.

use biblecompose_config::ConfigDocument;
use biblecompose_testkit::repo_root;

/// Every `value: "…"` in a suggestion list.
fn values(file: &str, field: &str) -> Vec<String> {
    let text = std::fs::read_to_string(repo_root().join(file).as_std_path())
        .unwrap_or_else(|e| panic!("{file} is where the window keeps its suggestions: {e}"));

    let needle = format!("{field}: \"");
    let found: Vec<String> = text
        .match_indices(&needle)
        .filter_map(|(at, _)| {
            let rest = &text[at + needle.len()..];
            rest.find('"').map(|end| rest[..end].to_owned())
        })
        .collect();

    assert!(!found.is_empty(), "no {field} found in {file}");
    found
}

fn diagnostics(body: &str) -> Vec<String> {
    let doc = ConfigDocument::parse("biblecompose.toml", body.to_owned()).expect("valid fixture");
    let (_, d) = biblecompose_config::resolve(Some(&doc));
    d.iter().map(|x| x.to_string()).collect()
}

#[test]
fn every_suggested_trim_is_a_page_size() {
    for trim in values("src/lib/trims.ts", "value") {
        let complaints = diagnostics(&format!("schema_version = 1\n[page]\nsize = \"{trim}\"\n"));
        assert!(complaints.is_empty(), "{trim}: {complaints:?}");
    }
}

/// Languages are free text, so this checks the weaker thing that can still go
/// wrong: a tag with a stray space or a smart quote in it, which would be
/// written to the file exactly as listed.
#[test]
fn every_suggested_language_is_a_plain_tag() {
    for tag in values("src/lib/languages.ts", "tag") {
        assert!(
            tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !tag.is_empty(),
            "{tag:?} is not a BCP-47 tag"
        );
        assert!(diagnostics(&format!(
            "schema_version = 1\n[project]\nlanguage = \"{tag}\"\n"
        ))
        .is_empty());
    }
}

/// The window groups the book list by testament, and the names it groups into
/// are the canon's.
///
/// Two lists that have to agree: the `Testament` union in `backend.ts` is what
/// the wire is typed as, and `TESTAMENTS` in the pane is what actually gets a
/// column. A testament added to the table and to neither would be a book with
/// nowhere to go — silently absent from a list whose whole job is to be
/// complete.
#[test]
fn every_testament_the_canon_has_gets_a_column() {
    use biblecompose_scripture::BookCode;

    let spellings: std::collections::BTreeSet<&str> =
        BookCode::all().map(|c| c.testament().as_str()).collect();
    assert_eq!(spellings.len(), 3, "the canon's testaments: {spellings:?}");

    let typed = std::fs::read_to_string(
        repo_root()
            .join("src/lib/services/backend.ts")
            .as_std_path(),
    )
    .expect("the wire types");
    let pane = std::fs::read_to_string(
        repo_root()
            .join("src/components/ProjectPane.svelte")
            .as_std_path(),
    )
    .expect("the book list");

    for spelling in spellings {
        assert!(
            typed.contains(&format!("\"{spelling}\"")),
            "`Testament` in backend.ts does not include {spelling:?}"
        );
        assert!(
            pane.contains(&format!("id: \"{spelling}\"")),
            "the book list has no column for {spelling:?}"
        );
    }
}
