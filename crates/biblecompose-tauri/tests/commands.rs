//! The window's data path, without a window.
//!
//! Every command that does not need an `AppHandle` is an ordinary function, so
//! the interesting half of the shell — what the project pane shows, what the
//! settings form shows, what happens when a field is edited — is testable in
//! CI on a machine with no display. What is left needing a person is the
//! window itself.

use biblecompose_tauri_lib::{clear_setting, project_at, write_setting};
use camino::Utf8PathBuf;

/// A project folder with one real book in it.
fn project(settings: Option<&str>) -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");

    std::fs::write(
        root.join("JHN.usfm").as_std_path(),
        "\\id JHN\n\\h John\n\\c 1\n\\p\n\\v 1 In the beginning was the Word.\n\
         \\c 2\n\\p\n\\v 1 On the third day a wedding took place.\n",
    )
    .expect("write the book");

    if let Some(toml) = settings {
        std::fs::write(root.join("biblecompose.toml").as_std_path(), toml).expect("write settings");
    }
    (dir, root)
}

#[test]
fn a_folder_of_usfm_opens_with_its_books_and_its_defaults() {
    let (_dir, root) = project(None);
    let p = project_at(&root);

    assert_eq!(p.books.len(), 1);
    let john = &p.books[0];
    assert_eq!(john.code, "JHN");
    assert_eq!(john.name, "John", "the running head name, not the code");
    assert_eq!(john.chapters, 2);
    assert_eq!((john.errors, john.warnings), (0, 0));

    assert!(!p.blocked);
    assert!(p.output.ends_with("bible.pdf"), "{}", p.output);

    // CFG-001: a folder with no settings file still has every setting.
    let size = p.settings.iter().find(|s| s.key == "page.size").unwrap();
    assert_eq!(size.value, "6x9in");
    assert!(!size.overridden);
    assert!(
        size.location.is_none(),
        "a built-in value invents no location"
    );
}

#[test]
fn a_setting_the_project_wrote_says_where() {
    let (_dir, root) = project(Some("schema_version = 1\n\n[page]\nsize = \"a5\"\n"));
    let p = project_at(&root);

    let size = p.settings.iter().find(|s| s.key == "page.size").unwrap();
    assert_eq!(size.value, "148x210mm");
    assert!(size.overridden);
    assert_eq!(size.location.as_ref().unwrap().line, Some(4));
}

/// GUI-002 end to end: editing a field writes the file and the next read sees
/// it, with no TOML typed by anyone.
#[test]
fn editing_a_field_writes_the_file() {
    let (_dir, root) = project(None);

    let p = write_setting(&root, "page.columns", "1").expect("1 is a column count");
    let columns = p.settings.iter().find(|s| s.key == "page.columns").unwrap();
    assert_eq!(columns.value, "1");
    assert!(columns.overridden);

    // The file exists now, and says only what was asked for.
    let written = std::fs::read_to_string(root.join("biblecompose.toml").as_std_path())
        .expect("a settings file was created");
    assert!(written.contains("schema_version = 1"));
    assert!(written.contains("columns = 1"));
    assert!(!written.contains("page.size"), "nothing else was written");
}

/// A bad value changes nothing and comes back as the reason.
#[test]
fn a_refused_edit_leaves_the_project_as_it_was() {
    let (_dir, root) = project(Some("schema_version = 1\n[page]\nsize = \"6x9in\"\n"));

    let refused =
        write_setting(&root, "page.size", "quarto").expect_err("quarto is not a page size");
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].code, "CFG-003");
    assert!(
        refused[0].help.is_some(),
        "a refusal has to say what to write"
    );

    let after = project_at(&root);
    let size = after
        .settings
        .iter()
        .find(|s| s.key == "page.size")
        .unwrap();
    assert_eq!(size.value, "6x9in");
}

/// CFG-007 through the shell: reset removes the override and the built-in
/// value comes back.
#[test]
fn resetting_a_field_restores_the_default() {
    let (_dir, root) = project(Some("schema_version = 1\n[page]\ncolumns = 1\n"));

    let p = clear_setting(&root, "page.columns").expect("resets");
    let columns = p.settings.iter().find(|s| s.key == "page.columns").unwrap();
    assert_eq!(columns.value, "2");
    assert!(!columns.overridden);

    let written = std::fs::read_to_string(root.join("biblecompose.toml").as_std_path()).unwrap();
    assert!(
        !written.contains("columns"),
        "the key is gone, not defaulted"
    );
}

/// A setting that changes which books there are must change the pane too,
/// which is why the command returns the whole project and not just the field.
#[test]
fn excluding_a_book_removes_it_from_the_pane() {
    let (_dir, root) = project(None);
    assert_eq!(project_at(&root).books.len(), 1);

    let p = write_setting(&root, "books.exclude", "JHN").expect("JHN is a book code");
    assert!(p.books.is_empty(), "the excluded book left the pane");
}

/// A file that will not parse blocks the build, and says so on the first line
/// rather than falling back to defaults silently (CFG-003).
#[test]
fn a_broken_settings_file_blocks_and_explains() {
    let (_dir, root) = project(Some("schema_version = 1\n[page\nsize = \"a5\"\n"));
    let p = project_at(&root);

    assert!(p.blocked);
    let bad = p
        .diagnostics
        .iter()
        .find(|d| d.code == "CFG-001")
        .expect("the parse error is reported");
    assert_eq!(bad.severity, "error");
    assert_eq!(bad.location.as_ref().unwrap().line, Some(2));
    assert!(bad.detail.is_some(), "the parser's own rendering is kept");
}

/// A book with something wrong in it carries its own count, so the pane can
/// mark that row and only that row.
#[test]
fn a_book_carries_its_own_problem_count() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(
        root.join("JHN.usfm").as_std_path(),
        "\\id JHN\n\\h John\n\\c 1\n\\p\n\\v 1 Word.\n\\zz custom\n",
    )
    .expect("write");

    let p = project_at(&root);
    let john = &p.books[0];
    assert!(
        john.errors + john.warnings > 0,
        "an unknown marker should be reported against its own book"
    );
    // The pane's badge and the panel's list are the same diagnostics, filtered
    // the same way — but the badge counts problems, and an `info` is not one.
    // A book whose only remark is informational shows clean, which is the
    // point of having three severities.
    let problems = p
        .diagnostics
        .iter()
        .filter(|d| d.location.as_ref().is_some_and(|l| l.path == john.path))
        .filter(|d| d.severity != "info")
        .count();
    assert_eq!(problems, john.errors + john.warnings);
}
