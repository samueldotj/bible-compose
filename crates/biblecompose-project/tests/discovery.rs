//! P1.3 — one test per requirement, against real directories.
//!
//! An integration test rather than a unit one, because discovery's contract is
//! a folder in and a set of books out, and a temporary directory is a cheaper
//! fake than an abstraction over the filesystem would be.

use biblecompose_project::{discover, identify, Discovery};
use camino::Utf8PathBuf;

struct Project {
    _dir: tempfile::TempDir,
    root: Utf8PathBuf,
}

fn project(files: &[(&str, &str)]) -> Project {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");
    for (rel, contents) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent").as_std_path()).expect("mkdir");
        std::fs::write(path.as_std_path(), contents).expect("write");
    }
    Project { _dir: dir, root }
}

fn codes(d: &Discovery) -> Vec<&'static str> {
    d.diagnostics.iter().map(|x| x.code.as_str()).collect()
}

/// PRJ-003, and the reason this module reads files rather than filenames.
#[test]
fn a_renamed_file_still_loads_as_the_book_it_declares() {
    let p = project(&[("wildly-misnamed.usfm", "\\id MAT Matthew\n\\c 1\n")]);
    let d = discover(&p.root);

    assert!(!d.blocked(), "{:?}", codes(&d));
    assert_eq!(d.books.len(), 1);
    assert_eq!(d.books[0].book.as_str(), "MAT");
}

/// PRJ-002.
#[test]
fn nested_directories_are_discovered_without_registration() {
    let p = project(&[
        ("books/nt/41-matthew.usfm", "\\id MAT\n"),
        ("books/ot/genesis.SFM", "\\id GEN\n"),
    ]);
    let d = discover(&p.root);

    assert!(!d.blocked(), "{:?}", codes(&d));
    let found: Vec<&str> = d.books.iter().map(|b| b.book.as_str()).collect();
    // Canonical order, not directory order — GEN was found second.
    assert_eq!(found, ["GEN", "MAT"]);
}

/// PRJ-004. Blocking rather than choosing is the whole point: either file
/// could be the current draft.
#[test]
fn two_files_claiming_one_book_block_the_build() {
    let p = project(&[
        ("draft/mat.usfm", "\\id MAT\n"),
        ("final/mat.usfm", "\\id MAT\n"),
    ]);
    let d = discover(&p.root);

    assert!(d.blocked());
    assert_eq!(codes(&d), ["PRJ-001"]);
    assert!(d.books.is_empty(), "an ambiguous book must not be built");

    // Naming both files is the difference between a diagnostic a publisher
    // can act on and one they cannot.
    let detail = d
        .diagnostics
        .iter()
        .next()
        .expect("one diagnostic")
        .detail
        .as_ref()
        .expect("the paths");
    assert!(detail.contains("draft"), "{detail}");
    assert!(detail.contains("final"), "{detail}");
}

/// PRJ-005.
#[test]
fn a_project_of_one_book_is_a_project() {
    let p = project(&[("jhn.usfm", "\\id JHN\n\\c 1\n")]);
    let d = discover(&p.root);

    assert!(!d.blocked(), "{:?}", codes(&d));
    assert_eq!(d.books.len(), 1);
}

/// PRJ-006. The failure this prevents is a build eating its own output — and
/// note that without the exclusion every one of these would also trip
/// PRJ-004, so the symptom would have been a duplicate-book error nobody
/// could explain.
#[test]
fn generated_directories_never_become_inputs() {
    let p = project(&[
        ("mat.usfm", "\\id MAT\n"),
        ("output/mat.usfm", "\\id MAT\n"),
        (".biblecompose/build/current/mat.usfm", "\\id MAT\n"),
        ("target/mat.usfm", "\\id MAT\n"),
    ]);
    let d = discover(&p.root);

    assert!(
        !d.blocked(),
        "a generated copy was treated as an input: {:?}",
        codes(&d)
    );
    assert_eq!(d.books.len(), 1);
    assert_eq!(d.books[0].path, p.root.join("mat.usfm"));
}

#[test]
fn an_empty_folder_is_reported_rather_than_built() {
    let p = project(&[("README.md", "not Scripture")]);
    let d = discover(&p.root);

    assert!(d.blocked());
    assert_eq!(codes(&d), ["PRJ-003"]);
}

#[test]
fn a_file_with_no_id_marker_is_named_rather_than_ignored() {
    let p = project(&[("mystery.usfm", "\\c 1\n\\p\n\\v 1 Text.\n")]);
    let d = discover(&p.root);

    assert!(d.blocked());
    assert_eq!(codes(&d), ["PRJ-004"]);
    let at = d
        .diagnostics
        .iter()
        .next()
        .expect("one diagnostic")
        .location
        .as_ref()
        .expect("a location");
    assert!(at.path.as_str().ends_with("mystery.usfm"), "{at}");
}

/// The shapes real files actually arrive in.
#[test]
fn identification_survives_what_real_files_look_like() {
    // A byte-order mark, which distributors add and translators never see.
    assert_eq!(identify("\u{FEFF}\\id MAT\n").expect("MAT").as_str(), "MAT");

    // A description after the code, which the specification allows.
    assert_eq!(
        identify("\\id 1CO First Corinthians, draft 3\n")
            .expect("1CO")
            .as_str(),
        "1CO"
    );

    // Lowercase, which Paratext tolerates.
    assert_eq!(identify("\\id mrk\n").expect("MRK").as_str(), "MRK");

    // Blank lines first.
    assert_eq!(identify("\n\n\\id LUK\n").expect("LUK").as_str(), "LUK");

    // `\ide` merely starts with `id`, and declares an encoding rather than a
    // book. Treating it as `\id e` would identify every file as nothing.
    assert_eq!(
        identify("\\ide UTF-8\n\\id JHN\n").expect("JHN").as_str(),
        "JHN"
    );

    // Content before the marker is a diagnostic upstream reports, not a
    // reason to give up on placing the file.
    assert_eq!(
        identify("Stray text\n\\id ROM\n").expect("ROM").as_str(),
        "ROM"
    );

    assert!(identify("\\c 1\n").is_none());
    assert!(identify("\\id ZZZ\n").is_none(), "ZZZ is not in the canon");
}

/// BLD-004 says a build must not overwrite its source. That is a property of
/// the architecture here rather than a rule anyone has to remember, so it is
/// asserted against the architecture: no non-test code in this crate opens
/// anything for writing.
#[test]
fn no_code_path_here_writes_to_a_scripture_file() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;

    for entry in std::fs::read_dir(&src).expect("src/") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        // Only the part before any test module: a test may write fixtures.
        let code = text.split("#[cfg(test)]").next().unwrap_or_default();
        for forbidden in [
            "fs::write",
            "File::create",
            "OpenOptions",
            "create_dir",
            "remove_file",
        ] {
            assert!(
                !code.contains(forbidden),
                "{} uses {forbidden}; discovery must never write (BLD-004)",
                path.display()
            );
        }
        checked += 1;
    }

    assert!(checked > 0, "the scan found no source files to check");
}
