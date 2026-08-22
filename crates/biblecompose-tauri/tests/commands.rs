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
///
/// The book left out stays *listed* and stops being *included*. It is still on
/// disk and it still has a place in the order; a pane that hid it would be a
/// pane you could not tick it back on from, which is where BOOK-004's
/// checkboxes live.
#[test]
fn a_book_left_out_stays_listed_and_is_marked() {
    let (_dir, root) = project(None);
    let before = project_at(&root);
    assert_eq!(before.books.len(), 1);
    assert!(before.books[0].included);

    let p = write_setting(&root, "books.include", "MAT").expect("MAT is a book code");
    assert_eq!(p.books.len(), 1, "still on disk, still listed");
    assert!(!p.books[0].included, "and no longer in the publication");
    assert_eq!(p.books[0].chapters, 0, "a book left out is never parsed");
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

// ------------------------------------------------------- P3.5: styles

use biblecompose_tauri_lib::{clear_style, write_style};

/// GUI-004 end to end: a style edit writes `styles.toml` and the next read
/// sees it, with no TOML typed by anyone.
#[test]
fn editing_a_style_writes_the_sheet() {
    let (_dir, root) = project(None);

    let p = write_style(&root, "chapter", "font_size", "30pt").expect("30pt is a length");
    let chapter = p
        .styles
        .iter()
        .find(|s| s.selector == "chapter")
        .expect("chapter is styled");
    let size = chapter
        .properties
        .iter()
        .find(|p| p.name == "font_size")
        .unwrap();

    assert_eq!(size.value, "30pt");
    assert_eq!(size.origin, "file");

    // The header explains the file, so it is at the top of it.
    assert_eq!(size.location.as_ref().unwrap().line, Some(5));

    let written = std::fs::read_to_string(root.join("styles.toml").as_std_path())
        .expect("a style sheet was created");
    assert!(written.starts_with("# BibleCompose styles."), "{written}");
    assert!(written.contains("[chapter]"), "{written}");
    assert!(written.contains("font_size = \"30pt\""));
}

/// STY-008 through the shell: a value the project did not set says whether it
/// is a built-in or came from another style.
#[test]
fn a_style_property_says_where_it_came_from() {
    let (_dir, root) = project(None);
    let p = project_at(&root);

    let of = |selector: &str, property: &str| {
        p.styles
            .iter()
            .find(|s| s.selector == selector)
            .and_then(|s| s.properties.iter().find(|p| p.name == property))
            .unwrap_or_else(|| panic!("no {selector}.{property}"))
            .clone()
    };

    assert_eq!(of("chapter", "font_size").origin, "builtin");
    assert!(of("chapter", "font_size").location.is_none());

    // `qr2` has no entry of its own; its alignment comes from `qr1`.
    let inherited = of("poetry.qr2", "align");
    assert_eq!(inherited.origin, "inherited");
    assert_eq!(inherited.from.as_deref(), Some("poetry.qr1"));
}

/// A style that inherits follows an edit to what it inherits from — the whole
/// point of the cascade, seen through the window.
#[test]
fn editing_a_parent_moves_what_inherits_from_it() {
    let (_dir, root) = project(None);
    let p = write_style(&root, "poetry.q1", "space_above", "5pt").expect("a length");

    let q2 = p
        .styles
        .iter()
        .find(|s| s.selector == "poetry.q2")
        .unwrap()
        .properties
        .iter()
        .find(|p| p.name == "space_above")
        .expect("q2 now has one");

    assert_eq!(q2.value, "5pt");
    assert_eq!(q2.origin, "inherited");
    assert_eq!(q2.from.as_deref(), Some("poetry.q1"));
}

#[test]
fn a_refused_style_edit_writes_nothing() {
    let (_dir, root) = project(None);
    let refused = write_style(&root, "chapter", "font_size", "enormous").expect_err("not a length");

    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].code, "CFG-003");
    assert!(
        !root.join("styles.toml").exists(),
        "a refused first edit must not leave a file behind"
    );
}

#[test]
fn a_property_the_schema_does_not_have_is_refused_before_anything_is_written() {
    let (_dir, root) = project(None);
    let refused = write_style(&root, "chapter", "colour", "red").expect_err("not a property");
    assert_eq!(refused[0].code, "STY-002");
    assert!(!root.join("styles.toml").exists());
}

/// STY-005's other half: removing an override puts the cascade back in charge.
#[test]
fn resetting_a_style_restores_the_cascade() {
    let (_dir, root) = project(None);
    write_style(&root, "chapter", "font_size", "30pt").expect("writes");

    let p = clear_style(&root, "chapter", "font_size").expect("resets");
    let size = p
        .styles
        .iter()
        .find(|s| s.selector == "chapter")
        .unwrap()
        .properties
        .iter()
        .find(|p| p.name == "font_size")
        .unwrap();

    assert_eq!(size.value, "21pt", "the built-in size is back");
    assert_eq!(size.origin, "builtin");
}

/// A style edit must not disturb the rest of the sheet (CFG-006 applies to
/// both files).
#[test]
fn a_style_edit_leaves_the_rest_of_the_sheet_alone() {
    let (_dir, root) = project(None);
    let sheet = "# My design.\n\n[chapter]\nweight   = 700   # heavy\nfont_size = \"20pt\"\n";
    std::fs::write(root.join("styles.toml").as_std_path(), sheet).expect("write");

    write_style(&root, "chapter", "font_size", "22pt").expect("writes");

    let after = std::fs::read_to_string(root.join("styles.toml").as_std_path()).unwrap();
    assert_eq!(after, sheet.replace("\"20pt\"", "\"22pt\""));
    assert!(after.contains("# My design."));
    assert!(after.contains("weight   = 700   # heavy"));
}

/// The window shows something before a project is open. CFG-001 and STY-001
/// both say there is always an answer, so two empty panes were showing less
/// than the truth.
#[test]
fn the_defaults_are_available_without_a_project() {
    let d = biblecompose_tauri_lib::builtin_config();

    let size = d
        .settings
        .iter()
        .find(|s| s.key == "page.size")
        .expect("page.size is a setting");
    assert_eq!(size.value, "6x9in");
    assert!(!size.overridden, "nothing has overridden anything yet");

    let chapter = d
        .styles
        .iter()
        .find(|s| s.selector == "chapter")
        .expect("chapter is styled");
    assert!(chapter
        .properties
        .iter()
        .all(|p| p.origin == "builtin" || p.origin == "inherited"));

    // Every selector the editor offers must be in here, or a row would render
    // blank the moment the window opens.
    for selector in [
        "chapter",
        "verse",
        "note.f",
        "poetry.q1",
        "character.bd",
        "head",
    ] {
        assert!(
            d.styles.iter().any(|s| s.selector == selector),
            "no `{selector}` in the defaults"
        );
    }
}

/// STY-008: the inspector answers for *any* element, so the window is sent
/// every selector — including the ones nothing has set.
#[test]
fn every_selector_reaches_the_window() {
    let (_dir, root) = project(None);
    let p = project_at(&root);

    // `\p` renders as body text: the built-in sheet sets nothing for it, and
    // "nothing decides this" is the answer to why it looks the way it does.
    let plain = p
        .styles
        .iter()
        .find(|s| s.selector == "paragraph.p")
        .expect("an unstyled paragraph is still an element");
    assert!(plain.properties.is_empty());

    // And the whole schema is there, not a curated few.
    assert!(
        p.styles.len() > 100,
        "only {} selectors reached the window",
        p.styles.len()
    );
}

/// The chain the inspector walks: an inherited property names the selector it
/// came from, and that selector is one the window also has.
#[test]
fn an_inherited_property_names_a_selector_the_window_has() {
    let (_dir, root) = project(None);
    let p = project_at(&root);

    let inherited: Vec<(&str, &str)> = p
        .styles
        .iter()
        .flat_map(|s| {
            s.properties
                .iter()
                .filter(|prop| prop.origin == "inherited")
                .map(move |prop| (s.selector.as_str(), prop.from.as_deref().unwrap_or("")))
        })
        .collect();

    assert!(!inherited.is_empty(), "nothing inherits anything");
    for (selector, from) in inherited {
        assert!(!from.is_empty(), "`{selector}` inherits from nowhere");
        assert!(
            p.styles.iter().any(|s| s.selector == from),
            "`{selector}` inherits from `{from}`, which the window does not have"
        );
    }
}

// -------------------------------------------------- P3.7: external edits

/// STY-006: a style sheet edited outside the window is in force after a
/// reload, with no restart.
#[test]
fn an_external_style_edit_is_reflected_after_reload() {
    let (_dir, root) = project(None);

    let before = project_at(&root);
    let size_of = |p: &biblecompose_tauri_lib::WireProject| {
        p.styles
            .iter()
            .find(|s| s.selector == "chapter")
            .unwrap()
            .properties
            .iter()
            .find(|p| p.name == "font_size")
            .unwrap()
            .clone()
    };
    assert_eq!(size_of(&before).value, "21pt");
    assert_eq!(size_of(&before).origin, "builtin");

    // Somebody edits the file in a text editor while the window is open.
    std::fs::write(
        root.join("styles.toml").as_std_path(),
        "[chapter]\nfont_size = \"27pt\"\n",
    )
    .expect("write");

    let after = size_of(&project_at(&root));
    assert_eq!(after.value, "27pt");
    assert_eq!(after.origin, "file");
    assert_eq!(after.location.as_ref().unwrap().line, Some(2));
}

/// FUN-006: the same for settings, and for the books the settings select.
#[test]
fn an_external_settings_edit_is_reflected_after_reload() {
    let (_dir, root) = project(None);
    assert_eq!(project_at(&root).books.len(), 1);

    std::fs::write(
        root.join("biblecompose.toml").as_std_path(),
        "schema_version = 1\n[books]\ninclude = [\"MAT\"]\n",
    )
    .expect("write");

    let after = project_at(&root);
    assert_eq!(after.books.len(), 1, "the book is still in the folder");
    assert!(
        !after.books[0].included,
        "the reloaded project must honour the file as written"
    );
}

/// FUN-007: an externally edited book is noticed.
#[test]
fn an_external_book_edit_is_noticed() {
    use biblecompose_app::Fingerprint;

    let (_dir, root) = project(None);
    let fingerprint = Fingerprint::take(&root);
    assert!(fingerprint.changes(&root).is_empty());

    std::fs::write(
        root.join("JHN.usfm").as_std_path(),
        "\\id JHN\n\\h John\n\\c 1\n\\p\n\\v 1 Edited elsewhere.\n",
    )
    .expect("write");

    let changes = fingerprint.changes(&root);
    assert_eq!(changes.modified.len(), 1, "{changes:?}");
    assert!(changes.modified[0].ends_with("JHN.usfm"));
}

/// And the window's own writes are not external edits — otherwise every
/// settings change would put a "reload?" notice on screen.
#[test]
fn the_windows_own_write_is_not_a_change() {
    use biblecompose_app::Fingerprint;

    let (_dir, root) = project(None);
    write_setting(&root, "page.columns", "1").expect("writes");

    // What the shell does after a write: take the fingerprint again.
    let fingerprint = Fingerprint::take(&root);
    assert!(
        fingerprint.changes(&root).is_empty(),
        "a file we just wrote is not somebody else's edit"
    );
}

/// The PDF goes in the project folder, and nothing in the settings moves it.
#[test]
fn the_output_path_is_the_project_folder() {
    let (_dir, root) = project(None);
    let expected = root.join(biblecompose_app::project::OUTPUT_FILE);
    assert_eq!(project_at(&root).output, expected.to_string());

    // Even for a project that tries. `output.file` was a setting once, and a
    // file still carrying it is told so rather than quietly obeyed.
    let (_dir2, opinionated) = project(Some(
        "schema_version = 1\n[output]\nfile = \"somewhere/else.pdf\"\n",
    ));
    let p = project_at(&opinionated);
    assert_eq!(
        p.output,
        opinionated
            .join(biblecompose_app::project::OUTPUT_FILE)
            .to_string()
    );
    assert!(
        p.diagnostics.iter().any(|d| d.code == "CFG-002"),
        "the key that no longer works says so"
    );
}

// ------------------------------------------------- PRJ-001: a new project

use biblecompose_app::project as app_project;

/// A new project is a folder and one settings file, and it opens.
#[test]
fn creating_a_project_writes_a_folder_and_its_settings() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parent = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");

    let root = app_project::create(&parent, "Tamil Bible", "ta").expect("a new project");
    assert_eq!(
        root,
        parent.join("Tamil Bible"),
        "named after the publication"
    );

    let written = std::fs::read_to_string(root.join("biblecompose.toml").as_std_path())
        .expect("the settings file was written");
    assert!(written.contains("schema_version = 1"), "{written}");
    assert!(written.contains("\"Tamil Bible\""), "{written}");
    assert!(written.contains("\"ta\""), "{written}");
    // Two keys and no more: everything else has a built-in answer, and a file
    // full of defaults is a file somebody has to maintain to change nothing.
    assert!(!written.contains("page"), "{written}");

    // And it is a project: it opens, with no books yet and nothing blocking.
    let p = project_at(&root);
    assert!(p.books.is_empty());
    let name = p.settings.iter().find(|s| s.key == "project.name").unwrap();
    assert_eq!(name.value, "Tamil Bible");
    assert!(name.overridden);
}

/// The name is a folder name, so it is checked rather than quietly mangled.
#[test]
fn a_name_that_cannot_be_a_folder_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parent = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");

    let refused =
        app_project::create(&parent, "Acts 1/2", "en").expect_err("a slash is not a name");
    assert_eq!(refused.code.as_str(), "PRJ-005");
    assert!(refused.help.is_some());

    let empty = app_project::create(&parent, "   ", "en").expect_err("nor is nothing");
    assert_eq!(empty.code.as_str(), "PRJ-005");
}

/// Over an existing folder it refuses rather than merges: "new project" onto
/// somebody's work is either a mistake or a different verb.
#[test]
fn an_existing_folder_is_not_taken_over() {
    let dir = tempfile::tempdir().expect("temp dir");
    let parent = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");
    let taken = parent.join("Already Here");
    std::fs::create_dir(taken.as_std_path()).expect("mkdir");
    std::fs::write(taken.join("JHN.usfm").as_std_path(), "\\id JHN\n").expect("write");

    let refused =
        app_project::create(&parent, "Already Here", "en").expect_err("something is there");
    assert_eq!(refused.code.as_str(), "PRJ-005");
    assert!(
        taken.join("JHN.usfm").exists(),
        "and nothing of theirs was touched"
    );
}
