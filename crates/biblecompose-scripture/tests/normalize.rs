//! P1.5 — USJ to `ScriptureDocument`.
//!
//! FUN-001 (the constructs normalize), FUN-002 (no text is lost), FUN-003 and
//! USFM-004 (an unsupported marker survives with a location).

use biblecompose_diagnostics::Diagnostics;
use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::{
    Align, Block, Book, BookCode, CharStyle, HeadingStyle, Inline, NoteKind, ParaStyle,
    PoetryStyle, ScriptureDocument,
};
use camino::Utf8Path;

fn run(src: &str) -> (Book, Diagnostics) {
    let document = usfm_core::Document::parse(src);
    let code = BookCode::parse("MAT").expect("MAT");
    normalize(code, Utf8Path::new("MAT.usfm"), &document)
}

fn book(src: &str) -> Book {
    run(src).0
}

/// Everything under the block, as the emitter would see it.
fn text_of(b: &Book) -> String {
    ScriptureDocument::new(vec![b.clone()]).text()
}

#[test]
fn book_names_come_off_the_identification_lines() {
    let b = book(
        "\\id MAT\n\\h Matthew\n\\toc1 The Gospel of Matthew\n\\toc2 Matthew\n\\toc3 Mat\n\\mt1 Matthew\n",
    );

    assert_eq!(b.names.running.as_deref(), Some("Matthew"));
    assert_eq!(b.names.long.as_deref(), Some("The Gospel of Matthew"));
    assert_eq!(b.names.short.as_deref(), Some("Matthew"));
    assert_eq!(b.names.abbrev.as_deref(), Some("Mat"));
    assert_eq!(b.names.title, ["Matthew"]);
    assert_eq!(b.names.for_running_head(), Some("Matthew"));

    // Identification lines are metadata about the file, not content of the
    // publication, so they produce no blocks.
    assert!(b.blocks.is_empty(), "{:?}", b.blocks);
}

/// SCR-001. Upstream emits `chapter` as a sibling of paragraphs; ours has to
/// become an anchor inside the block that follows it.
#[test]
fn a_chapter_becomes_an_anchor_on_the_following_block() {
    let b = book("\\id MAT\n\\c 1\n\\p\n\\v 1 In the beginning.\n");

    assert_eq!(b.blocks.len(), 1, "{:?}", b.blocks);
    let Block::Paragraph { content, .. } = &b.blocks[0] else {
        panic!("expected a paragraph, got {:?}", b.blocks[0]);
    };

    assert!(
        matches!(content[0], Inline::Chapter { number: 1, .. }),
        "{content:?}"
    );
    assert!(
        matches!(content[1], Inline::Verse { .. }),
        "the verse anchor should follow the chapter: {content:?}"
    );
}

/// A chapter with nothing after it still has to exist — it is a running head
/// and a PDF destination even when it opens no text.
#[test]
fn a_trailing_chapter_anchor_is_not_lost() {
    let b = book("\\id MAT\n\\p\n\\v 1 Text.\n\\c 2\n");

    let anchors: usize = b
        .blocks
        .iter()
        .filter_map(|blk| match blk {
            Block::Paragraph { content, .. } => Some(content),
            _ => None,
        })
        .flatten()
        .filter(|i| matches!(i, Inline::Chapter { number: 2, .. }))
        .count();
    assert_eq!(anchors, 1, "{:?}", b.blocks);
}

#[test]
fn verse_ranges_segments_and_published_numbers() {
    let b = book("\\id MAT\n\\p\n\\v 1-2 Range.\n\\v 3a Segment.\n\\v 4 \\vp 4b\\vp* Published.\n");
    let Block::Paragraph { content, .. } = &b.blocks[0] else {
        panic!("paragraph");
    };

    let verses: Vec<String> = content
        .iter()
        .filter_map(|i| match i {
            Inline::Verse { id, .. } => Some(id.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(verses, ["1-2", "3a", "4"]);

    let published: Vec<&str> = content
        .iter()
        .filter_map(|i| match i {
            Inline::Verse { published, .. } => published.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(published, ["4b"]);
}

#[test]
fn poetry_carries_its_family_and_its_level() {
    let b = book("\\id MAT\n\\q1 First\n\\q2 Second\n\\qr Right\n\\b\n\\q1 Third\n");

    let poetry: Vec<(PoetryStyle, u8)> = b
        .blocks
        .iter()
        .filter_map(|blk| match blk {
            Block::Poetry { style, level, .. } => Some((*style, *level)),
            _ => None,
        })
        .collect();
    assert_eq!(
        poetry,
        [
            (PoetryStyle::Q, 1),
            (PoetryStyle::Q, 2),
            (PoetryStyle::Qr, 1),
            (PoetryStyle::Q, 1),
        ]
    );
    assert!(
        b.blocks.iter().any(|blk| matches!(blk, Block::Break)),
        "the stanza break is a block of its own"
    );
}

#[test]
fn headings_and_list_items_carry_their_level() {
    let b = book("\\id MAT\n\\s1 Section\n\\s2 Sub\n\\r (Luke 3)\n\\li1 One\n\\li2 Two\n");

    let headings: Vec<(HeadingStyle, u8)> = b
        .blocks
        .iter()
        .filter_map(|blk| match blk {
            Block::Heading { style, level, .. } => Some((*style, *level)),
            _ => None,
        })
        .collect();
    assert_eq!(
        headings,
        [
            (HeadingStyle::S, 1),
            (HeadingStyle::S, 2),
            (HeadingStyle::R, 1)
        ]
    );

    let items: Vec<u8> = b
        .blocks
        .iter()
        .filter_map(|blk| match blk {
            Block::ListItem { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(items, [1, 2]);
}

/// `\pi1` is its own paragraph style, not `\pi` at level 1 — the digit is part
/// of the marker's name. Getting this wrong turns an indented paragraph into
/// an unknown one.
#[test]
fn a_marker_whose_digit_is_part_of_its_name_matches_whole() {
    let b = book("\\id MAT\n\\pi1 Indented\n\\pi2 More\n");
    let styles: Vec<ParaStyle> = b
        .blocks
        .iter()
        .filter_map(|blk| match blk {
            Block::Paragraph { style, .. } => Some(*style),
            _ => None,
        })
        .collect();
    assert_eq!(styles, [ParaStyle::Pi1, ParaStyle::Pi2]);
}

#[test]
fn character_styles_nest() {
    let b = book("\\id MAT\n\\p \\add supplied \\bd bold\\bd*\\add* plain\n");
    let Block::Paragraph { content, .. } = &b.blocks[0] else {
        panic!("paragraph");
    };

    let Some(Inline::Char {
        style: CharStyle::Add,
        content: inner,
    }) = content.iter().find(|i| matches!(i, Inline::Char { .. }))
    else {
        panic!("expected an \\add run: {content:?}");
    };
    assert!(
        inner.iter().any(|i| matches!(
            i,
            Inline::Char {
                style: CharStyle::Bd,
                ..
            }
        )),
        "the nested \\bd should be inside the \\add: {inner:?}"
    );
}

/// SCR-004. Upstream models `\x` as a note with a different marker; a
/// publication treats the two differently, so the model does too.
#[test]
fn a_cross_reference_is_not_a_note() {
    let b = book(
        "\\id MAT\n\\p\n\\v 1 Text\\f + \\fr 1:1 \\ft A footnote.\\f* more\\x - \\xo 1:1 \\xt Gen 1:1\\x*.\n",
    );
    let Block::Paragraph { content, .. } = &b.blocks[0] else {
        panic!("paragraph");
    };

    let note = content
        .iter()
        .find_map(|i| match i {
            Inline::Note(n) => Some(n),
            _ => None,
        })
        .expect("a footnote");
    assert_eq!(note.kind, NoteKind::Footnote);
    assert_eq!(note.caller, "+");
    assert_eq!(note.origin.as_deref(), Some("1:1"));
    assert!(!note.content.is_empty(), "the note body survived");

    let xref = content
        .iter()
        .find_map(|i| match i {
            Inline::Ref(r) => Some(r),
            _ => None,
        })
        .expect("a cross-reference");
    assert_eq!(xref.caller, "-");
    assert_eq!(xref.origin.as_deref(), Some("1:1"));
    assert!(!xref.content.is_empty());
}

#[test]
fn tables_keep_their_rows_cells_and_header_flag() {
    let b = book("\\id MAT\n\\tr \\th1 Name \\th2 Count\n\\tr \\tc1 Levi \\tc2 3\n");

    let Some(Block::Table { rows }) = b.blocks.iter().find(|x| matches!(x, Block::Table { .. }))
    else {
        panic!("expected a table: {:?}", b.blocks);
    };
    assert_eq!(rows.len(), 2);
    assert!(rows[0].header, "a row of \\th cells is a header row");
    assert!(!rows[1].header);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(rows[0].cells[0].align, Align::Start);
}

#[test]
fn a_figure_keeps_its_source_and_attributes() {
    let b = book("\\id MAT\n\\p\n\\fig The ark|src=\"ark.png\" size=\"span\" ref=\"1:1\"\\fig*\n");

    let Some(Block::Figure(f)) = b.blocks.iter().find(|x| matches!(x, Block::Figure(_))) else {
        panic!("expected a figure: {:?}", b.blocks);
    };
    assert_eq!(f.src, "ark.png");
    assert_eq!(f.size.as_deref(), Some("span"));
    assert_eq!(f.alt.as_deref(), Some("The ark"));
    // USFM-003: attributes we do not model are preserved rather than dropped.
    assert!(
        f.attributes.iter().any(|a| a.key == "ref"),
        "{:?}",
        f.attributes
    );
}

/// FUN-003 and USFM-004, and the failure that is easiest to write by accident:
/// an unrecognised paragraph marker swallows everything after it in the USJ
/// tree, so treating it as a leaf silently deletes the rest of the chapter.
#[test]
fn an_unsupported_marker_keeps_the_text_underneath_it() {
    let (b, d) = run("\\id MAT\n\\zmine custom\n\\p\n\\v 1 This text must survive.\n");

    let text = text_of(&b);
    assert!(
        text.contains("This text must survive."),
        "text was lost under the unsupported marker: {text:?}"
    );
    // `\id` runs to the end of its line, so the parser lowers the marker on
    // the next line as a child of the book node. Discarding that node — which
    // is the obvious way to skip the `\id` description — deletes this word.
    assert!(
        text.contains("custom"),
        "text nested under \\id was dropped: {text:?}"
    );

    let doc = ScriptureDocument::new(vec![b]);
    let unsupported = doc.unsupported();
    assert_eq!(unsupported.len(), 1, "{unsupported:?}");
    assert_eq!(unsupported[0].marker, "zmine");

    // USFM-004 wants a location, not just a name.
    let at = unsupported[0]
        .location
        .as_ref()
        .expect("the marker's location");
    assert_eq!(at.path, "MAT.usfm");
    assert_eq!(at.line, Some(2), "reported at {at}");

    // Reported, but not fatal: an unknown marker is not a reason to refuse to
    // typeset a Gospel.
    assert!(d.blocking().next().is_none(), "{d:?}");
    assert_eq!(
        d.iter().map(|x| x.code.as_str()).collect::<Vec<_>>(),
        ["USFM-003"]
    );
}

#[test]
fn an_unsupported_character_style_keeps_its_words() {
    let (b, _) = run("\\id MAT\n\\p \\v 1 Plain \\zzz styled\\zzz* end.\n");
    let text = text_of(&b);
    assert!(text.contains("styled"), "{text:?}");
    assert!(text.contains("end."), "{text:?}");
}

/// FUN-002 across every construct at once: normalization moves text between
/// structures, and the one thing it must never do is lose a word.
#[test]
fn no_scripture_text_is_lost_across_the_construct_set() {
    let (b, _) = run(concat!(
        "\\id MAT Matthew\n\\h Matthew\n\\mt1 Matthew\n",
        "\\c 1\n\\s1 The Genealogy\n",
        "\\p\n\\v 1 alpha \\add beta\\add* gamma\n",
        "\\q1 delta\n\\q2 epsilon\n",
        "\\li1 zeta\n",
        "\\p\n\\v 2 eta \\w theta|strong=\"H1\"\\w* iota\n",
        "\\zmine kappa\n",
    ));

    let text = text_of(&b);
    for word in [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
    ] {
        assert!(text.contains(word), "{word} is missing from {text:?}");
    }

    // Apparatus is not Scripture text and must not leak into the comparison.
    let (with_note, _) = run("\\id MAT\n\\p\n\\v 1 body\\f + \\ft aside.\\f*\n");
    let t = text_of(&with_note);
    assert!(t.contains("body"), "{t:?}");
    assert!(
        !t.contains("aside"),
        "a note is apparatus, not body text: {t:?}"
    );
}

#[test]
fn milestones_survive_with_their_attributes() {
    let (b, _) = run("\\id MAT\n\\p \\v 1 \\qt-s |who=\"Jesus\"\\*Follow me\\qt-e\\*.\n");
    let Block::Paragraph { content, .. } = &b.blocks[0] else {
        panic!("paragraph");
    };

    let milestones: Vec<(&str, bool)> = content
        .iter()
        .filter_map(|i| match i {
            Inline::Milestone(m) => Some((m.marker.as_str(), m.start)),
            _ => None,
        })
        .collect();
    assert_eq!(milestones, [("qt-s", true), ("qt-e", false)]);

    let start = content
        .iter()
        .find_map(|i| match i {
            Inline::Milestone(m) if m.start => Some(m),
            _ => None,
        })
        .expect("a start milestone");
    assert!(
        start.attributes.iter().any(|a| a.key == "who"),
        "{:?}",
        start.attributes
    );
}
