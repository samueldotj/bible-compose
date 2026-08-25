//! P6.2 — three editions, each of which has to set a page (NFR: GUI-002).
//!
//! "Each produces an acceptable PDF from a bare USFM folder." *Acceptable* is
//! a word a test cannot check, so this checks the four things that make a page
//! unacceptable and can be measured:
//!
//! * nothing is set outside the measure, which is what a page that has run off
//!   the paper looks like from the inside (the assertion P5.3 built);
//! * no page carries nothing;
//! * the trim size is the one the preset asked for; and
//! * the thing each preset is *for* actually happened — one column, or no
//!   verse numbers, or type a reader with low vision can read.
//!
//! Started from a folder holding nothing but Scripture, because "from a bare
//! USFM folder" is half the claim.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, project, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_config::preset;
use biblecompose_config::{ConfigDocument, TomlFile, SCHEMA_VERSION};
use biblecompose_testkit::pdf::{Line, Pdf};
use camino::Utf8PathBuf;
use common::have_backend;

const GENESIS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.\n",
    "\\v 2 Now the earth was formless and void, and darkness was over the ",
    "surface of the deep, and the Spirit of God was hovering over the waters. ",
    "\\f + \\fr 1:2 \\ft Or a mighty wind from God.\\f*\n",
    "\\v 3 And God said, Let there be light, and there was light.\n",
    "\\v 4 God saw that the light was good, and He separated the light from ",
    "the darkness. \\x - \\xo 1:4 \\xt John 1:5\\x*\n",
    "\\q1 And the evening and the morning\n",
    "\\q2 were the first day.\n",
);

/// Everything a page's bounds depend on, in points.
struct Measure {
    page_width: f64,
    inner: f64,
    outer: f64,
}

impl Measure {
    /// The left and right edges of the text block on one page.
    ///
    /// **The two margins are not the same and swap on every page.** The inner
    /// one is the binding edge — on the left of a recto and on the right of a
    /// verso — so a bound derived by assuming symmetry is 13pt wrong on one
    /// side of every spread, which is enough to call a good page overfull and
    /// to let a bad one through.
    fn bounds(&self, page: usize) -> (f64, f64) {
        let recto = page % 2 == 1;
        let (left, right) = if recto {
            (self.inner, self.outer)
        } else {
            (self.outer, self.inner)
        };
        (left, self.page_width - right)
    }
}

/// A bare folder, then a preset written into it the way the window does.
fn build_with(preset_id: &str) -> (tempfile::TempDir, Utf8PathBuf, Vec<Line>, Pdf, Measure) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm").as_std_path(), GENESIS).expect("write the book");

    // Through `TomlFile`, which is the path the command takes, so this asserts
    // what a publisher would actually get rather than what the TOML says.
    let path = root.join("biblecompose.toml");
    let mut file = TomlFile::create(path.clone(), &TomlFile::settings_header(SCHEMA_VERSION));
    let chosen = preset::by_id(preset_id).expect("a preset this release ships");
    let complaints = preset::apply(&mut file, chosen);
    assert!(complaints.is_empty(), "{preset_id}: {complaints:?}");
    file.save().expect("write the settings");

    // And read back off disk, so a preset that wrote something unreadable
    // fails here rather than being resolved from memory.
    ConfigDocument::read(&path).unwrap_or_else(|d| panic!("{preset_id} wrote bad TOML: {d}"));

    let opened = project::open(&root);
    assert!(
        !opened.blocked(),
        "{preset_id}: {:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );

    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = opened.settings.clone();
    request.styles = opened.styles.clone();
    request.prior = opened.diagnostics.clone();

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(
        &opened.document,
        &request,
        &CancelToken::new(),
        &mut reporter,
    );
    assert_eq!(
        report.state,
        BuildState::Succeeded,
        "{preset_id}: {:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );

    let raw =
        std::fs::read(report.output.as_ref().expect("a PDF").as_std_path()).expect("read the PDF");
    let measure = Measure {
        page_width: opened.settings.page.size.width.points(),
        inner: opened.settings.page.margin_inner.points(),
        outer: opened.settings.page.margin_outer.points(),
    };
    (guard, root, Pdf::lines(&raw), Pdf::parse(&raw), measure)
}

/// Every preset builds, and every page it produces is sound.
#[test]
fn every_preset_sets_an_acceptable_page() {
    if !have_backend() {
        return;
    }
    for chosen in preset::ALL {
        let (_g, _root, lines, pdf, measure) = build_with(chosen.id);

        // One trim size throughout, and it is a real one.
        let (width, height) = pdf
            .uniform_page_size_inches()
            .unwrap_or_else(|| panic!("{}: pages of differing sizes", chosen.id));
        assert!(
            (4.0..=9.0).contains(&width) && (6.0..=12.0).contains(&height),
            "{}: {width}x{height}in is not a book",
            chosen.id
        );

        // No page carries nothing.
        for page in 1..=pdf.pages {
            assert!(
                lines.iter().any(|l| l.page == page),
                "{}: page {page} of {} is blank",
                chosen.id,
                pdf.pages
            );
        }

        // **And nothing is set outside the measure**, on either side, with
        // the bounds taken from the preset's own margins and swapped for the
        // verso pages.
        let mut worst: Option<(f64, usize, f64)> = None;
        for line in &lines {
            let (left, right) = measure.bounds(line.page);
            for mark in &line.marks {
                let over = (left - mark.x).max(mark.x - right);
                if over > 0.25 && worst.is_none_or(|(w, _, _)| over > w) {
                    worst = Some((over, line.page, mark.x));
                }
            }
        }
        assert!(
            worst.is_none(),
            "{}: something is set outside the measure — {worst:?} (over, page, x)",
            chosen.id
        );
    }
}

/// **A reader's edition takes the machinery off the page** — and keeps the
/// anchors, because hiding a number does not remove the place it marks
/// (SCR-001).
#[test]
fn the_readers_edition_has_no_apparatus() {
    if !have_backend() {
        return;
    }
    let (_g, root, lines, _pdf, _measure) = build_with("reader");

    // Verse numbers are the only thing set at 6.4pt, and there are none.
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&6.4)),
        "a reader's edition has no verse numbers"
    );
    // Nor a footnote or a reference, which are the only 7.4pt things.
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&7.4)),
        "a reader's edition has no apparatus"
    );
    // The note's own words are not on the page either — hidden rather than
    // merely unstyled.
    let text: String = lines.iter().map(Line::text).collect();
    assert!(!text.replace(' ', "").contains("amightywind"));
    assert!(!text.replace(' ', "").contains("John1:5"));

    // But the Scripture is, and so are the anchors.
    assert!(text.replace(' ', "").contains("Inthebeginning"));
    let raw = std::fs::read(root.join("out.pdf").as_std_path()).expect("the PDF");
    assert!(
        Pdf::destinations(&raw).iter().any(|d| d == "GEN.1"),
        "the chapter is still a place the file can name"
    );
}

/// **Large print is actually large**, in one column, and not justified.
#[test]
fn large_print_is_readable_at_arms_length() {
    if !have_backend() {
        return;
    }
    let (_g, _root, lines, pdf, _measure) = build_with("large-print");

    assert!(
        lines.iter().any(|l| l.sizes().contains(&14.0)),
        "large print should be set at 14pt: {:?}",
        lines.iter().map(|l| l.sizes()).collect::<Vec<_>>()
    );
    assert_eq!(pdf.uniform_page_size_inches(), Some((7.0, 10.0)));

    // One column: every line of Scripture begins at the same left edge, where
    // a second column would put half of them somewhere else entirely.
    let body: Vec<&Line> = lines.iter().filter(|l| l.sizes().contains(&14.0)).collect();
    let left = body.iter().map(|l| l.left()).fold(f64::INFINITY, f64::min);
    assert!(
        body.iter().all(|l| l.left() - left < 40.0),
        "something is set well right of the measure's left edge, which is what \
         a second column looks like"
    );

    // Ragged right. A justified page ends most of its lines at the same x; a
    // ragged one does not, and the rightmost mark of each line is where that
    // shows. Measured as: the lines do not all end within a point of each
    // other.
    let ends: Vec<f64> = body
        .iter()
        .filter_map(|l| l.marks.last().map(|m| m.x))
        .collect();
    let widest = ends.iter().copied().fold(0.0_f64, f64::max);
    let narrowest = ends.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        widest - narrowest > 5.0,
        "every line ends in the same place, which is what justified looks like"
    );
}

/// The two-column preset is the conventional page, and it is not the reader's.
#[test]
fn two_column_is_the_conventional_page() {
    if !have_backend() {
        return;
    }
    let (_g, _root, lines, pdf, _measure) = build_with("two-column");

    assert_eq!(pdf.uniform_page_size_inches(), Some((6.0, 9.0)));
    assert!(
        lines.iter().any(|l| l.sizes().contains(&6.4)),
        "the conventional page has verse numbers"
    );
    assert!(
        lines.iter().any(|l| l.sizes().contains(&7.4)),
        "and an apparatus at the foot"
    );

    // The first verse of the chapter goes unnumbered, because the chapter
    // figure beside it already says which verse it is. Verse `2` is on the
    // page and verse `1` is not — as a number of its own, at the verse size.
    let numbers: Vec<String> = lines
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| m.size == 6.4)
        .map(|m| m.text.clone())
        .collect();
    assert!(numbers.iter().any(|n| n == "2"), "{numbers:?}");
    assert!(
        !numbers.iter().any(|n| n == "1"),
        "verse 1 should be unnumbered under its chapter figure: {numbers:?}"
    );
}
