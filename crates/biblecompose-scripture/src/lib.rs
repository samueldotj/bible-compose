//! The normalized Scripture document model — *what the publication is*.
//!
//! This is the second of the three models in ARCHITECTURE §5. It sits above
//! the source-faithful USJ tree that `usfm-core` will produce at M1, and below
//! emission. At M0 it is built by hand from fixtures, which is deliberate: it
//! proves the second half of the pipeline before the first half exists.
//!
//! Three properties are load-bearing and each is asserted by a test below.
//!
//! * **Chapter and verse are inline anchors, not containers.** SCR-001
//!   requires that hiding a number must not lose the reference. As anchors,
//!   visibility is a style question and the anchor is always present — for
//!   running heads, for PDF destinations, and for diagnostics that name a
//!   reference. It also matches the reality that a paragraph legitimately
//!   spans verses and a verse legitimately spans paragraphs.
//! * **Cross-references are their own type**, not notes with a flag (SCR-004).
//! * **`Unsupported` is a variant, not an omission** (FUN-003, USFM-004). The
//!   emitter is where the decision to drop something is made, and it is logged.

pub mod canon;
pub mod fixtures;
pub mod normalize;
pub mod plan;
pub mod usfm;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub use canon::{BookCode, Testament};

/// A whole publication, canon-ordered, with book inclusion already applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptureDocument {
    pub books: Vec<Book>,
    /// book → file, so every diagnostic can name a source.
    pub provenance: Vec<BookSource>,
}

impl ScriptureDocument {
    pub fn new(books: Vec<Book>) -> Self {
        ScriptureDocument {
            books,
            provenance: Vec::new(),
        }
    }

    /// Concatenated Scripture text, in document order. The crude assertion
    /// P1.6 will run over the corpus: it catches the only failure that
    /// matters, which is text going missing or moving.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for book in &self.books {
            book.write_text(&mut out);
        }
        out
    }

    /// Every unsupported marker carried through, with where it came from.
    pub fn unsupported(&self) -> Vec<&Unsupported> {
        let mut found = Vec::new();
        for book in &self.books {
            for block in &book.blocks {
                block.collect_unsupported(&mut found);
            }
        }
        found
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSource {
    pub code: BookCode,
    pub path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub code: BookCode,
    pub names: BookNames,
    pub blocks: Vec<Block>,
}

impl Book {
    pub fn new(code: BookCode, names: BookNames, blocks: Vec<Block>) -> Self {
        Book {
            code,
            names,
            blocks,
        }
    }

    /// How many chapters this book contains.
    ///
    /// Counted rather than stored, because SCR-001 makes a chapter an anchor
    /// inside a paragraph and not a container — there is no list of chapters
    /// to take a length of, and a second field holding the number would be a
    /// field that can disagree with the content.
    ///
    /// Distinct numbers, not occurrences: a chapter marker repeated by mistake
    /// is a parser diagnostic, not two chapters.
    pub fn chapter_count(&self) -> usize {
        let mut seen = std::collections::BTreeSet::new();
        for block in &self.blocks {
            block.each_inline(&mut |inline| {
                if let Inline::Chapter { number, .. } = inline {
                    seen.insert(*number);
                }
            });
        }
        seen.len()
    }

    fn write_text(&self, out: &mut String) {
        for block in &self.blocks {
            block.write_text(out);
        }
    }
}

/// `\h`, `\toc1`–`\toc3`, `\mt1`–`\mt4`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookNames {
    /// `\h` — the running-head name.
    pub running: Option<String>,
    /// `\toc1` — long name.
    pub long: Option<String>,
    /// `\toc2` — short name.
    pub short: Option<String>,
    /// `\toc3` — abbreviation.
    pub abbrev: Option<String>,
    /// `\mt1`–`\mt4`, in order.
    pub title: Vec<String>,
}

impl BookNames {
    pub fn named(running: &str) -> Self {
        BookNames {
            running: Some(running.to_owned()),
            long: Some(running.to_owned()),
            short: Some(running.to_owned()),
            abbrev: None,
            title: vec![running.to_owned()],
        }
    }

    /// What a running head should show, falling back sensibly.
    pub fn for_running_head(&self) -> Option<&str> {
        self.running
            .as_deref()
            .or(self.short.as_deref())
            .or(self.long.as_deref())
            .or_else(|| self.title.first().map(String::as_str))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Block {
    Paragraph {
        style: ParaStyle,
        content: Vec<Inline>,
    },
    Poetry {
        style: PoetryStyle,
        level: u8,
        content: Vec<Inline>,
    },
    Heading {
        style: HeadingStyle,
        level: u8,
        content: Vec<Inline>,
    },
    ListItem {
        level: u8,
        content: Vec<Inline>,
    },
    Table {
        rows: Vec<Row>,
    },
    Figure(FigureRef),
    /// `\b` — a deliberate blank line between poetry stanzas.
    Break,
}

impl Block {
    /// Every inline in this block, including the ones nested inside character
    /// spans, notes and cross-references.
    ///
    /// The third walker over this shape, and the first one general enough to
    /// share. `write_text` and `collect_unsupported` predate it and each want
    /// something slightly different from the traversal; this exists because
    /// counting chapters wanted a third slightly different thing, and three
    /// hand-written descents is where a missed variant starts costing.
    pub fn each_inline(&self, f: &mut impl FnMut(&Inline)) {
        match self {
            Block::Paragraph { content, .. }
            | Block::Poetry { content, .. }
            | Block::Heading { content, .. }
            | Block::ListItem { content, .. } => each_inline(content, f),
            Block::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        each_inline(&cell.content, f);
                    }
                }
            }
            Block::Figure(_) | Block::Break => {}
        }
    }

    fn write_text(&self, out: &mut String) {
        match self {
            Block::Paragraph { content, .. }
            | Block::Poetry { content, .. }
            | Block::Heading { content, .. }
            | Block::ListItem { content, .. } => write_inlines(content, out),
            Block::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        write_inlines(&cell.content, out);
                    }
                }
            }
            Block::Figure(f) => {
                if let Some(caption) = &f.caption {
                    out.push_str(caption);
                }
            }
            Block::Break => {}
        }
    }

    fn collect_unsupported<'a>(&'a self, found: &mut Vec<&'a Unsupported>) {
        match self {
            Block::Paragraph { content, .. }
            | Block::Poetry { content, .. }
            | Block::Heading { content, .. }
            | Block::ListItem { content, .. } => collect_unsupported(content, found),
            Block::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_unsupported(&cell.content, found);
                    }
                }
            }
            Block::Figure(_) | Block::Break => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub header: bool,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub align: Align,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inline {
    Text(String),
    /// An anchor, not a container. Always present even when the number is
    /// configured hidden (SCR-001).
    Chapter {
        number: u16,
        published: Option<String>,
        alternate: Option<u16>,
    },
    Verse {
        id: VerseId,
        published: Option<String>,
        alternate: Option<VerseId>,
    },
    Char {
        style: CharStyle,
        content: Vec<Inline>,
    },
    Note(Note),
    /// `\x` — a distinct type from `Note`, per SCR-004.
    Ref(CrossReference),
    Milestone(Milestone),
    /// Diagnosed, never silently dropped (FUN-003).
    Unsupported(Unsupported),
}

/// Depth-first, parents before children, so a caller can stop at whatever
/// depth it cares about by ignoring the rest.
fn each_inline(items: &[Inline], f: &mut impl FnMut(&Inline)) {
    for item in items {
        f(item);
        match item {
            Inline::Char { content, .. } => each_inline(content, f),
            Inline::Ref(r) => each_inline(&r.content, f),
            Inline::Note(n) => {
                for block in &n.content {
                    block.each_inline(f);
                }
            }
            Inline::Text(_) | Inline::Chapter { .. } | Inline::Verse { .. } => {}
            Inline::Milestone(_) | Inline::Unsupported(_) => {}
        }
    }
}

fn write_inlines(items: &[Inline], out: &mut String) {
    for item in items {
        match item {
            Inline::Text(t) => out.push_str(t),
            Inline::Char { content, .. } => write_inlines(content, out),
            // Notes and cross-references are apparatus, not Scripture text;
            // P1.6's comparison is against the running text of the book.
            Inline::Note(_)
            | Inline::Ref(_)
            | Inline::Chapter { .. }
            | Inline::Verse { .. }
            | Inline::Milestone(_)
            | Inline::Unsupported(_) => {}
        }
    }
}

fn collect_unsupported<'a>(items: &'a [Inline], found: &mut Vec<&'a Unsupported>) {
    for item in items {
        match item {
            Inline::Unsupported(u) => found.push(u),
            Inline::Char { content, .. } => collect_unsupported(content, found),
            Inline::Note(n) => {
                for b in &n.content {
                    b.collect_unsupported(found);
                }
            }
            Inline::Ref(r) => collect_unsupported(&r.content, found),
            _ => {}
        }
    }
}

/// `\v 1`, `\v 1-2`, `\v 1a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerseId {
    pub start: u16,
    pub end: u16,
    pub segment: Option<char>,
}

impl VerseId {
    pub fn single(n: u16) -> Self {
        VerseId {
            start: n,
            end: n,
            segment: None,
        }
    }

    pub fn range(start: u16, end: u16) -> Self {
        VerseId {
            start,
            end,
            segment: None,
        }
    }

    pub fn is_range(&self) -> bool {
        self.end > self.start
    }
}

impl std::fmt::Display for VerseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.start)?;
        if self.is_range() {
            write!(f, "-{}", self.end)?;
        }
        if let Some(s) = self.segment {
            write!(f, "{s}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub kind: NoteKind,
    /// `+` for auto-numbered, `-` for suppressed, or a literal caller.
    pub caller: String,
    /// `\fr` — the origin reference.
    pub origin: Option<String>,
    /// Notes carry block content, not a flat string (USFM-002).
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteKind {
    /// `\f`
    Footnote,
    /// `\fe`
    Endnote,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossReference {
    pub caller: String,
    /// `\xo` — the origin reference.
    pub origin: Option<String>,
    /// `\xt`, `\xk`, `\xq` and friends, structurally.
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigureRef {
    /// `src`, relative to the project.
    pub src: Utf8PathBuf,
    pub alt: Option<String>,
    pub caption: Option<String>,
    /// `size` — `col` or `span`.
    pub size: Option<String>,
    /// `loc`, `copy`, `ref` and any other attributes, preserved (USFM-003).
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub marker: String,
    pub start: bool,
    pub attributes: Vec<Attribute>,
}

/// A marker the release does not support, carried through with its location so
/// support can be added later without touching the parser boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unsupported {
    pub marker: String,
    pub text: String,
    pub location: Option<biblecompose_diagnostics::SourceLoc>,
}

macro_rules! styles {
    ($(
        $(#[$m:meta])* $name:ident {
            $( $(#[$vm:meta])* $variant:ident => $s:literal ),* $(,)?
        }
    )*) => {
        $(
            $(#[$m])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
            pub enum $name { $( $(#[$vm])* $variant ),* }

            impl $name {
                /// The USFM marker this style corresponds to. Also the string
                /// the emitter writes, so a style and its wire form cannot
                /// drift apart.
                pub const fn marker(self) -> &'static str {
                    match self { $( $name::$variant => $s ),* }
                }

                pub const fn all() -> &'static [$name] {
                    &[ $( $name::$variant ),* ]
                }
            }

            impl ::std::fmt::Display for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    f.write_str(self.marker())
                }
            }
        )*
    };
}

styles! {
    /// Paragraph-level markers.
    ParaStyle {
        P => "p", M => "m", Po => "po", Pr => "pr", Cls => "cls",
        Pmo => "pmo", Pm => "pm", Pmc => "pmc", Pmr => "pmr",
        Pi1 => "pi1", Pi2 => "pi2", Pi3 => "pi3",
        Mi => "mi", Nb => "nb", Pc => "pc",

        // Introduction matter. Present because a corpus run found it in a
        // fifth of real books: leaving it unsupported produced hundreds of
        // warnings per file and told a publisher nothing they could act on.
        // Whether an introduction is *printed* is a style question (M3);
        // whether it survives normalization is not.
        Ip => "ip", Ipi => "ipi", Im => "im", Imi => "imi",
        Ipq => "ipq", Imq => "imq", Ipr => "ipr", Iex => "iex",
        Io1 => "io1", Io2 => "io2", Io3 => "io3", Io4 => "io4",
        Ili1 => "ili1", Ili2 => "ili2",
        Ie => "ie",
        /// `\\cl` — a chapter label, printed in place of or beside the number.
        Cl => "cl",
    }

    /// Poetry. The `level` on `Block::Poetry` carries the digit; this is the
    /// family.
    PoetryStyle {
        Q => "q", Qr => "qr", Qc => "qc", Qa => "qa", Qm => "qm", Qd => "qd",
    }

    /// Section headings and their relatives.
    HeadingStyle {
        S => "s", Sr => "sr", R => "r", D => "d", Sp => "sp",
        /// Introduction headings, for the same reason as the paragraphs.
        Is => "is", Imt => "imt", Iot => "iot",
    }

    /// Character-level markers.
    CharStyle {
        Add => "add", Bd => "bd", Bdit => "bdit", Em => "em", It => "it",
        Nd => "nd", No => "no", Sc => "sc", Sup => "sup", Wj => "wj",
        Qt => "qt", Sig => "sig", Tl => "tl", K => "k", Pn => "pn",
        Ord => "ord", W => "w",

        // Note and cross-reference internals. `\\ft` alone appeared 2,758
        // times in a 200-file corpus run; treating the body of every footnote
        // as an unsupported style would bury the diagnostics panel under
        // warnings about documents that are perfectly ordinary.
        //
        // `\\fr` and `\\xo` are here even though the *first* of each is
        // consumed into `Note::origin` / `CrossReference::origin` before this
        // table is consulted. A cross-reference may carry several origins,
        // and the corpus run found 26 second ones — without a style they
        // would each be reported as unsupported, which is a warning about
        // nothing.
        Fr => "fr", Xo => "xo",
        Ft => "ft", Fq => "fq", Fqa => "fqa", Fk => "fk", Fp => "fp",
        Fv => "fv", Fdc => "fdc", Fl => "fl", Fw => "fw",
        Xt => "xt", Xk => "xk", Xq => "xq",
        Xot => "xot", Xnt => "xnt", Xdc => "xdc",
        /// `\\ior` — an outline reference inside introduction matter.
        Ior => "ior",
        /// `\\iqt` — quoted text inside introduction matter.
        Iqt => "iqt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_round_trip_through_json() {
        for (name, doc) in fixtures::all() {
            let json = serde_json::to_string(&doc).expect("serialize");
            let back: ScriptureDocument = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(doc, back, "fixture {name} did not round-trip");
        }
    }

    /// P0.3's acceptance list, asserted rather than assumed.
    #[test]
    fn fixtures_cover_every_construct() {
        let doc = fixtures::kitchen_sink();
        let mut seen = std::collections::BTreeSet::new();
        for book in &doc.books {
            for block in &book.blocks {
                seen.insert(match block {
                    Block::Paragraph { .. } => "paragraph",
                    Block::Poetry { .. } => "poetry",
                    Block::Heading { .. } => "heading",
                    Block::ListItem { .. } => "list",
                    Block::Table { .. } => "table",
                    Block::Figure(_) => "figure",
                    Block::Break => "break",
                });
                collect_inline_kinds(block, &mut seen);
            }
        }
        for required in [
            "paragraph",
            "poetry",
            "heading",
            "list",
            "table",
            "figure",
            "break",
            "chapter",
            "verse",
            "char",
            "note",
            "crossref",
            "milestone",
            "unsupported",
        ] {
            assert!(
                seen.contains(required),
                "kitchen_sink is missing {required}"
            );
        }
    }

    fn collect_inline_kinds(block: &Block, seen: &mut std::collections::BTreeSet<&'static str>) {
        let content: &[Inline] = match block {
            Block::Paragraph { content, .. }
            | Block::Poetry { content, .. }
            | Block::Heading { content, .. }
            | Block::ListItem { content, .. } => content,
            Block::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        walk(&cell.content, seen);
                    }
                }
                return;
            }
            _ => return,
        };
        walk(content, seen);
    }

    fn walk(items: &[Inline], seen: &mut std::collections::BTreeSet<&'static str>) {
        for i in items {
            match i {
                Inline::Text(_) => {}
                Inline::Chapter { .. } => {
                    seen.insert("chapter");
                }
                Inline::Verse { .. } => {
                    seen.insert("verse");
                }
                Inline::Char { content, .. } => {
                    seen.insert("char");
                    walk(content, seen);
                }
                Inline::Note(n) => {
                    seen.insert("note");
                    for b in &n.content {
                        collect_inline_kinds(b, seen);
                    }
                }
                Inline::Ref(r) => {
                    seen.insert("crossref");
                    walk(&r.content, seen);
                }
                Inline::Milestone(_) => {
                    seen.insert("milestone");
                }
                Inline::Unsupported(_) => {
                    seen.insert("unsupported");
                }
            }
        }
    }

    /// SCR-001: the anchor survives independently of whether the number shows.
    #[test]
    fn verse_anchors_are_inline_not_containers() {
        let doc = fixtures::john_1_1_5();
        let book = &doc.books[0];
        // One paragraph holds several verse anchors — which is only possible
        // because verses are not containers.
        let Block::Paragraph { content, .. } = &book.blocks[1] else {
            panic!("expected a paragraph");
        };
        let verses = content
            .iter()
            .filter(|i| matches!(i, Inline::Verse { .. }))
            .count();
        assert!(
            verses > 1,
            "a paragraph should be able to hold several verses"
        );
    }

    #[test]
    fn unsupported_markers_are_reachable_not_dropped() {
        let doc = fixtures::kitchen_sink();
        let found = doc.unsupported();
        assert!(!found.is_empty());
        assert!(found.iter().any(|u| u.marker == "zmystery"));
    }

    #[test]
    fn text_extraction_omits_apparatus() {
        let doc = fixtures::john_1_1_5();
        let text = doc.text();
        assert!(text.contains("In the beginning was the Word"));
        // The footnote body is apparatus, not running text.
        assert!(!text.contains("Or comprehended"));
    }

    #[test]
    fn verse_id_display() {
        assert_eq!(VerseId::single(3).to_string(), "3");
        assert_eq!(VerseId::range(1, 2).to_string(), "1-2");
        assert_eq!(
            VerseId {
                start: 1,
                end: 1,
                segment: Some('a')
            }
            .to_string(),
            "1a"
        );
    }

    #[test]
    fn style_markers_are_unique_within_a_family() {
        fn unique<T: Copy + Ord + std::fmt::Debug>(all: &[T], marker: impl Fn(T) -> &'static str) {
            let mut m: Vec<&str> = all.iter().map(|s| marker(*s)).collect();
            m.sort_unstable();
            let before = m.len();
            m.dedup();
            assert_eq!(before, m.len(), "duplicate marker in family");
        }
        unique(ParaStyle::all(), ParaStyle::marker);
        unique(PoetryStyle::all(), PoetryStyle::marker);
        unique(HeadingStyle::all(), HeadingStyle::marker);
        unique(CharStyle::all(), CharStyle::marker);
    }
}
