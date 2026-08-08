//! Hand-built documents, with no parser involved.
//!
//! M0's whole point: prove the second half of the pipeline before the first
//! half exists. Everything downstream — the emitter, the golden files, the
//! backend invocation, the CLI — is exercised against these.
//!
//! The Scripture text is the Berean Standard Bible (public domain), the same
//! passage the S0 spike set by hand, so an M0 PDF can be compared against
//! `spike/out/render/` by eye.

use camino::Utf8PathBuf;

use crate::{
    canon::BookCode, Align, Attribute, Block, Book, BookNames, Cell, CharStyle, CrossReference,
    FigureRef, HeadingStyle, Inline, Milestone, Note, NoteKind, ParaStyle, PoetryStyle, Row,
    ScriptureDocument, Unsupported, VerseId,
};

fn book(code: &str) -> BookCode {
    BookCode::parse(code).expect("fixture uses a real book code")
}

fn text(s: &str) -> Inline {
    Inline::Text(s.to_owned())
}

fn verse(n: u16) -> Inline {
    Inline::Verse {
        id: VerseId::single(n),
        published: None,
        alternate: None,
    }
}

fn chapter(n: u16) -> Inline {
    Inline::Chapter {
        number: n,
        published: None,
        alternate: None,
    }
}

fn footnote(origin: &str, body: &str) -> Inline {
    Inline::Note(Note {
        kind: NoteKind::Footnote,
        caller: "+".to_owned(),
        origin: Some(origin.to_owned()),
        content: vec![Block::Paragraph {
            style: ParaStyle::P,
            content: vec![text(body)],
        }],
    })
}

/// The smallest useful document: one book, one chapter, five verses, one note.
pub fn john_1_1_5() -> ScriptureDocument {
    ScriptureDocument::new(vec![Book::new(
        book("JHN"),
        BookNames::named("John"),
        vec![
            Block::Heading {
                style: HeadingStyle::S,
                level: 1,
                content: vec![text("The Beginning")],
            },
            Block::Paragraph {
                style: ParaStyle::M,
                content: vec![
                    chapter(1),
                    verse(1),
                    text("In the beginning was the Word, and the Word was with God, and the Word was God."),
                    verse(2),
                    text("He was with God in the beginning."),
                    verse(3),
                    text("Through Him all things were made, and without Him nothing was made that has been made."),
                    verse(4),
                    text("In Him was life, and that life was the light of men."),
                    verse(5),
                    text("The Light shines in the darkness, and the darkness has not overcome"),
                    footnote("1:5", "Or comprehended"),
                    text(" it."),
                ],
            },
        ],
    )])
}

/// Two books, so canonical ordering and per-book running heads are exercised.
pub fn two_books() -> ScriptureDocument {
    let mut doc = john_1_1_5();
    doc.books.insert(
        0,
        Book::new(
            book("GEN"),
            BookNames::named("Genesis"),
            vec![Block::Paragraph {
                style: ParaStyle::P,
                content: vec![
                    chapter(1),
                    verse(1),
                    text("In the beginning God created the heavens and the earth."),
                ],
            }],
        ),
    );
    doc
}

/// Every construct the model can represent, in one book.
///
/// Not realistic Scripture — deliberately. It exists so that P0.3's coverage
/// test, the golden XML, and the emitter's match arms are all exercised by a
/// single fixture, and so a new variant added to `Block` or `Inline` without a
/// fixture fails a test rather than sailing through untested.
pub fn kitchen_sink() -> ScriptureDocument {
    ScriptureDocument::new(vec![Book::new(
        book("PSA"),
        BookNames {
            running: Some("Psalms".into()),
            long: Some("The Book of Psalms".into()),
            short: Some("Psalms".into()),
            abbrev: Some("Ps".into()),
            title: vec!["The Psalms".into()],
        },
        vec![
            Block::Heading {
                style: HeadingStyle::S,
                level: 1,
                content: vec![text("A heading")],
            },
            Block::Heading {
                style: HeadingStyle::R,
                level: 1,
                content: vec![text("(Genesis 1:1-2)")],
            },
            Block::Paragraph {
                style: ParaStyle::P,
                content: vec![
                    chapter(23),
                    verse(1),
                    text("Plain text, then "),
                    Inline::Char {
                        style: CharStyle::Nd,
                        content: vec![text("the LORD")],
                    },
                    text(", then "),
                    Inline::Char {
                        style: CharStyle::Wj,
                        content: vec![
                            text("words of Jesus with "),
                            Inline::Char {
                                style: CharStyle::It,
                                content: vec![text("nesting")],
                            },
                        ],
                    },
                    text("."),
                    footnote("23:1", "A footnote body."),
                    Inline::Ref(CrossReference {
                        caller: "-".to_owned(),
                        origin: Some("23:1".to_owned()),
                        content: vec![text("John 10:11; Isaiah 40:11")],
                    }),
                    Inline::Milestone(Milestone {
                        marker: "qt-s".to_owned(),
                        start: true,
                        attributes: vec![Attribute {
                            key: "who".to_owned(),
                            value: "David".to_owned(),
                        }],
                    }),
                    text(" quoted speech "),
                    Inline::Milestone(Milestone {
                        marker: "qt-e".to_owned(),
                        start: false,
                        attributes: vec![],
                    }),
                    Inline::Unsupported(Unsupported {
                        marker: "zmystery".to_owned(),
                        text: "custom content".to_owned(),
                        location: None,
                    }),
                ],
            },
            Block::Poetry {
                style: PoetryStyle::Q,
                level: 1,
                content: vec![verse(2), text("A line of poetry,")],
            },
            Block::Poetry {
                style: PoetryStyle::Q,
                level: 2,
                content: vec![text("indented further.")],
            },
            Block::Break,
            Block::ListItem {
                level: 1,
                content: vec![verse(3), text("A list item.")],
            },
            Block::Table {
                rows: vec![
                    Row {
                        header: true,
                        cells: vec![
                            Cell {
                                align: Align::Start,
                                content: vec![text("Tribe")],
                            },
                            Cell {
                                align: Align::End,
                                content: vec![text("Number")],
                            },
                        ],
                    },
                    Row {
                        header: false,
                        cells: vec![
                            Cell {
                                align: Align::Start,
                                content: vec![text("Reuben")],
                            },
                            Cell {
                                align: Align::End,
                                content: vec![text("46,500")],
                            },
                        ],
                    },
                ],
            },
            Block::Figure(FigureRef {
                src: Utf8PathBuf::from("assets/images/map.png"),
                alt: Some("A map".to_owned()),
                caption: Some("The land, as divided.".to_owned()),
                size: Some("col".to_owned()),
                attributes: vec![Attribute {
                    key: "copy".to_owned(),
                    value: "Public domain".to_owned(),
                }],
            }),
        ],
    )])
}

/// Text engineered to break a string-templating emitter.
///
/// Every character here is one that means something in SIL syntax. S0.6
/// measured what happens when this reaches SILE as syntax rather than as data:
/// a build that reports no errors and silently reflows the verse
/// (spike/NOTES.md F-13). The golden test and the escaping test both use it.
pub fn adversarial() -> ScriptureDocument {
    ScriptureDocument::new(vec![Book::new(
        book("MAT"),
        BookNames::named("Matthew {literal} \\bd"),
        vec![Block::Paragraph {
            style: ParaStyle::P,
            content: vec![
                chapter(1),
                verse(1),
                text("Backslash \\bd and \\par and \\skip[height=40pt] are text. "),
                text("Braces {like this} and 100% and an ampersand & and <angle> brackets. "),
                Inline::Char {
                    style: CharStyle::Bd,
                    content: vec![text("Even \\em{inside} a character style.")],
                },
                footnote(
                    "1:1",
                    "A note with \\footnote{nested} and % a comment marker.",
                ),
            ],
        }],
    )])
}

/// Every fixture, for tests that should run over all of them.
pub fn all() -> Vec<(&'static str, ScriptureDocument)> {
    vec![
        ("john_1_1_5", john_1_1_5()),
        ("two_books", two_books()),
        ("kitchen_sink", kitchen_sink()),
        ("adversarial", adversarial()),
    ]
}

/// Fixture names, in the order [`all`] returns them.
pub fn names() -> Vec<&'static str> {
    all().into_iter().map(|(n, _)| n).collect()
}

/// Look one up by name, for the CLI.
pub fn by_name(name: &str) -> Option<ScriptureDocument> {
    all().into_iter().find(|(n, _)| *n == name).map(|(_, d)| d)
}
