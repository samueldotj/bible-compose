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

/// Two chapters carrying every question the apparatus has to answer (P4.1,
/// P4.2).
///
/// Built for the layout, not for the model — [`kitchen_sink`] already proves a
/// note and a cross-reference normalize and emit. What this one is for is what
/// only a real page can show: that two sequences run side by side without
/// interleaving, that an editor's own caller is printed and skipped over, that
/// a note longer than the note area splits across pages, and that a caller
/// sequence starts again at a chapter.
///
/// Deliberately in one book with two chapters, because the restart boundary is
/// the chapter and a fixture with one chapter cannot show it.
pub fn apparatus() -> ScriptureDocument {
    // Long enough that the note area cannot hold it, so the insertion splits
    // and continues on the next page. Assembled rather than written out: the
    // property that matters is the length, and 6,000 characters of prose in a
    // source file would bury the fixture it belongs to.
    let long = format!(
        "A note too long for one page's note area, so that it has to split. {}",
        "Repeated filler that carries the note past the foot of the page. ".repeat(90)
    );

    let reference = |origin: &str, body: &str| {
        Inline::Ref(CrossReference {
            caller: "+".to_owned(),
            origin: Some(origin.to_owned()),
            content: vec![Inline::Char {
                style: CharStyle::Xt,
                content: vec![text(body)],
            }],
        })
    };

    ScriptureDocument::new(vec![Book::new(
        book("1JN"),
        BookNames::named("1 John"),
        vec![
            Block::Paragraph {
                style: ParaStyle::P,
                content: vec![
                    chapter(1),
                    verse(1),
                    text("That which was from the beginning"),
                    reference("1:1", "John 1:1; John 1:14"),
                    text(", which we have heard, which we have seen with our own eyes. "),
                    verse(2),
                    text("And this is the life that was revealed"),
                    footnote("1:2", "Or everlasting life."),
                    text("; we have seen it and testified to it. "),
                    verse(3),
                    text("We proclaim to you what we have seen and heard"),
                    reference("1:3", "Acts 4:20"),
                    text(", so that you also may have fellowship with us. "),
                    verse(4),
                    text("We write these things so that our"),
                    // An editor's own caller: printed as written, and it does
                    // not take a place in the sequence.
                    Inline::Note(Note {
                        kind: NoteKind::Footnote,
                        caller: "*".to_owned(),
                        origin: Some("1:4".to_owned()),
                        content: vec![Block::Paragraph {
                            style: ParaStyle::P,
                            content: vec![text("BYZ and TR read "), text("your.")],
                        }],
                    }),
                    text(" joy may be complete. "),
                    verse(5),
                    text("And this is the message we have heard from Him"),
                    footnote("1:5", &long),
                    text(": God is light, and in Him there is no darkness at all."),
                ],
            },
            Block::Paragraph {
                style: ParaStyle::P,
                content: vec![
                    chapter(2),
                    verse(1),
                    text("My little children, I am writing these things to you"),
                    footnote("2:1", "The first note of a new chapter."),
                    text(" so that you will not sin. "),
                    verse(2),
                    text("He Himself is the atoning sacrifice for our sins"),
                    reference("2:2", "John 14:15"),
                    text(", and not only for ours but also for the whole world."),
                ],
            },
        ],
    )])
}

/// A chapter whose second verse fills several pages on its own (P4.4).
///
/// The running head is built from the references a page *collects*, and
/// `chapterverse` collects one where a verse number is typeset — so a page
/// wholly inside one verse collects nothing. This is the fixture that shows
/// it: verse 1 and verses 3 and 4 are a line each, and verse 2 is pages long,
/// so the pages in the middle have no verse starting on them at all.
///
/// Deliberately short of the length at which a single paragraph starts
/// producing trailing blank pages — see the note on P4.4 in the roadmap. Five
/// pages is enough to have three with no verse of their own and few enough to
/// typeset in a test.
pub fn long_verse() -> ScriptureDocument {
    let sentence = "and they went out and preached everywhere while the Lord \
                    worked with them and confirmed the word by the signs that \
                    accompanied it ";
    ScriptureDocument::new(vec![Book::new(
        book("1JN"),
        BookNames::named("1 John"),
        vec![Block::Paragraph {
            style: ParaStyle::P,
            content: vec![
                chapter(1),
                verse(1),
                text("That which was from the beginning, which we have heard. "),
                verse(2),
                text(&format!("{}.", sentence.repeat(100).trim())),
                verse(3),
                text(" We proclaim to you what we have seen and heard. "),
                verse(4),
                text("We write these things so that our joy may be complete."),
            ],
        }],
    )])
}

/// Every kind of heading USFM has, in one psalm (P4.6).
///
/// Six of them share the `heading` element and differ only by the style they
/// resolve to, so this is the fixture that shows whether a reader can tell them
/// apart. A psalm, because `\d` and `\sp` belong to psalms and to Job and the
/// Song, and putting them anywhere else would be a fixture nobody recognises.
pub fn headings() -> ScriptureDocument {
    let says = |style: HeadingStyle, level: u8, words: &str| Block::Heading {
        style,
        level,
        content: vec![text(words)],
    };
    let verse_of = |n: u16, words: &str| Block::Paragraph {
        style: ParaStyle::P,
        content: vec![verse(n), text(words)],
    };

    ScriptureDocument::new(vec![Book::new(
        book("PSA"),
        BookNames::named("Psalms"),
        vec![
            // The chapter anchor lives in the superscription, which is where
            // USJ puts it when a psalm has one.
            Block::Heading {
                style: HeadingStyle::D,
                level: 1,
                content: vec![
                    chapter(3),
                    text("A Psalm of David, when he fled from Absalom his son."),
                ],
            },
            says(HeadingStyle::S, 1, "A first-level heading"),
            says(HeadingStyle::R, 1, "(2 Samuel 15:13-30)"),
            verse_of(1, "O LORD, how many are my foes!"),
            says(HeadingStyle::S, 2, "A second-level heading"),
            verse_of(2, "Many are saying of me."),
            says(HeadingStyle::S, 3, "A third-level heading"),
            verse_of(3, "But You, O LORD, are a shield around me."),
            says(HeadingStyle::S, 4, "A fourth-level heading"),
            verse_of(4, "To the LORD I cry aloud."),
            says(HeadingStyle::Sp, 1, "The Beloved"),
            verse_of(5, "I lie down and sleep."),
            says(HeadingStyle::Sr, 1, "(3:1-8)"),
            verse_of(6, "I will not fear the tens of thousands."),
        ],
    )])
}

/// Enough prose to fill a column, then a heading — so that, laid out without
/// care, the heading is the last line the column can hold (P4.6).
///
/// The lengths are chosen against the built-in page rather than guessed: a
/// heading that landed comfortably mid-column would make the test pass without
/// testing anything, so the paragraph before each one is sized to run the
/// column out.
pub fn orphan_heading() -> ScriptureDocument {
    let sentence = "and they went out and preached everywhere while the Lord \
                    worked with them and confirmed the word by the signs that \
                    accompanied it. ";
    let mut blocks = vec![Block::Paragraph {
        style: ParaStyle::P,
        content: vec![chapter(1), verse(1), text(sentence)],
    }];

    // Several headings at different depths into the column, so at least one of
    // them lands where a break would want to be whatever the exact measure.
    for (n, fill) in [(2u16, 11usize), (3, 13), (4, 15), (5, 17)] {
        blocks.push(Block::Paragraph {
            style: ParaStyle::P,
            content: vec![verse(n), text(&sentence.repeat(fill))],
        });
        blocks.push(Block::Heading {
            style: HeadingStyle::S,
            level: 1,
            content: vec![text(&format!("Heading {n}"))],
        });
        blocks.push(Block::Paragraph {
            style: ParaStyle::P,
            content: vec![verse(n + 100), text(sentence)],
        });
    }

    ScriptureDocument::new(vec![Book::new(
        book("MRK"),
        BookNames::named("Mark"),
        blocks,
    )])
}

/// Every fixture, for tests that should run over all of them.
pub fn all() -> Vec<(&'static str, ScriptureDocument)> {
    vec![
        ("john_1_1_5", john_1_1_5()),
        ("two_books", two_books()),
        ("kitchen_sink", kitchen_sink()),
        ("adversarial", adversarial()),
        ("apparatus", apparatus()),
        ("long_verse", long_verse()),
        ("headings", headings()),
        ("orphan_heading", orphan_heading()),
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
