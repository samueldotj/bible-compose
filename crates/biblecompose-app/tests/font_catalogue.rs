//! FONT-006 — a font is suggested for the language, fetched, and packaged.
//!
//! The suggesting half is unit-tested beside the table it reads, because it is
//! a question about data. This is the other half: what reaches the disk, and
//! more to the point what does not.
//!
//! Everything here runs offline. The module takes fetched bytes rather than a
//! URL for exactly this reason — the failures worth testing are a truncated
//! download, a file that is not the one the catalogue describes, a file that is
//! not a font at all, and a font that cannot draw the script it was offered
//! for, and not one of them is reproducible against a working server.
//!
//! Two vendored fonts stand in for a download. `spike/assets/fonts` has a
//! Tamil face and `tests/fonts` a Latin one, which is the same pair
//! `font_preflight.rs` uses and for the same reason: one covers the script
//! under test and one does not.

use biblecompose_app::font::PROJECT_FONTS;
use biblecompose_app::typeface::{self, Face, Fetched, Licence, SourceFile, Typeface};
use biblecompose_testkit::repo_root;
use camino::{Utf8Path, Utf8PathBuf};

/// A licence that reads like one, and its digest.
const LICENCE: &str = "Copyright 2024 the test. SIL OPEN FONT LICENSE Version 1.1\n";
const LICENCE_SHA: &str = "d59391fdb1aedf5827aac21091fdd6fb7dac70ade5cfe4bc34639b862571e7fe";

/// A catalogue entry for the vendored Tamil face.
///
/// The byte count and digest are the real ones, so this exercises the same
/// comparisons a real download goes through rather than a relaxed version of
/// them.
static TAMIL: Typeface = Typeface {
    family: "Noto Serif Tamil",
    directory: "notoseriftamil",
    scripts: &["taml"],
    note: "the vendored face",
    licence: Licence {
        bytes: 59,
        sha256: LICENCE_SHA,
    },
    files: &[SourceFile {
        name: "NotoSerifTamil-Regular.ttf",
        face: Face::Regular,
        bytes: 51508,
        sha256: "3b166155947b5f620bf0aa5ad102a398408a9a31cbe2e8b58ad0f947276adf26",
        variable: false,
    }],
};

/// The same entry, pointed at a Latin font: a catalogue row that is simply
/// wrong about what the file can draw.
static WRONG_SCRIPT: Typeface = Typeface {
    family: "DejaVu Serif",
    directory: "dejavuserif",
    scripts: &["taml"],
    note: "a Latin face offered for Tamil",
    licence: Licence {
        bytes: 59,
        sha256: LICENCE_SHA,
    },
    files: &[SourceFile {
        name: "DejaVuSerif.ttf",
        face: Face::Regular,
        bytes: 380132,
        sha256: "3731a19f2b50d86d81dd1a1effcd81492ed1fa44f6ea904792dd98fdae9b3dfe",
        variable: false,
    }],
};

fn vendored(relative: &str) -> Vec<u8> {
    std::fs::read(repo_root().join(relative).as_std_path()).expect("the vendored font is there")
}

fn tamil_bytes() -> Vec<u8> {
    vendored("spike/assets/fonts/NotoSerifTamil-Regular.ttf")
}

fn licence() -> Fetched {
    Fetched {
        name: "OFL.txt".to_owned(),
        bytes: LICENCE.as_bytes().to_vec(),
    }
}

fn font(name: &str, bytes: Vec<u8>) -> Fetched {
    Fetched {
        name: name.to_owned(),
        bytes,
    }
}

fn project() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
    (dir, root)
}

fn fonts_dir(root: &Utf8Path) -> Utf8PathBuf {
    root.join(PROJECT_FONTS)
}

/// Whatever is in `assets/fonts/`, by name.
fn listing(root: &Utf8Path) -> Vec<String> {
    let dir = fonts_dir(root);
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(dir.as_std_path())
        .expect("read the font directory")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// --------------------------------------------------------- the packaging works

/// The acceptance criterion: a font fetched for the project's script lands in
/// `assets/fonts/` with its licence, where FONT-003 makes it travel with the
/// book — and the build then resolves it from the project rather than the
/// system.
#[test]
fn a_packaged_font_is_resolved_from_the_project() {
    let (_dir, root) = project();

    let packaged = typeface::package(
        &TAMIL,
        &[licence(), font("NotoSerifTamil-Regular.ttf", tamil_bytes())],
        &root,
    )
    .expect("the vendored font passes every check");

    assert_eq!(packaged.family, "Noto Serif Tamil");
    assert_eq!(packaged.files, vec!["NotoSerifTamil-Regular.ttf"]);
    assert_eq!(packaged.licence, "OFL-NotoSerifTamil.txt");
    assert_eq!(packaged.faces, vec![Face::Regular]);

    assert_eq!(
        listing(&root),
        vec!["NotoSerifTamil-Regular.ttf", "OFL-NotoSerifTamil.txt"],
    );

    // The point of the whole exercise: resolution now finds it, and finds it
    // in the project, which is what decides that the backend is handed a path
    // and the book renders the same way on somebody else's machine.
    let resolved = biblecompose_app::font::resolve("Noto Serif Tamil", &root, &[])
        .expect("a packaged font resolves");
    assert!(resolved.from_project, "{resolved:?}");
    assert_eq!(
        resolved.path,
        fonts_dir(&root).join("NotoSerifTamil-Regular.ttf")
    );
}

/// Packaging the same family twice must not fail and must not leave a
/// half-written file: a publisher who clicks twice, or who repackages after
/// moving a project, gets the same two files.
#[test]
fn packaging_twice_leaves_the_same_two_files() {
    let (_dir, root) = project();
    let fetched = [licence(), font("NotoSerifTamil-Regular.ttf", tamil_bytes())];

    typeface::package(&TAMIL, &fetched, &root).expect("first");
    typeface::package(&TAMIL, &fetched, &root).expect("second");

    assert_eq!(
        listing(&root),
        vec!["NotoSerifTamil-Regular.ttf", "OFL-NotoSerifTamil.txt"],
    );
}

/// `suggest` reports a family the project already holds, so the window can say
/// "in this project" rather than offering the same download again.
#[test]
fn a_packaged_family_is_reported_as_packaged() {
    let (_dir, root) = project();
    let characters = std::collections::BTreeSet::from(['க']);

    let before = typeface::suggest("ta", &characters, Some(&root));
    assert!(
        before.iter().all(|s| !s.packaged),
        "nothing is packaged in an empty project",
    );

    let real = typeface::typeface("Noto Serif Tamil").expect("in the catalogue");
    std::fs::create_dir_all(fonts_dir(&root).as_std_path()).expect("make the directory");
    for file in real.files {
        std::fs::write(
            fonts_dir(&root).join(file.name).as_std_path(),
            b"not checked here",
        )
        .expect("write");
    }

    let after = typeface::suggest("ta", &characters, Some(&root));
    let tamil = after
        .iter()
        .find(|s| s.family == "Noto Serif Tamil")
        .expect("offered for Tamil");
    assert!(tamil.packaged);
    // And the others still are not, so one family's files cannot vouch for
    // another's.
    assert!(after
        .iter()
        .filter(|s| s.family != "Noto Serif Tamil")
        .all(|s| !s.packaged));
}

// ------------------------------------------------- nothing is written on doubt

/// Every refusal asserts the same thing: the project is untouched. A font
/// half-written is a project that resolves a family whose bold is missing,
/// which is worse than one that resolved nothing.
fn refused(fetched: &[Fetched], entry: &'static Typeface) -> String {
    let (_dir, root) = project();
    let failure = typeface::package(entry, fetched, &root).expect_err("refused");
    assert!(
        listing(&root).is_empty(),
        "the project was written to anyway: {:?}",
        listing(&root),
    );
    format!("{} — {}", failure.message, failure.help.unwrap_or_default())
}

#[test]
fn a_truncated_download_is_refused_and_says_so() {
    let mut short = tamil_bytes();
    short.truncate(40_000);

    let message = refused(
        &[licence(), font("NotoSerifTamil-Regular.ttf", short)],
        &TAMIL,
    );
    assert!(message.contains("40000"), "{message}");
    assert!(message.contains("51508"), "{message}");
    assert!(message.contains("interrupted"), "{message}");
}

/// The length matches and the content does not, which is the case a byte count
/// alone would wave through.
#[test]
fn a_file_of_the_right_length_and_the_wrong_content_is_refused() {
    let swapped = vec![b'x'; 51508];

    let message = refused(
        &[licence(), font("NotoSerifTamil-Regular.ttf", swapped)],
        &TAMIL,
    );
    assert!(message.contains("not the file"), "{message}");
    assert!(message.contains("checksum"), "{message}");
}

#[test]
fn a_file_missing_from_the_download_is_refused() {
    let message = refused(&[licence()], &TAMIL);
    assert!(message.contains("NotoSerifTamil-Regular.ttf"), "{message}");
    assert!(message.contains("not downloaded"), "{message}");

    let no_licence = refused(&[font("NotoSerifTamil-Regular.ttf", tamil_bytes())], &TAMIL);
    assert!(no_licence.contains("OFL.txt"), "{no_licence}");
}

/// A licence that is not a licence is the one check that is about redistributing
/// somebody else's work rather than about bytes arriving intact.
#[test]
fn a_licence_that_is_not_the_open_font_license_is_refused() {
    static NOT_A_LICENCE: Typeface = Typeface {
        licence: Licence {
            bytes: 64,
            sha256: "83f9ad2dd18dd675f8182de7efd05d89c19cf27ca4f3bf0e87825f69056a24af",
        },
        ..TAMIL
    };

    let readme = Fetched {
        name: "OFL.txt".to_owned(),
        bytes: b"Copyright 2024 the test. No licence here at all, just a readme.\n".to_vec(),
    };
    let message = refused(
        &[readme, font("NotoSerifTamil-Regular.ttf", tamil_bytes())],
        &NOT_A_LICENCE,
    );
    assert!(message.contains("Open Font License"), "{message}");
}

/// What a mirror serving an error page looks like from here. It would pass a
/// length check if the page happened to be the right size, which is why the
/// parse is a separate step with its own message.
#[test]
fn something_that_is_not_a_font_is_refused() {
    let mut page = b"<!doctype html><title>404</title>".to_vec();
    page.resize(51508, b' ');

    let message = refused(
        &[licence(), font("NotoSerifTamil-Regular.ttf", page)],
        &TAMIL,
    );
    // The digest catches it first, which is the right order: a file that is not
    // the expected file is a clearer thing to say than a file that will not
    // parse.
    assert!(message.contains("not the file"), "{message}");
}

/// The check the digests cannot make: whether the catalogue was right about
/// what the font covers. A Latin face offered for Tamil is a suggestion that
/// would have produced a page of empty boxes, and FONT-002 would only have
/// caught it at the next build.
#[test]
fn a_font_that_cannot_draw_the_script_it_was_offered_for_is_refused() {
    let message = refused(
        &[
            licence(),
            font("DejaVuSerif.ttf", vendored("tests/fonts/DejaVuSerif.ttf")),
        ],
        &WRONG_SCRIPT,
    );
    assert!(message.contains("Tamil"), "{message}");
    assert!(message.contains("no glyph"), "{message}");
}

/// The family name is what a build resolves against, so a file that calls
/// itself something else would be written and then never found.
#[test]
fn a_font_that_names_a_different_family_is_refused() {
    static MISNAMED: Typeface = Typeface {
        family: "Charis SIL",
        files: &[SourceFile {
            name: "NotoSerifTamil-Regular.ttf",
            face: Face::Regular,
            bytes: 51508,
            sha256: "3b166155947b5f620bf0aa5ad102a398408a9a31cbe2e8b58ad0f947276adf26",
            variable: false,
        }],
        scripts: &["taml"],
        ..TAMIL
    };

    let message = refused(
        &[licence(), font("NotoSerifTamil-Regular.ttf", tamil_bytes())],
        &MISNAMED,
    );
    assert!(
        message.contains("does not call itself Charis SIL"),
        "{message}"
    );
}
