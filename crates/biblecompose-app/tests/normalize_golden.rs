//! P1.7 — golden XML for the M1 construct set, from USFM rather than structs.
//!
//! The M0 goldens are emitted from hand-built `ScriptureDocument`s, which
//! proves the emitter but says nothing about normalization. This one starts
//! from USFM text, so it pins the whole of `parse → normalize → emit` and will
//! notice if any stage changes what it produces.
//!
//! Update with `UPDATE_GOLDEN=1 cargo test -p biblecompose-testkit`.

use biblecompose_app::project;
use biblecompose_scripture::plan::BookPlan;
use biblecompose_scripture::ScriptureDocument;
use biblecompose_testkit::{corpus, golden};
use camino::Utf8PathBuf;

/// Every construct M1 claims to handle, in one book.
///
/// Hand-written rather than taken from the corpus because a golden has to be
/// readable: when this diff turns red, the reviewer needs to see which
/// construct moved, and 16 chapters of Mark would bury that.
const M1_CONSTRUCTS: &str = concat!(
    "\\id MRK The Gospel according to Mark\n",
    "\\h Mark\n\\toc1 The Gospel according to Mark\n\\toc2 Mark\n\\toc3 Mk\n",
    "\\mt1 The Gospel according to\n\\mt2 Mark\n",
    "\\is1 Introduction\n",
    "\\ip An introduction paragraph with \\iqt quoted words\\iqt* in it.\n",
    "\\iot Outline\n\\io1 The beginning \\ior (1:1-8)\\ior*\n",
    "\\c 1\n\\cl Chapter One\n",
    "\\s1 The Beginning of the Gospel\n",
    "\\r (Matthew 3:1-12; Luke 3:1-18)\n",
    "\\p\n",
    "\\v 1 The beginning of the gospel of \\nd Jesus Christ\\nd*, \\add the\\add* Son of God.\n",
    "\\v 2-3 As it is written: \\wj A voice of one calling in the wilderness\\wj*",
    "\\f + \\fr 1:2 \\fq wilderness \\ft Or \\fqa desert\\f*",
    "\\x - \\xo 1:2 \\xt Isaiah 40:3\\x*.\n",
    "\\q1 Prepare the way for the Lord,\n",
    "\\q2 make straight paths for Him.\n",
    "\\b\n",
    "\\m A continuation paragraph with no indent.\n",
    "\\pi1 An indented paragraph.\n",
    "\\li1 A list item\n\\li2 A nested list item\n",
    "\\tr \\th1 Name \\th2 Count\n",
    "\\tr \\tc1 Levi \\tc2 3\n",
    "\\p \\v 4 A word with attributes: \\w baptism|strong=\"G0908\"\\w*,",
    " and a nested marker: \\add supplied \\+bd bold\\+bd*\\add*.\n",
    "\\v 5 \\va 5b\\va* Alternate numbering, and a figure:",
    " \\fig The Jordan|src=\"jordan.png\" size=\"span\" ref=\"1:5\"\\fig*\n",
    "\\p \\v 6 \\qt-s |who=\"John\"\\*A quotation milestone\\qt-e\\* and a \\zcustom custom marker\\zcustom*.\n",
);

/// Write one book to a scratch folder and load it the way the CLI does.
fn load(name: &str, usfm: &str) -> (tempfile::TempDir, ScriptureDocument) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");
    std::fs::write(root.join(name).as_std_path(), usfm).expect("write");

    let loaded = project::load(&root, &BookPlan::canonical());
    assert!(
        !loaded.blocked(),
        "{name} did not load: {:?}",
        loaded.diagnostics.iter().collect::<Vec<_>>()
    );
    (dir, loaded.document)
}

fn golden_path(name: &str) -> Utf8PathBuf {
    corpus::root()
        .parent()
        .expect("workspace root")
        .join("tests/golden")
        .join(format!("{name}.xml"))
}

#[test]
fn the_m1_construct_set_emits_a_stable_document() {
    let (_dir, document) = load("MRK.usfm", M1_CONSTRUCTS);
    let emitted = biblecompose_app::emit(&document, &styles());
    golden::assert_matches(&golden_path("m1_constructs"), &emitted.xml);
}

/// Emission is deterministic (DET-001) all the way from USFM, not only from a
/// hand-built model — normalization uses ordered collections throughout, and
/// this is what says so.
#[test]
fn the_same_usfm_emits_identically_twice() {
    let once = emit_once();
    let twice = emit_once();
    assert_eq!(once, twice);
}

fn emit_once() -> String {
    let (_dir, document) = load("MRK.usfm", M1_CONSTRUCTS);
    biblecompose_app::emit(&document, &styles()).xml
}

/// A whole real book emits without the emitter dropping a construct it does
/// not recognise. The golden above is readable; this one is exhaustive.
#[test]
fn every_corpus_book_emits_with_nothing_unsupported_by_the_emitter() {
    for entry in corpus::books() {
        let source = corpus::read(&entry);
        let name = format!("{}.usfm", entry.book);
        let (_dir, document) = load(&name, &source);

        let emitted = biblecompose_app::emit(&document, &styles());
        assert!(
            emitted.unsupported.is_empty(),
            "{} — the emitter dropped: {:?}",
            entry.path,
            emitted.unsupported
        );
        assert!(
            emitted.xml.contains("<book"),
            "{} produced no book element",
            entry.path
        );
    }
}

/// The emitted XML is well formed.
///
/// Added because the construct golden shipped a `<figure>` with `size` and
/// `file` written twice — duplicate attribute names, which is a
/// well-formedness error rather than a cosmetic one. Reading goldens catches
/// that once; a parser catches it every time.
#[test]
fn every_corpus_book_emits_well_formed_xml() {
    for entry in corpus::books() {
        let source = corpus::read(&entry);
        let name = format!("{}.usfm", entry.book);
        let (_dir, document) = load(&name, &source);
        let xml = biblecompose_app::emit(&document, &styles()).xml;

        let mut reader = quick_xml::Reader::from_str(&xml);
        let mut buf = Vec::new();
        loop {
            use quick_xml::events::Event;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                // Attributes are only validated when they are read, so they
                // have to be read. Iterating events alone accepts a duplicate
                // attribute silently, which is the bug this test exists for.
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    for attribute in e.attributes().with_checks(true) {
                        if let Err(err) = attribute {
                            panic!("{}: emitted XML is not well formed: {err}", entry.path);
                        }
                    }
                    buf.clear();
                }
                Ok(_) => buf.clear(),
                Err(e) => panic!("{}: emitted XML is not well formed: {e}", entry.path),
            }
        }
    }
}

/// No styles.
///
/// These goldens are about one thing: USFM becoming the document structure.
/// The built-in sheet would add eighty lines of appearance above the two dozen
/// that are the subject, and would churn this file every time a point size
/// changed. The styles block has its own golden, which is where a change to it
/// belongs.
fn styles() -> biblecompose_config::ResolvedStyles {
    biblecompose_config::ResolvedStyles::default()
}
