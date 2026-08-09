//! USJ to [`Book`] — *what the file says* becoming *what the publication is*.
//!
//! P1.5, and the seam [ADR-001](../../../docs/adr/001-usfm-core.md) draws.
//! Upstream hands us a source-faithful tree keyed by marker; this decides what
//! each marker *means* for a publication, and refuses to lose anything on the
//! way.
//!
//! Four rules, each of which the tests hold:
//!
//! **Nothing is dropped** (FUN-002). A marker we do not support becomes
//! [`Unsupported`] carrying its location, and — this is the part that is easy
//! to get wrong — if it had children, those children are still normalized.
//! An unrecognised paragraph marker swallows the rest of a chapter in the USJ
//! tree, so treating it as a leaf would silently delete Scripture.
//!
//! **Chapter and verse are anchors** (SCR-001). Upstream emits `chapter` as a
//! sibling of paragraphs, not a parent, which is the same shape our model
//! wants — but it means a chapter arriving between blocks has to be attached
//! to the block that follows it rather than becoming one.
//!
//! **A cross-reference is not a note** (SCR-004). Upstream models `\x` as a
//! `note` with a different marker, because that is what USJ says. Ours are
//! separate types, because a publication treats them differently: footnotes
//! go to the foot, cross-references to a column or a margin.
//!
//! **Unknown is not the same as unsupported.** USJ's `unknown` covers the `\z`
//! namespace, which the specification leaves open on purpose. It is reported
//! the same way, because BibleCompose cannot typeset it either, but the
//! diagnostic says so differently.

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, SourceLoc};
use camino::Utf8Path;
use usfm_core::{Document, Node, NodeKind};

use crate::{
    Align, Attribute, Block, Book, BookCode, BookNames, Cell, CharStyle, CrossReference, FigureRef,
    HeadingStyle, Inline, Milestone, Note, NoteKind, ParaStyle, PoetryStyle, Row, Unsupported,
    VerseId,
};

/// Normalize one parsed file into a publication book.
///
/// The book code comes from discovery rather than from the tree: discovery has
/// already read the `\id` marker, resolved it against the canon, and refused
/// the file if two claimed the same book. Re-deriving it here would be a
/// second answer to a question already settled.
pub fn normalize(code: BookCode, path: &Utf8Path, document: &Document) -> (Book, Diagnostics) {
    let mut cx = Cx {
        path,
        document,
        diagnostics: Diagnostics::new(),
        hoisted: Vec::new(),
    };

    let mut names = BookNames::default();
    let mut blocks = Vec::new();
    // A chapter or verse arriving between blocks, waiting for one to attach to.
    let mut pending: Vec<Inline> = Vec::new();

    for node in document.content() {
        cx.top_level(node, &mut names, &mut blocks, &mut pending);
    }

    // An anchor with nothing after it still has to exist: a chapter marker at
    // the end of a file is unusual, but losing it would lose a PDF
    // destination and a running head (SCR-001).
    if !pending.is_empty() {
        blocks.push(Block::Paragraph {
            style: ParaStyle::P,
            content: std::mem::take(&mut pending),
        });
    }
    blocks.append(&mut cx.hoisted);

    (Book::new(code, names, blocks), cx.diagnostics)
}

struct Cx<'a> {
    path: &'a Utf8Path,
    document: &'a Document,
    diagnostics: Diagnostics,
    /// Figures found while walking inline content.
    ///
    /// `\fig` is a character-level marker, so it arrives inside a paragraph —
    /// but a figure is a float, and the model has it as a block. Collected
    /// here and emitted after the block that contained it, which is where a
    /// float would land anyway; SILE decides the final position.
    hoisted: Vec<Block>,
}

impl Cx<'_> {
    fn top_level(
        &mut self,
        node: &Node,
        names: &mut BookNames,
        blocks: &mut Vec<Block>,
        pending: &mut Vec<Inline>,
    ) {
        match node.kind {
            // The `\id` line. Discovery already used the code, and the free
            // text after it describes the file rather than the publication.
            //
            // But `\id` runs to the end of its line, so a marker on the *next*
            // line can be lowered as its child — and that marker is real
            // content. Dropping the whole node would delete it silently, which
            // is the failure FUN-002 exists to prevent.
            NodeKind::Book => {
                let marked: Vec<Node> = node
                    .children
                    .iter()
                    .filter(|c| c.kind != NodeKind::Text)
                    .cloned()
                    .collect();
                if !marked.is_empty() {
                    let inline = self.inlines(&marked);
                    pending.extend(inline);
                }
            }

            NodeKind::Chapter | NodeKind::Verse => {
                if let Some(anchor) = self.anchor(node) {
                    pending.push(anchor);
                }
            }

            NodeKind::Para => {
                let marker = node.marker.as_ref().map(usfm_core::Marker::as_str);
                let Some(marker) = marker else {
                    // A paragraph with no marker is not something the parser
                    // produces; carrying it as unsupported is cheaper than a
                    // panic and says more than ignoring it.
                    pending.push(self.unsupported(node, "para", "a paragraph with no marker"));
                    return;
                };

                if self.book_name(marker, node, names) {
                    return;
                }

                let content = {
                    let mut c = std::mem::take(pending);
                    c.extend(self.inlines(&node.children));
                    c
                };

                match classify(marker) {
                    Para::Paragraph(style) => blocks.push(Block::Paragraph { style, content }),
                    Para::Poetry(style, level) => {
                        blocks.push(Block::Poetry {
                            style,
                            level,
                            content,
                        });
                    }
                    Para::Heading(style, level) => blocks.push(Block::Heading {
                        style,
                        level,
                        content,
                    }),
                    Para::ListItem(level) => blocks.push(Block::ListItem { level, content }),
                    Para::Break => {
                        // `\b` carries nothing, but an anchor waiting on it
                        // must not be discarded with it.
                        if !content.is_empty() {
                            blocks.push(Block::Paragraph {
                                style: ParaStyle::P,
                                content,
                            });
                        }
                        blocks.push(Block::Break);
                    }
                    Para::Unknown => {
                        let mut c = content;
                        c.insert(
                            0,
                            self.unsupported(
                                node,
                                marker,
                                "this paragraph marker is not supported",
                            ),
                        );
                        blocks.push(Block::Paragraph {
                            style: ParaStyle::P,
                            content: c,
                        });
                    }
                }
            }

            NodeKind::Table => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph {
                        style: ParaStyle::P,
                        content: std::mem::take(pending),
                    });
                }
                blocks.push(Block::Table {
                    rows: self.rows(&node.children),
                });
            }

            NodeKind::Figure => blocks.push(Block::Figure(self.figure(node))),

            // Anything else at top level is inline content the parser placed
            // outside a paragraph — malformed, but the text is still the
            // author's and must survive.
            _ => {
                let inline = self.inlines(std::slice::from_ref(node));
                pending.extend(inline);
            }
        }

        // A figure found inside the block just handled belongs after it, not
        // at the end of the book.
        blocks.append(&mut self.hoisted);
    }

    /// `\h`, `\toc1`–`\toc3`, `\mt1`–`\mt4`. Returns whether it was one.
    fn book_name(&mut self, marker: &str, node: &Node, names: &mut BookNames) -> bool {
        let text = flatten(&node.children);
        match marker {
            "h" | "h1" => names.running = Some(text),
            "toc1" => names.long = Some(text),
            "toc2" => names.short = Some(text),
            "toc3" => names.abbrev = Some(text),
            "mt" | "mt1" | "mt2" | "mt3" | "mt4" => names.title.push(text),
            // Identification lines that are metadata rather than content.
            // Carried nowhere on purpose: they describe the file, and the
            // publication is not the file.
            "ide" | "sts" | "rem" | "usfm" | "toca1" | "toca2" | "toca3" => {}
            _ => return false,
        }
        true
    }

    fn inlines(&mut self, nodes: &[Node]) -> Vec<Inline> {
        let mut out = Vec::new();
        for node in nodes {
            match node.kind {
                NodeKind::Text => {
                    if let Some(t) = &node.text {
                        out.push(Inline::Text(t.clone()));
                    }
                }
                NodeKind::Chapter | NodeKind::Verse => {
                    if let Some(a) = self.anchor(node) {
                        out.push(a);
                    }
                }
                NodeKind::Char => {
                    let marker = node.marker.as_ref().map_or("", usfm_core::Marker::as_str);
                    match char_style(marker) {
                        Some(style) => out.push(Inline::Char {
                            style,
                            content: self.inlines(&node.children),
                        }),
                        None => {
                            // Unsupported *styling*, not unsupported text: the
                            // marker is reported and the words go through.
                            out.push(self.unsupported(
                                node,
                                marker,
                                "this character style is not supported",
                            ));
                            out.extend(self.inlines(&node.children));
                        }
                    }
                }
                NodeKind::Figure => {
                    let figure = self.figure(node);
                    self.hoisted.push(Block::Figure(figure));
                }
                NodeKind::Note => out.push(self.note(node)),
                NodeKind::Milestone => out.push(Inline::Milestone(self.milestone(node))),
                NodeKind::OptBreak => {
                    // `//` — a discretionary line break. Nothing in the model
                    // holds one yet, and it carries no text, so it is dropped
                    // rather than reported: a diagnostic per soft break would
                    // bury the panel on a poetry book.
                }
                NodeKind::Reference => out.extend(self.inlines(&node.children)),
                NodeKind::Unknown => {
                    let marker = node.marker.as_ref().map_or("z", usfm_core::Marker::as_str);
                    out.push(self.unsupported(
                        node,
                        marker,
                        "this is a custom \\z marker, which has no defined meaning",
                    ));
                    out.extend(self.inlines(&node.children));
                }
                // A block arriving inside inline content is the parser being
                // honest about malformed input. Flatten rather than drop.
                _ => out.extend(self.inlines(&node.children)),
            }
        }
        out
    }

    fn anchor(&mut self, node: &Node) -> Option<Inline> {
        let number = node.attribute("number").unwrap_or_default();
        let published = node.attribute("pubnumber").map(str::to_owned);

        if node.kind == NodeKind::Chapter {
            let Some(n) = number.trim().parse::<u16>().ok() else {
                self.diagnostics.push(self.at(
                    Diagnostic::warning(
                        code::UNSUPPORTED_MARKER,
                        format!("chapter number {number:?} is not a number"),
                    ),
                    node,
                ));
                return None;
            };
            return Some(Inline::Chapter {
                number: n,
                published,
                alternate: node
                    .attribute("altnumber")
                    .and_then(|a| a.trim().parse().ok()),
            });
        }

        let id = parse_verse(number)?;
        Some(Inline::Verse {
            id,
            published,
            alternate: node.attribute("altnumber").and_then(parse_verse),
        })
    }

    fn note(&mut self, node: &Node) -> Inline {
        let marker = node.marker.as_ref().map_or("f", usfm_core::Marker::as_str);
        let caller = node.attribute("caller").unwrap_or("+").to_owned();

        // SCR-004: `\x` is a cross-reference, not a note with a flag.
        if marker.starts_with('x') {
            let (origin, content) = self.apparatus(&node.children, "xo");
            return Inline::Ref(CrossReference {
                caller,
                origin,
                content,
            });
        }

        let kind = if marker == "fe" {
            NoteKind::Endnote
        } else {
            NoteKind::Footnote
        };
        let (origin, content) = self.apparatus(&node.children, "fr");

        Inline::Note(Note {
            kind,
            caller,
            origin,
            // USFM-002: a note holds block content, not a flat string. The
            // parser gives a flat run of char nodes, so one paragraph is the
            // honest translation of it — a note with genuine paragraphs is
            // rare and would arrive as nested paras, which flatten below.
            content: if content.is_empty() {
                Vec::new()
            } else {
                vec![Block::Paragraph {
                    style: ParaStyle::P,
                    content,
                }]
            },
        })
    }

    /// Split an origin reference (`\fr`, `\xo`) off the front of note content.
    fn apparatus(&mut self, nodes: &[Node], origin_marker: &str) -> (Option<String>, Vec<Inline>) {
        let mut origin = None;
        let mut rest = Vec::new();

        for node in nodes {
            let marker = node.marker.as_ref().map_or("", usfm_core::Marker::as_str);
            if marker == origin_marker && origin.is_none() {
                origin = Some(flatten(&node.children));
                continue;
            }
            rest.push(node.clone());
        }

        (origin, self.inlines(&rest))
    }

    fn milestone(&self, node: &Node) -> Milestone {
        let marker = node.marker.as_ref().map_or("", usfm_core::Marker::as_str);
        Milestone {
            marker: marker.to_owned(),
            // `-s` starts, `-e` ends; a self-closing milestone is a start.
            start: !marker.ends_with("-e"),
            attributes: attributes(node),
        }
    }

    fn figure(&mut self, node: &Node) -> FigureRef {
        FigureRef {
            // USJ calls it `file`; the model calls it `src`, because that is
            // what the USFM attribute is called and what a user will look for.
            src: node.attribute("file").unwrap_or_default().into(),
            // `\fig caption|alt="..." src="..."\fig*` — the content is the
            // **caption** and `alt` is a separate attribute. Reading the
            // content as `alt` left `caption` empty and, when a file supplied
            // a real `alt` too, emitted the attribute twice. A Malayalam
            // Ephesians in the corpus does exactly that, and the resulting XML
            // was not well formed.
            caption: {
                let caption = flatten(&node.children);
                (!caption.is_empty()).then_some(caption)
            },
            alt: node.attribute("alt").map(str::to_owned),
            size: node.attribute("size").map(str::to_owned),
            attributes: attributes(node),
        }
    }

    fn rows(&mut self, nodes: &[Node]) -> Vec<Row> {
        nodes
            .iter()
            .filter(|n| n.kind == NodeKind::TableRow)
            .map(|row| Row {
                header: row.children.iter().any(|c| {
                    c.marker
                        .as_ref()
                        .is_some_and(|m| m.as_str().starts_with("th"))
                }),
                cells: row
                    .children
                    .iter()
                    .filter(|c| c.kind == NodeKind::TableCell)
                    .map(|cell| Cell {
                        align: if cell.attribute("align") == Some("end") {
                            Align::End
                        } else {
                            Align::Start
                        },
                        content: self.inlines(&cell.children),
                    })
                    .collect(),
            })
            .collect()
    }

    fn unsupported(&mut self, node: &Node, marker: &str, why: &str) -> Inline {
        let location = self.location(node);
        self.diagnostics.push(
            Diagnostic::warning(code::UNSUPPORTED_MARKER, format!("\\{marker}: {why}"))
                .help("the text is kept; only the marker's effect is lost")
                .at(location
                    .clone()
                    .unwrap_or_else(|| SourceLoc::file(self.path))),
        );

        Inline::Unsupported(Unsupported {
            marker: marker.to_owned(),
            // Empty on purpose. Every caller normalizes this node's children
            // into the surrounding content, so copying the text here as well
            // would emit it twice — which the M1 construct golden caught, and
            // which is a worse failure than the loss FUN-002 guards against:
            // a duplicated verse reads as Scripture.
            //
            // `Unsupported` is a record that a marker's *effect* was lost. The
            // words are not lost, so they are not its to carry.
            text: String::new(),
            location,
        })
    }

    fn location(&self, node: &Node) -> Option<SourceLoc> {
        let span = node.span.as_ref()?;
        let at = self.document.line_col(span.start)?;
        Some(SourceLoc::at(self.path, at.line, at.column))
    }

    fn at(&self, d: Diagnostic, node: &Node) -> Diagnostic {
        match self.location(node) {
            Some(loc) => d.at(loc),
            None => d.at(SourceLoc::file(self.path)),
        }
    }
}

fn attributes(node: &Node) -> Vec<Attribute> {
    node.attributes
        .iter()
        // `number`, `sid` and friends are modelled as fields, not attributes;
        // repeating them here would be two places to change.
        .filter(|a| {
            !matches!(
                a.key.as_str(),
                "number"
                    | "sid"
                    | "eid"
                    | "caller"
                    | "altnumber"
                    | "pubnumber"
                    | "style"
                    | "align"
                    // A figure carries these as `src` and `size`; emitting
                    // them again produces duplicate XML attributes, which is
                    // a well-formedness error rather than an untidiness.
                    | "file"
                    | "src"
                    | "size"
                    | "alt"
            )
        })
        .map(|a| Attribute {
            key: a.key.clone(),
            value: a.value.clone(),
        })
        .collect()
}

/// All the text under a node, in document order.
fn flatten(nodes: &[Node]) -> String {
    let mut out = String::new();
    fn go(nodes: &[Node], out: &mut String) {
        for n in nodes {
            if let Some(t) = &n.text {
                out.push_str(t);
            }
            go(&n.children, out);
        }
    }
    go(nodes, &mut out);
    out.trim().to_owned()
}

enum Para {
    Paragraph(ParaStyle),
    Poetry(PoetryStyle, u8),
    Heading(HeadingStyle, u8),
    ListItem(u8),
    Break,
    Unknown,
}

/// Split a marker into its family and its level: `q2` is `q` at 2, `s` is `s`
/// at 1. A level is the last character and only when it is a digit, so `toc1`
/// and `pi1` — where the digit is part of the name — are matched whole first.
fn split_level(marker: &str) -> (&str, u8) {
    match marker.chars().last() {
        Some(d) if d.is_ascii_digit() && marker.len() > 1 => (
            &marker[..marker.len() - 1],
            d.to_digit(10).unwrap_or(1) as u8,
        ),
        _ => (marker, 1),
    }
}

fn classify(marker: &str) -> Para {
    if marker == "b" {
        return Para::Break;
    }
    // Whole-marker match first: `pi1` is its own paragraph style, not `pi`
    // at level 1.
    if let Some(style) = ParaStyle::all().iter().find(|s| s.marker() == marker) {
        return Para::Paragraph(*style);
    }

    let (family, level) = split_level(marker);

    if let Some(style) = PoetryStyle::all().iter().find(|s| s.marker() == family) {
        return Para::Poetry(*style, level);
    }
    if let Some(style) = HeadingStyle::all().iter().find(|s| s.marker() == family) {
        return Para::Heading(*style, level);
    }
    if family == "li" {
        return Para::ListItem(level);
    }
    // `\ms` is a major section heading; the family table has `s`, and a major
    // section is a section for typesetting purposes at a larger size.
    if family == "ms" || family == "mte" {
        return Para::Heading(HeadingStyle::S, level);
    }
    if let Some(style) = ParaStyle::all().iter().find(|s| s.marker() == family) {
        return Para::Paragraph(*style);
    }
    Para::Unknown
}

fn char_style(marker: &str) -> Option<CharStyle> {
    CharStyle::all()
        .iter()
        .find(|s| s.marker() == marker)
        .copied()
}

/// `1`, `1-2`, `3a`.
fn parse_verse(raw: &str) -> Option<VerseId> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let (range, segment) = match raw.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&raw[..raw.len() - 1], Some(c)),
        _ => (raw, None),
    };

    let (start, end) = match range.split_once(['-', '\u{2013}']) {
        Some((a, b)) => (a.trim().parse().ok()?, b.trim().parse().ok()?),
        None => {
            let n = range.trim().parse().ok()?;
            (n, n)
        }
    };

    Some(VerseId {
        start,
        end,
        segment,
    })
}
