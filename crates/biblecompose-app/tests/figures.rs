//! P4.3 — what a build does about the artwork a project names (SCR-006).
//!
//! Every one of these was a silent success before. A project naming two
//! figures that did not exist reported `[completed]`, wrote a PDF, and left
//! two holes in it; an absolute path to artwork outside the project embedded
//! without a word (spike F-14).
//!
//! The pre-flight runs whether or not a typesetter is installed, so unlike the
//! apparatus tests these need no backend — they call it directly.

use biblecompose_app::asset;
use biblecompose_config::value::MissingAsset;
use biblecompose_diagnostics::{Diagnostics, Severity};
use biblecompose_scripture::{
    canon::BookCode, Attribute, Block, Book, BookNames, FigureRef, Inline, ScriptureDocument,
    VerseId,
};
use camino::{Utf8Path, Utf8PathBuf};

/// One book, one verse, and the figures given.
fn document(figures: Vec<FigureRef>) -> ScriptureDocument {
    let mut blocks = vec![Block::Paragraph {
        style: biblecompose_scripture::ParaStyle::P,
        content: vec![
            Inline::Chapter {
                number: 1,
                published: None,
                alternate: None,
            },
            Inline::Verse {
                id: VerseId::single(1),
                published: None,
                alternate: None,
            },
            Inline::Text("That which was from the beginning.".to_owned()),
        ],
    }];
    blocks.extend(figures.into_iter().map(Block::Figure));
    ScriptureDocument::new(vec![Book::new(
        BookCode::parse("1JN").expect("a real book code"),
        BookNames::named("1 John"),
        blocks,
    )])
}

fn figure(src: &str) -> FigureRef {
    FigureRef {
        src: Utf8PathBuf::from(src),
        alt: None,
        caption: Some("A plate.".to_owned()),
        size: Some("col".to_owned()),
        attributes: Vec::new(),
    }
}

fn project() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::create_dir_all(root.join("art").as_std_path()).expect("make art/");
    (dir, root)
}

fn place(root: &Utf8Path, at: &str, bytes: &[u8]) {
    let path = root.join(at);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent.as_std_path()).expect("make the folder");
    }
    std::fs::write(path.as_std_path(), bytes).expect("write the file");
}

/// One column, so `size="span"` never fires; the two-column case has its own
/// test.
fn check(doc: &ScriptureDocument, root: &Utf8Path, policy: MissingAsset) -> Diagnostics {
    let mut d = Diagnostics::new();
    asset::preflight(doc, root, policy, 1, &mut d);
    d
}

fn codes(d: &Diagnostics) -> Vec<String> {
    d.iter().map(|x| x.code.as_str().to_owned()).collect()
}

#[test]
fn artwork_that_is_there_says_nothing() {
    let (_g, root) = project();
    place(&root, "art/map.png", &biblecompose_testkit::PIXEL_PNG);
    let d = check(
        &document(vec![figure("art/map.png")]),
        &root,
        MissingAsset::Stop,
    );
    assert!(d.is_empty(), "{:?}", codes(&d));
}

/// The default: a book with a hole in it is worse than a build that says why.
#[test]
fn a_missing_figure_stops_the_build_by_default() {
    let (_g, root) = project();
    let d = check(
        &document(vec![figure("art/map.png")]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-001"]);
    assert!(d.has_blocking());
    assert!(d.iter().next().unwrap().message.contains("art/map.png"));
}

/// And the other policy: warn, leave it out, carry on — which is what a proof
/// wants while the artwork is still being drawn.
#[test]
fn omit_warns_and_names_the_figure_to_withhold() {
    let (_g, root) = project();
    let doc = document(vec![figure("art/map.png")]);
    let mut d = Diagnostics::new();
    let out = asset::preflight(&doc, &root, MissingAsset::Omit, 1, &mut d);

    assert_eq!(codes(&d), ["ASSET-001"]);
    assert!(!d.has_blocking(), "the build is not stopped by this");
    assert_eq!(d.iter().next().unwrap().severity, Severity::Warning);
    assert_eq!(out.omitted, [Utf8PathBuf::from("art/map.png")]);
}

/// **Spike F-14.** SILE validates a figure's format and never its provenance:
/// an absolute path to a valid image outside the project embedded silently.
/// SRS §15 says the check is ours, and this is it.
#[test]
fn a_figure_outside_the_project_is_refused() {
    let (_g, root) = project();
    let (_elsewhere, outside) = project();
    place(&outside, "map.png", &biblecompose_testkit::PIXEL_PNG);

    let d = check(
        &document(vec![figure(outside.join("map.png").as_str())]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-002"]);
    assert!(d.has_blocking());
}

/// The same by the other route, and the one that needs no filesystem at all to
/// be wrong: `..` out of the project.
#[test]
fn climbing_out_of_the_project_is_refused() {
    let (_g, root) = project();
    let d = check(
        &document(vec![figure("../elsewhere.png")]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-002"]);
}

/// Containment is not a policy. `omit` is about artwork that has not arrived
/// yet; a path outside the project is a rule about what a project *is*.
#[test]
fn omit_does_not_relax_containment() {
    let (_g, root) = project();
    let d = check(
        &document(vec![figure("../elsewhere.png")]),
        &root,
        MissingAsset::Omit,
    );
    assert_eq!(codes(&d), ["ASSET-002"]);
    assert!(d.has_blocking(), "still an error under omit");
}

/// Recognised by its bytes and not its name, so a file that is prose with a
/// `.png` on the end is caught.
#[test]
fn a_file_that_is_not_an_image_is_refused() {
    let (_g, root) = project();
    place(&root, "art/map.png", b"This is prose, not a picture.\n");
    let d = check(
        &document(vec![figure("art/map.png")]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-003"]);
    assert!(d.has_blocking());
}

/// And the converse: a real PNG under a name that says otherwise is fine,
/// because the extension was never the question.
#[test]
fn the_name_is_not_what_decides_the_format() {
    let (_g, root) = project();
    place(&root, "art/plate.dat", &biblecompose_testkit::PIXEL_PNG);
    let d = check(
        &document(vec![figure("art/plate.dat")]),
        &root,
        MissingAsset::Stop,
    );
    assert!(d.is_empty(), "{:?}", codes(&d));
}

/// A format the reader knows and this release has never placed: a warning
/// naming what does work, not a refusal of what might.
#[test]
fn an_untested_format_warns_rather_than_blocks() {
    let (_g, root) = project();
    place(&root, "art/map.gif", b"GIF89a\x01\x00\x01\x00\x00\x00\x00");
    let d = check(
        &document(vec![figure("art/map.gif")]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-003"]);
    assert!(!d.has_blocking());
}

/// A PDF placed as artwork brings its whole page box and its embedded fonts.
/// Measured, not refusable, and worth one line in the log.
#[test]
fn a_pdf_plate_is_noted_once_however_many_there_are() {
    let (_g, root) = project();
    place(&root, "art/one.pdf", b"%PDF-1.5\n% a plate\n");
    place(&root, "art/two.pdf", b"%PDF-1.5\n% another\n");
    let d = check(
        &document(vec![figure("art/one.pdf"), figure("art/two.pdf")]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-004"], "said once, not per plate");
    assert!(!d.has_blocking());
    assert_eq!(d.iter().next().unwrap().severity, Severity::Info);
}

/// `size="span"` needs the frame to be the measure, and in two columns it is
/// one column. The figure sets at column width, which is a reasonable answer
/// and a silent one — so it is said out loud.
#[test]
fn span_is_reported_where_it_cannot_be_honoured() {
    let (_g, root) = project();
    place(&root, "art/map.png", &biblecompose_testkit::PIXEL_PNG);
    let mut wide = figure("art/map.png");
    wide.size = Some("span".to_owned());

    let mut two = Diagnostics::new();
    asset::preflight(
        &document(vec![wide.clone()]),
        &root,
        MissingAsset::Stop,
        2,
        &mut two,
    );
    assert_eq!(codes(&two), ["ASSET-005"]);
    assert!(!two.has_blocking());

    let mut one = Diagnostics::new();
    asset::preflight(
        &document(vec![wide]),
        &root,
        MissingAsset::Stop,
        1,
        &mut one,
    );
    assert!(one.is_empty(), "one column is what span asks for");
}

/// USFM-003: `ref` is the author's own word for where the figure belongs, and
/// it beats anything this layer could work out.
///
/// It has to, because normalization hoists a figure out of the paragraph that
/// contained it (P1.5) — so the running verse when the figure is reached is
/// the paragraph's last, not the one it was written beside.
#[test]
fn a_figure_is_reported_where_the_author_said_it_belongs() {
    let (_g, root) = project();
    let mut with_ref = figure("art/map.png");
    with_ref.attributes = vec![Attribute {
        key: "ref".to_owned(),
        value: "1:1".to_owned(),
    }];

    let d = check(&document(vec![with_ref]), &root, MissingAsset::Stop);
    assert!(
        d.iter().next().unwrap().message.contains("at 1:1"),
        "{:?}",
        d.iter().next().unwrap().message
    );

    // And without one, the chapter — which is true whatever the hoisting did.
    let plain = check(
        &document(vec![figure("art/map.png")]),
        &root,
        MissingAsset::Stop,
    );
    assert!(
        plain.iter().next().unwrap().message.contains("in 1 John 1"),
        "{:?}",
        plain.iter().next().unwrap().message
    );
}

/// Every figure is reported, not just the first: DIA-002 wants a blocked build
/// to say everything that is wrong at once.
#[test]
fn every_figure_is_checked() {
    let (_g, root) = project();
    place(&root, "art/there.png", &biblecompose_testkit::PIXEL_PNG);
    let d = check(
        &document(vec![
            figure("art/there.png"),
            figure("art/absent.png"),
            figure("../outside.png"),
        ]),
        &root,
        MissingAsset::Stop,
    );
    assert_eq!(codes(&d), ["ASSET-001", "ASSET-002"]);
}
