//! Front matter: a `FRT.usfm` with a title page and a copyright page, set
//! before the first book of Scripture.
//!
//! Through the whole application from a folder, like the acceptance
//! scenarios, because every part of this crosses a seam — discovery has to
//! place the book, the parser's chapter rule has to be set aside for it, the
//! emitter writes a different element, and the class sets it with none of a
//! Scripture page's furniture and then puts the furniture back. A test of
//! any one layer would pass with the others wrong.

mod common;

use biblecompose_app::project;
use biblecompose_testkit::pdf::{Line, Pdf};
use camino::Utf8PathBuf;
use common::{have_backend, HEAD};

const FRONT: &str = concat!(
    "\\id FRT Front matter\n\\h Front matter\n",
    "\\periph Title Page|id=\"title\"\n",
    "\\mt1 The Holy Bible\n\\mt2 Berean Standard Bible\n",
    "\\periph Copyright|id=\"copyright\"\n",
    "\\pc This text has been dedicated to the public domain.\n",
    "\\pc Printed by nobody in particular.\n",
);

const GENESIS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.\n",
    "\\v 2 Now the earth was formless and void.\n",
);

/// A folder holding these files, opened and built the way the window does.
fn build(
    files: &[(&str, &str)],
    settings: &str,
) -> (tempfile::TempDir, biblecompose_app::BuildReport) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    for (name, body) in files {
        std::fs::write(root.join(name).as_std_path(), body).expect("write the file");
    }
    if !settings.is_empty() {
        std::fs::write(
            root.join("biblecompose.toml").as_std_path(),
            format!("schema_version = 1\n{settings}\n"),
        )
        .expect("write the settings");
    }

    let opened = project::open(&root);
    assert!(
        !opened.blocked(),
        "the project should open: {:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    let mut request = biblecompose_app::BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = opened.settings.clone();
    request.styles = opened.styles.clone();
    request.prior = opened.diagnostics.clone();
    let (mut reporter, _events) = biblecompose_app::BuildReporter::new();
    let report = biblecompose_app::build(
        &opened.document,
        &request,
        &biblecompose_app::CancelToken::new(),
        &mut reporter,
    );
    (guard, report)
}

fn lines(report: &biblecompose_app::BuildReport) -> Vec<Line> {
    let path = report.output.as_ref().unwrap_or_else(|| {
        panic!(
            "expected a PDF: {:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        )
    });
    Pdf::lines(&std::fs::read(path.as_std_path()).expect("read the PDF"))
}

/// Everything on one page, with the spaces a PDF does not record removed.
fn page_text(lines: &[Line], page: usize) -> String {
    lines
        .iter()
        .filter(|l| l.page == page)
        .map(|l| l.text().replace(' ', ""))
        .collect()
}

/// The lines of page furniture — head or folio — on one page.
fn furniture(lines: &[Line], page: usize) -> Vec<&Line> {
    lines
        .iter()
        .filter(|l| l.page == page && l.sizes() == vec![HEAD])
        .collect()
}

/// A folder with front matter opens with the front matter first, and the
/// parser's complaint that it has no chapter is not raised against it.
#[test]
fn front_matter_is_a_book_the_project_knows_and_lists_first() {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm").as_std_path(), GENESIS).expect("write");
    // Not named FRT.usfm: a book is its `\id`, not its filename (PRJ-003).
    std::fs::write(root.join("zz-front.usfm").as_std_path(), FRONT).expect("write");

    let opened = project::open(&root);
    assert!(
        !opened.blocked(),
        "{:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        !opened
            .diagnostics
            .iter()
            .any(|d| d.message.contains("chapter marker")),
        "a chapterless front matter is not a fault: {:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );

    let codes: Vec<&str> = opened
        .document
        .books
        .iter()
        .map(|b| b.code.as_str())
        .collect();
    assert_eq!(
        codes,
        ["FRT", "GEN"],
        "bound first, whatever the filename sorts as"
    );

    let front = &opened.document.books[0];
    let titles: Vec<&str> = front
        .blocks
        .iter()
        .filter_map(|b| match b {
            biblecompose_scripture::Block::Division { title, .. } => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["Title Page", "Copyright"]);
    assert_eq!(front.chapter_count(), 0);
}

/// The pages: title, copyright, then Genesis on a right-hand page numbered
/// 1 — in two columns, with the front matter in one.
#[test]
fn a_title_page_and_a_copyright_page_precede_the_scripture() {
    if !have_backend() {
        return;
    }
    let (_guard, report) = build(&[("FRT.usfm", FRONT), ("GEN.usfm", GENESIS)], "");
    assert_eq!(
        report.state,
        biblecompose_app::BuildState::Succeeded,
        "{:?}",
        report.diagnostics
    );
    let lines = lines(&report);
    let pages = common::pages(&lines);
    assert_eq!(pages.len(), 3, "title, copyright, Genesis: {pages:?}");

    // Page 1 is the title page, and nothing but. The title is set in the
    // `mt1` style — 24pt, the largest thing in the document — so it is
    // recognisable by size alone.
    let title = page_text(&lines, 1);
    assert!(title.contains("TheHolyBible"), "{title}");
    assert!(title.contains("BereanStandardBible"), "{title}");
    assert!(
        !title.contains("publicdomain"),
        "the copyright page is a page of its own: {title}"
    );
    assert!(
        lines.iter().any(|l| l.page == 1 && l.sizes() == vec![24.0]),
        "the title is set in the mt1 size"
    );

    // Page 2 is the copyright page.
    let copyright = page_text(&lines, 2);
    assert!(copyright.contains("publicdomain"), "{copyright}");
    assert!(!copyright.contains("TheHolyBible"), "{copyright}");

    // Neither carries a running head or a folio.
    assert!(
        furniture(&lines, 1).is_empty(),
        "a title page is not headed: {:?}",
        furniture(&lines, 1)
    );
    assert!(
        furniture(&lines, 2).is_empty(),
        "nor is a copyright page numbered"
    );

    // Page 3 is Genesis, headed, and numbered 1: the body starts at 1 and the
    // pages before it were counted but not printed.
    let body = page_text(&lines, 3);
    assert!(body.contains("Inthebeginning"), "{body}");
    assert_eq!(common::folio(&lines, 3), "1", "the body is numbered from 1");
    assert!(
        common::head(&lines, 3).contains("Genesis"),
        "{:?}",
        common::head(&lines, 3)
    );
}

/// The copyright page is one column wide whatever the body is set in: a
/// centred line on it is centred on the page, not within the left column.
#[test]
fn front_matter_is_set_in_one_column_even_when_the_body_has_two() {
    if !have_backend() {
        return;
    }
    let (_guard, report) = build(
        &[("FRT.usfm", FRONT), ("GEN.usfm", GENESIS)],
        "[page]\ncolumns = 2\n",
    );
    let lines = lines(&report);
    let pdf = {
        let path = report.output.as_ref().expect("a PDF");
        Pdf::parse(&std::fs::read(path.as_std_path()).expect("read the PDF"))
    };
    let (width_in, _) = pdf.uniform_page_size_inches().expect("one page size");
    let centre = width_in * 72.0 / 2.0;

    // A `\pc` line on the copyright page: its left edge is left of centre
    // and it reaches past it. In two columns a centred line of the left
    // column would end before the gutter, well short of the page's centre.
    let line = lines
        .iter()
        .find(|l| l.page == 2 && l.text().contains("Printed"))
        .expect("the copyright page's second line");
    let right = line.marks.iter().map(|m| m.x).fold(f64::MIN, f64::max);
    assert!(
        line.left() < centre && right > centre,
        "left {} right {right} centre {centre}",
        line.left()
    );
}

/// The table of contents: every book with the page it begins on, under the
/// heading the file gives it, after the copyright page — and the front matter
/// padded to an even number of pages so the first book still opens on a
/// right-hand page.
#[test]
fn the_contents_page_lists_every_book_with_its_page() {
    if !have_backend() {
        return;
    }
    let with_toc = format!("{FRONT}\\periph Table of Contents|id=\"toc\"\n\\imt1 Contents\n");
    let john = concat!(
        "\\id JHN\n\\h John\n\\toc1 The Gospel according to John\n\\c 1\n\\p\n",
        "\\v 1 In the beginning was the Word.\n",
    );
    let (_guard, report) = build(
        &[
            ("FRT.usfm", &with_toc),
            ("GEN.usfm", GENESIS),
            ("JHN.usfm", john),
        ],
        "",
    );
    assert_eq!(
        report.state,
        biblecompose_app::BuildState::Succeeded,
        "{:?}",
        report.diagnostics
    );
    let lines = lines(&report);
    // Title, copyright, contents, a blank to make the front matter even, then
    // Genesis and John.
    assert_eq!(
        common::pages(&lines),
        [1, 2, 3, 5, 6],
        "{:?}",
        common::pages(&lines)
    );

    let contents = page_text(&lines, 3);
    assert!(contents.starts_with("Contents"), "{contents}");
    // The long name from `\toc1` where the file gives one, the running
    // head's where it does not; the page is the one the body prints.
    assert!(contents.contains("Genesis"), "{contents}");
    assert!(contents.contains("TheGospelaccordingtoJohn"), "{contents}");
    let entry = |name: &str| -> String {
        lines
            .iter()
            .filter(|l| l.page == 3 && l.text().contains(name))
            .map(|l| l.text())
            .collect()
    };
    assert!(entry("Genesis").ends_with('1'), "{}", entry("Genesis"));
    assert!(entry("John").ends_with('2'), "{}", entry("John"));
    assert!(
        entry("Genesis").contains("...."),
        "dot leaders: {}",
        entry("Genesis")
    );
    assert!(
        furniture(&lines, 3).is_empty(),
        "a contents page is not headed"
    );

    // Page 4 is the blank, with nothing on it at all; Genesis is page 5 and
    // still numbered 1, on a right-hand page.
    assert!(lines.iter().all(|l| l.page != 4));
    assert_eq!(common::folio(&lines, 5), "1");
    assert_eq!(common::folio(&lines, 6), "2");
}

/// A project with no `FRT` of its own is given the built-in front matter:
/// the publication's name on a title page, a copyright page, a contents page
/// — and `books.include` leaves it out like any book.
#[test]
fn a_project_without_front_matter_is_given_the_built_in_one() {
    if !have_backend() {
        return;
    }
    let (_guard, report) = build(
        &[("GEN.usfm", GENESIS)],
        "[project]\nname = \"Vedapusthakam\"\nauthor = \"The Translators\"\n",
    );
    assert_eq!(
        report.state,
        biblecompose_app::BuildState::Succeeded,
        "{:?}",
        report.diagnostics
    );
    let lines = lines(&report);
    assert_eq!(
        common::pages(&lines),
        [1, 2, 3, 5],
        "{:?}",
        common::pages(&lines)
    );
    assert!(
        page_text(&lines, 1).contains("Vedapusthakam"),
        "{}",
        page_text(&lines, 1)
    );
    assert!(
        page_text(&lines, 2).contains("TheTranslators"),
        "{}",
        page_text(&lines, 2)
    );
    assert!(
        page_text(&lines, 3).contains("Genesis"),
        "{}",
        page_text(&lines, 3)
    );
    assert!(
        page_text(&lines, 5).contains("Inthebeginning"),
        "{}",
        page_text(&lines, 5)
    );
    assert_eq!(common::folio(&lines, 5), "1");

    let (_guard, report) = build(&[("GEN.usfm", GENESIS)], "[books]\ninclude = [\"GEN\"]\n");
    let path = report.output.as_ref().expect("a PDF");
    let without = Pdf::lines(&std::fs::read(path.as_std_path()).expect("read the PDF"));
    assert_eq!(common::pages(&without), [1], "turned off, it is gone");
}

/// Front matter that is switched off is left out like any other book, and
/// the body then starts on page 1 as it always did.
#[test]
fn front_matter_can_be_left_out_like_any_book() {
    if !have_backend() {
        return;
    }
    let (_guard, report) = build(
        &[("FRT.usfm", FRONT), ("GEN.usfm", GENESIS)],
        "[books]\ninclude = [\"GEN\"]\n",
    );
    let lines = lines(&report);
    assert_eq!(common::pages(&lines), [1]);
    assert!(!page_text(&lines, 1).contains("TheHolyBible"));
    assert_eq!(common::folio(&lines, 1), "1");
}

/// The starter the window writes: only into a folder with no front matter,
/// and what it writes is a project the application then reads back with a
/// title page and a copyright page in it.
#[test]
fn the_starter_front_matter_is_written_once_and_reads_back() {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm").as_std_path(), GENESIS).expect("write");
    std::fs::write(
        root.join("biblecompose.toml").as_std_path(),
        "schema_version = 1\n[project]\nname = \"Vedapusthakam\"\nauthor = \"The Translators\"\n",
    )
    .expect("write");
    let before = std::fs::read(root.join("GEN.usfm").as_std_path()).expect("read");

    let path = project::add_front_matter(&root).expect("a starter is written");
    assert_eq!(path, root.join("FRT.usfm"));
    let written = std::fs::read_to_string(path.as_std_path()).expect("read it back");
    assert!(written.starts_with("\\id FRT"), "{written}");
    assert!(
        written.contains("\\periph Title Page|id=\"title\""),
        "{written}"
    );
    assert!(
        written.contains("\\periph Copyright|id=\"copyright\""),
        "{written}"
    );
    assert!(written.contains("\\mt1 Vedapusthakam"), "{written}");
    assert!(written.contains("The Translators"), "{written}");

    // Nothing of theirs was touched (BLD-004).
    assert_eq!(
        std::fs::read(root.join("GEN.usfm").as_std_path()).expect("read"),
        before
    );

    // And it is a book the project reads.
    let opened = project::open(&root);
    assert!(
        !opened.blocked(),
        "{:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(opened.document.books[0].code.as_str(), "FRT");
    assert_eq!(
        opened.document.books[0]
            .blocks
            .iter()
            .filter(|b| matches!(b, biblecompose_scripture::Block::Division { .. }))
            .count(),
        3,
        "a title page, a copyright page and a table of contents"
    );

    // A second time refuses, naming the file that is in the way.
    let refused = project::add_front_matter(&root).expect_err("there is front matter already");
    assert!(refused.message.contains("FRT.usfm"), "{refused}");

    // And so does a project whose front matter is under another name.
    std::fs::rename(
        root.join("FRT.usfm").as_std_path(),
        root.join("front.usfm").as_std_path(),
    )
    .expect("rename");
    let refused = project::add_front_matter(&root).expect_err("front.usfm holds it");
    assert!(refused.message.contains("front.usfm"), "{refused}");
    assert!(!root.join("FRT.usfm").exists(), "and nothing was written");
}
