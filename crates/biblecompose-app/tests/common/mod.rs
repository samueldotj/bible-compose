//! Building a fixture into a real PDF and reading the page back.
//!
//! **Through the whole application and not through the emitter and class
//! alone**, which is why the tests that use this live in this crate. A page is
//! styled by `styles.toml` and configured by `defaults.toml`; a test that ran
//! only the two layers below would be reading a page set entirely in the body
//! size, with every assertion about which part is which vacuously true.
//!
//! Shared because there are now three suites asking the same two questions —
//! what is on the page, and where — and three copies of the answer is where a
//! harness starts drifting from itself.
//!
//! `dead_code` is allowed because each integration test is its own binary
//! and compiles this module separately: anything only one suite needs is
//! unused in the other two, and the warning would be about nothing.
#![allow(dead_code)]

use biblecompose_app::{build, BuildReporter, BuildRequest, CancelToken};
use biblecompose_config::{cascade, settings, ConfigDocument, Settings};
use biblecompose_scripture::fixtures;
use biblecompose_testkit::pdf::{Line, Pdf};
use camino::Utf8PathBuf;

/// The sizes the built-in style sheet gives the body, the notes and the page
/// furniture. How one part of a page is told from another without knowing
/// where the frames ended up.
pub const BODY: f64 = 9.2;
pub const NOTE: f64 = 7.4;
pub const HEAD: f64 = 8.2;

/// Whether a typesetter is installed. Printed once when it is not, so a
/// machine without one still runs the rest of the suite and says why these did
/// not run.
pub fn have_backend() -> bool {
    match biblecompose_app::backend_version() {
        Ok(_) => true,
        Err(d) => {
            eprintln!("skipping: {d}");
            false
        }
    }
}

/// The built-in settings with some keys overridden, resolved the way a project
/// file is — so a spelling a test uses and the resolver rejects fails here
/// rather than being quietly ignored by the class.
pub fn settings(body: &str) -> Settings {
    let toml = format!("schema_version = 1\n{body}\n");
    let doc = ConfigDocument::parse("test.toml", toml).expect("valid TOML");
    let (resolved, diagnostics) = settings::resolve(Some(&doc));
    assert!(
        diagnostics.is_empty(),
        "the test's own settings are not valid: {:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    resolved
}

/// Build one fixture and read the page back.
///
/// The guard comes back with the lines because dropping it deletes the PDF
/// they were read from — and, more to the point, the folder the build wrote
/// into.
pub fn typeset(fixture: &str, overrides: &str) -> (tempfile::TempDir, Vec<Line>) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    let doc = fixtures::by_name(fixture).expect("a known fixture");
    // Every fixture that names artwork needs it to be there: since P4.3 a
    // figure whose file is absent stops the build.
    biblecompose_testkit::place_fixture_assets(&root);

    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = settings(overrides);
    request.styles = cascade::resolve(None, false).0;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&doc, &request, &CancelToken::new(), &mut reporter);
    let pdf = report.output.unwrap_or_else(|| {
        panic!(
            "{fixture} failed to build: {:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        )
    });
    let raw = std::fs::read(pdf.as_std_path()).expect("read the PDF");
    (guard, Pdf::lines(&raw))
}

/// The same build, as bytes rather than as lines.
///
/// For the assertions that are about the file rather than the page — its
/// properties, its destinations, its outline — none of which is reachable from
/// a list of glyph positions.
pub fn raw_pdf(fixture: &str, overrides: &str) -> (tempfile::TempDir, Vec<u8>) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    let doc = fixtures::by_name(fixture).expect("a known fixture");
    biblecompose_testkit::place_fixture_assets(&root);

    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = settings(overrides);
    request.styles = cascade::resolve(None, false).0;

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(&doc, &request, &CancelToken::new(), &mut reporter);
    let pdf = report.output.unwrap_or_else(|| {
        panic!(
            "{fixture} failed to build: {:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        )
    });
    let raw = std::fs::read(pdf.as_std_path()).expect("read the PDF");
    (guard, raw)
}

/// A single column, so "the note area" and "the measure" each mean one thing.
pub const ONE_COLUMN: &str = "[page]\ncolumns = 1\n";

/// The pages the document has, in order.
pub fn pages(lines: &[Line]) -> Vec<usize> {
    let mut seen: Vec<usize> = lines.iter().map(|l| l.page).collect();
    seen.dedup();
    seen
}

/// The running head of one page, as one string.
///
/// The head is the only 8.2pt line at the top of a page — the folio is the
/// same size and at the bottom, so position is what separates them. SILE
/// writes top-down `y`, so "nearer the top" is "less negative".
pub fn head(lines: &[Line], page: usize) -> String {
    lines
        .iter()
        .filter(|l| l.page == page && l.sizes() == vec![HEAD] && l.y > -60.0)
        .map(Line::text)
        .collect()
}

/// And the folio, by the same rule from the other end.
pub fn folio(lines: &[Line], page: usize) -> String {
    lines
        .iter()
        .filter(|l| l.page == page && l.sizes() == vec![HEAD] && l.y < -400.0)
        .map(Line::text)
        .collect()
}

/// Every line of Scripture, which is the only thing set at the body size.
pub fn body_lines(lines: &[Line]) -> Vec<&Line> {
    lines.iter().filter(|l| l.sizes().contains(&BODY)).collect()
}

/// Lines that are wholly note-sized and long enough to be a line of a note
/// rather than a caller.
///
/// The callers in the body are note-sized too — they are set from the note's
/// own style — so length is what separates them.
pub fn note_lines(lines: &[Line]) -> Vec<&Line> {
    lines
        .iter()
        .filter(|l| l.sizes() == vec![NOTE] && l.text().chars().count() > 3)
        .collect()
}

/// The whole document's text at one size, joined — for asking whether
/// something appears at all.
pub fn text_at(lines: &[Line], size: f64) -> String {
    lines
        .iter()
        .filter(|l| l.sizes().contains(&size))
        .map(Line::text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The same text with the Latin f-ligatures written out.
///
/// A PDF search for "first" does not find it, because the font shaped `fi`
/// into one glyph and its `/ToUnicode` map honestly reports that glyph as
/// U+FB01. The page is right and the assertion would be wrong, so the
/// assertion is the thing that gives.
pub fn unligature(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{fb00}' => out.push_str("ff"),
            '\u{fb01}' => out.push_str("fi"),
            '\u{fb02}' => out.push_str("fl"),
            '\u{fb03}' => out.push_str("ffi"),
            '\u{fb04}' => out.push_str("ffl"),
            _ => out.push(c),
        }
    }
    out
}

/// Every line whose size and face are exactly these — how one style is picked
/// out of a page when several share a size.
pub fn lines_set_in<'a>(lines: &'a [Line], size: f64, face: &str) -> Vec<&'a Line> {
    lines
        .iter()
        .filter(|l| l.sizes() == vec![size] && l.faces() == vec![face.to_owned()])
        .collect()
}
