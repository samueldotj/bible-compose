//! The XML emitter — the Rust half of ADR-002's contract.
//!
//! Scripture becomes an XML **text node**, never command syntax. S0.6 measured
//! why that distinction is load-bearing: the same characters templated into
//! SIL syntax produced a build that reported zero errors, exited zero, and
//! silently tore a verse into three pieces (spike/NOTES.md F-13). A missed
//! escape there does not crash — it succeeds.
//!
//! Two rules keep the output byte-reproducible (SILE-005, DET-001):
//!
//! * **No `HashMap` anywhere on this path.** Rust randomises its iteration
//!   order per process, so one would make golden tests fail on one machine in
//!   three. Attributes are ordered `Vec`s and lookups are `BTreeMap`.
//! * **Fixed attribute order, `\n` line endings, no insignificant whitespace.**
//!   Written explicitly rather than relying on a serializer's defaults.

use std::io::Cursor;

use biblecompose_scripture::{
    Align, Attribute, Block, Book, Cell, CrossReference, FigureRef, Inline, Milestone, Note,
    NoteKind, Row, ScriptureDocument, Unsupported,
};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

/// The contract version written on the root element (SILE-009).
///
/// The class refuses a version it does not know. S0.6 established that an
/// unknown element is a hard error in SILE's XML reader, which is what makes
/// this enforceable rather than advisory.
pub const CONTRACT_VERSION: &str = "1";

/// Where the emitted line came from, so a backend error deep in a book can be
/// reported as a Scripture reference rather than an XML line number (SILE-007).
///
/// Built during emission because it cannot be reconstructed afterwards.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineMap {
    entries: Vec<LineRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRef {
    pub line: u32,
    pub book: String,
    pub chapter: u16,
    pub verse: Option<u16>,
}

impl LineMap {
    /// The nearest reference at or before `line`.
    pub fn resolve(&self, line: u32) -> Option<&LineRef> {
        self.entries.iter().rev().find(|e| e.line <= line)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The emitted document plus everything derived during emission.
#[derive(Debug, Clone)]
pub struct Emitted {
    pub xml: String,
    pub line_map: LineMap,
    /// Markers carried through the model but not representable in the
    /// contract. Reported rather than dropped silently (FUN-003).
    pub dropped: Vec<Unsupported>,
}

/// Emit a document as backend input.
///
/// Takes only the model. There is no provenance parameter and no way to pass
/// one: [ADR-005] requires that origin information cannot reach the emitter,
/// because a file path in the output is a file path in a golden file.
///
/// [ADR-005]: ../../../docs/adr/005-provenance.md
pub fn emit(doc: &ScriptureDocument, styles: &[StyleRule]) -> Emitted {
    emit_hiding(doc, styles, Hidden::nothing())
}

/// The same, with parts of each book withheld (see [`Hidden`]).
pub fn emit_hiding(doc: &ScriptureDocument, styles: &[StyleRule], hidden: Hidden) -> Emitted {
    let mut w = Writer::new(Cursor::new(Vec::new()));
    let mut state = EmitState {
        hidden,
        ..EmitState::default()
    };

    let mut root = BytesStart::new("biblecompose");
    root.push_attribute(("version", CONTRACT_VERSION));
    root.push_attribute(("class", "biblecompose"));
    write(&mut w, Event::Start(root), &mut state);

    emit_styles(&mut w, styles, &mut state);

    for book in &doc.books {
        emit_book(&mut w, book, &mut state);
    }

    write(
        &mut w,
        Event::End(BytesEnd::new("biblecompose")),
        &mut state,
    );
    // A trailing newline, so the file ends the way every other text file does.
    write_raw(&mut w, "\n", &mut state);

    let bytes = w.into_inner().into_inner();
    let xml = String::from_utf8(bytes).expect("the writer only ever emits UTF-8");

    Emitted {
        xml,
        line_map: state.line_map,
        dropped: state.dropped,
    }
}

/// One selector's resolved appearance, as the backend takes it.
///
/// Plain strings, already in the units SILE reads, already in the order they
/// are to be written. Declared here rather than taken from
/// `biblecompose-config` because this crate must not know a configuration
/// layer exists (ARCHITECTURE §2) — and because [ADR-005] requires that the
/// emitter cannot see provenance, which a type with nowhere to put it
/// guarantees rather than promises.
///
/// [ADR-005]: ../../../docs/adr/005-provenance.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRule {
    /// The selector's key — `chapter`, `poetry.q1`, `character.bd`.
    pub selector: String,
    /// Property name and value, in a fixed order. A `Vec` and not a map:
    /// attribute order is part of what makes the output byte-reproducible
    /// (SILE-005), and a `HashMap` would shuffle it per process.
    pub properties: Vec<(String, String)>,
}

/// The resolved styles, as data at the head of the document.
///
/// ADR-002's rule is that Scripture is a text node and never syntax; the same
/// reasoning applies to style values, which arrive as attributes and never as
/// command fragments. A style that could carry `\command` would be a style
/// file that can run code, and style files travel with projects.
///
/// Only selectors with something to say are written. The class treats an
/// absent one as "no styling", which is what the built-in sheet means by an
/// empty entry.
fn emit_styles(w: &mut Writer<Cursor<Vec<u8>>>, styles: &[StyleRule], state: &mut EmitState) {
    if styles.is_empty() {
        return;
    }

    newline(w, state);
    write(w, Event::Start(BytesStart::new("styles")), state);
    for rule in styles {
        newline(w, state);
        let mut el = BytesStart::new("style");
        el.push_attribute(("for", rule.selector.as_str()));
        for (name, value) in &rule.properties {
            el.push_attribute((name.as_str(), value.as_str()));
        }
        write(w, Event::Empty(el), state);
    }
    newline(w, state);
    write(w, Event::End(BytesEnd::new("styles")), state);
}

#[derive(Default)]
struct EmitState {
    line: u32,
    line_map: LineMap,
    dropped: Vec<Unsupported>,
    book: String,
    chapter: u16,
    /// Parts of each book this project does not print.
    hidden: Hidden,
}

impl EmitState {
    fn record(&mut self, verse: Option<u16>) {
        if self.book.is_empty() {
            return;
        }
        self.line_map.entries.push(LineRef {
            line: self.line,
            book: self.book.clone(),
            chapter: self.chapter,
            verse,
        });
    }
}

fn write(w: &mut Writer<Cursor<Vec<u8>>>, ev: Event<'_>, state: &mut EmitState) {
    w.write_event(ev)
        .expect("writing to an in-memory cursor cannot fail");
    let _ = state;
}

/// Newlines are written explicitly rather than by the writer's indent mode:
/// the layout of the file is part of what golden tests compare, so it is
/// decided here and not by a dependency's defaults.
fn newline(w: &mut Writer<Cursor<Vec<u8>>>, state: &mut EmitState) {
    write_raw(w, "\n", state);
    state.line += 1;
}

fn write_raw(w: &mut Writer<Cursor<Vec<u8>>>, s: &str, state: &mut EmitState) {
    use std::io::Write;
    w.get_mut()
        .write_all(s.as_bytes())
        .expect("writing to an in-memory cursor cannot fail");
    let _ = state;
}

fn emit_book(w: &mut Writer<Cursor<Vec<u8>>>, book: &Book, state: &mut EmitState) {
    state.book = book
        .names
        .for_running_head()
        .unwrap_or_else(|| book.code.english_name())
        .to_owned();
    state.chapter = 0;

    newline(w, state);
    let mut el = BytesStart::new("book");
    el.push_attribute(("code", book.code.as_str()));
    if let Some(name) = book.names.for_running_head() {
        el.push_attribute(("name", name));
    }
    // The fuller form, for a running head configured to want it. Emitted even
    // when it is the same string: which name a head shows is the class's
    // decision to make, and a missing attribute would make it the emitter's.
    if let Some(name) = book.names.for_alternate_head() {
        el.push_attribute(("altname", name));
    }
    write(w, Event::Start(el), state);

    for block in &book.blocks {
        emit_block(w, block, state);
    }

    newline(w, state);
    write(w, Event::End(BytesEnd::new("book")), state);
}

/// Parts of a book a project has chosen not to print.
///
/// # Why this is emission and not the class
///
/// [ADR-002] puts the division at "the document says what, the class says
/// how", and every other thing that can be turned off — verse numbers,
/// footnotes, the running head — is hidden by the class from a document that
/// still carries it. These three are not, and the reason is measured rather
/// than aesthetic: a class that returns without typesetting a section heading
/// leaves two balanced columns that SILE cannot resolve, and it does not fail
/// — it spins. Three ways of writing that hide were tried (a bare return, one
/// that closes the paragraph, one that keeps the break penalties) and all
/// three hang; the same document with the headings absent from the XML sets
/// in seconds.
///
/// So the choice is between a setting that hangs the backend and one that
/// costs a re-emission, and re-emission costs nothing here: every build emits
/// from the model anyway.
///
/// [ADR-002]: ../../../docs/adr/002-sile-interface.md
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hidden {
    pub book_introductions: bool,
    pub introductory_outlines: bool,
    pub section_headings: bool,
}

impl Hidden {
    /// Everything printed, which is what a golden file wants.
    pub fn nothing() -> Hidden {
        Hidden::default()
    }

    /// Whether a paragraph marker is one of the parts being withheld.
    fn hides_para(&self, marker: &str) -> bool {
        const INTRO: [&str; 9] = ["ip", "ipi", "im", "imi", "ipq", "imq", "ipr", "iex", "ie"];
        const OUTLINE: [&str; 6] = ["io1", "io2", "io3", "io4", "ili1", "ili2"];
        (self.book_introductions && INTRO.contains(&marker))
            || (self.introductory_outlines && OUTLINE.contains(&marker))
    }

    /// And a heading marker. `\s` is the section heading; the parallel
    /// reference line, a psalm's superscription and a speaker are in the same
    /// family and are not what that setting is about.
    fn hides_heading(&self, marker: &str) -> bool {
        (self.book_introductions && matches!(marker, "is" | "imt"))
            || (self.introductory_outlines && marker == "iot")
            || (self.section_headings && marker == "s")
    }
}

fn emit_block(w: &mut Writer<Cursor<Vec<u8>>>, block: &Block, state: &mut EmitState) {
    newline(w, state);
    match block {
        Block::Paragraph { style, content } => {
            if state.hidden.hides_para(style.marker()) {
                emit_anchors(w, content, state);
                return;
            }
            let mut el = BytesStart::new("para");
            el.push_attribute(("style", style.marker()));
            write(w, Event::Start(el), state);
            emit_inlines(w, content, state);
            write(w, Event::End(BytesEnd::new("para")), state);
        }
        Block::Poetry {
            style,
            level,
            content,
        } => {
            let mut el = BytesStart::new("poetry");
            el.push_attribute(("style", style.marker()));
            el.push_attribute(("level", level.to_string().as_str()));
            write(w, Event::Start(el), state);
            emit_inlines(w, content, state);
            write(w, Event::End(BytesEnd::new("poetry")), state);
        }
        Block::Heading {
            style,
            level,
            content,
        } => {
            if state.hidden.hides_heading(style.marker()) {
                emit_anchors(w, content, state);
                return;
            }
            let mut el = BytesStart::new("heading");
            el.push_attribute(("style", style.marker()));
            el.push_attribute(("level", level.to_string().as_str()));
            write(w, Event::Start(el), state);
            emit_inlines(w, content, state);
            write(w, Event::End(BytesEnd::new("heading")), state);
        }
        Block::ListItem { level, content } => {
            let mut el = BytesStart::new("item");
            el.push_attribute(("level", level.to_string().as_str()));
            write(w, Event::Start(el), state);
            emit_inlines(w, content, state);
            write(w, Event::End(BytesEnd::new("item")), state);
        }
        Block::Table { rows } => {
            write(w, Event::Start(BytesStart::new("table")), state);
            for row in rows {
                emit_row(w, row, state);
            }
            newline(w, state);
            write(w, Event::End(BytesEnd::new("table")), state);
        }
        Block::Figure(f) => emit_figure(w, f, state),
        Block::Break => {
            write(w, Event::Empty(BytesStart::new("break")), state);
        }
    }
}

fn emit_row(w: &mut Writer<Cursor<Vec<u8>>>, row: &Row, state: &mut EmitState) {
    newline(w, state);
    let mut el = BytesStart::new("row");
    el.push_attribute(("header", if row.header { "true" } else { "false" }));
    write(w, Event::Start(el), state);
    for cell in &row.cells {
        emit_cell(w, cell, state);
    }
    write(w, Event::End(BytesEnd::new("row")), state);
}

fn emit_cell(w: &mut Writer<Cursor<Vec<u8>>>, cell: &Cell, state: &mut EmitState) {
    let mut el = BytesStart::new("cell");
    el.push_attribute((
        "align",
        match cell.align {
            Align::Start => "start",
            Align::End => "end",
        },
    ));
    write(w, Event::Start(el), state);
    emit_inlines(w, &cell.content, state);
    write(w, Event::End(BytesEnd::new("cell")), state);
}

fn emit_figure(w: &mut Writer<Cursor<Vec<u8>>>, f: &FigureRef, state: &mut EmitState) {
    let mut el = BytesStart::new("figure");
    el.push_attribute(("src", f.src.as_str()));
    if let Some(size) = &f.size {
        el.push_attribute(("size", size.as_str()));
    }
    if let Some(alt) = &f.alt {
        el.push_attribute(("alt", alt.as_str()));
    }
    // Preserved in declaration order, not sorted, because USFM attribute order
    // is the author's and re-ordering it would be a silent edit (USFM-003).
    for Attribute { key, value } in &f.attributes {
        el.push_attribute((key.as_str(), value.as_str()));
    }
    if let Some(caption) = &f.caption {
        write(w, Event::Start(el), state);
        text_node(w, caption, state);
        write(w, Event::End(BytesEnd::new("figure")), state);
    } else {
        write(w, Event::Empty(el), state);
    }
}

fn emit_inlines(w: &mut Writer<Cursor<Vec<u8>>>, items: &[Inline], state: &mut EmitState) {
    for item in items {
        emit_inline(w, item, state);
    }
}

/// The anchors from a block that is not being printed.
///
/// Normalization tucks a chapter marker inside the heading it falls before —
/// `<heading style="s"><chapter n="1"/>The Beginning</heading>` — so dropping
/// the heading whole would drop chapter 1 with it. The book would then be set
/// with no chapter recorded anywhere, and a running head asking for its
/// reference range would be asking about a page that does not know where it
/// is. Measured consequence: the backend does not fail, it spins.
///
/// So a hidden block still gives up its chapter and verse anchors. They set
/// no type; they are what tells the page where it is.
fn emit_anchors(w: &mut Writer<Cursor<Vec<u8>>>, items: &[Inline], state: &mut EmitState) {
    for item in items {
        match item {
            Inline::Chapter { .. } | Inline::Verse { .. } => emit_inline(w, item, state),
            _ => {}
        }
    }
}

fn emit_inline(w: &mut Writer<Cursor<Vec<u8>>>, item: &Inline, state: &mut EmitState) {
    match item {
        Inline::Text(t) => text_node(w, t, state),

        Inline::Chapter {
            number,
            published,
            alternate,
        } => {
            state.chapter = *number;
            state.record(None);
            let mut el = BytesStart::new("chapter");
            // `n` is a string on the wire and a string in the model's own
            // hands before it gets here: spike F-9 found that anything SILE
            // later stringifies must already BE a string, or a running head
            // renders "table: 0x55f…".
            el.push_attribute(("n", number.to_string().as_str()));
            if let Some(p) = published {
                el.push_attribute(("pub", p.as_str()));
            }
            if let Some(a) = alternate {
                el.push_attribute(("alt", a.to_string().as_str()));
            }
            write(w, Event::Empty(el), state);
        }

        Inline::Verse {
            id,
            published,
            alternate,
        } => {
            state.record(Some(id.start));
            let mut el = BytesStart::new("verse");
            el.push_attribute(("n", id.to_string().as_str()));
            el.push_attribute(("start", id.start.to_string().as_str()));
            el.push_attribute(("end", id.end.to_string().as_str()));
            if let Some(p) = published {
                el.push_attribute(("pub", p.as_str()));
            }
            if let Some(a) = alternate {
                el.push_attribute(("alt", a.to_string().as_str()));
            }
            write(w, Event::Empty(el), state);
        }

        Inline::Char { style, content } => {
            let mut el = BytesStart::new("char");
            el.push_attribute(("style", style.marker()));
            write(w, Event::Start(el), state);
            emit_inlines(w, content, state);
            write(w, Event::End(BytesEnd::new("char")), state);
        }

        Inline::Note(note) => emit_note(w, note, state),
        Inline::Ref(r) => emit_ref(w, r, state),
        Inline::Milestone(m) => emit_milestone(w, m, state),

        Inline::Unsupported(u) => {
            // Carried into the output as inert text rather than dropped, and
            // recorded so the build log can say what was not rendered.
            state.dropped.push(u.clone());
            let mut el = BytesStart::new("unsupported");
            el.push_attribute(("marker", u.marker.as_str()));
            write(w, Event::Start(el), state);
            text_node(w, &u.text, state);
            write(w, Event::End(BytesEnd::new("unsupported")), state);
        }
    }
}

fn emit_note(w: &mut Writer<Cursor<Vec<u8>>>, note: &Note, state: &mut EmitState) {
    let mut el = BytesStart::new("note");
    el.push_attribute((
        "style",
        match note.kind {
            NoteKind::Footnote => "f",
            NoteKind::Endnote => "fe",
        },
    ));
    el.push_attribute(("caller", note.caller.as_str()));
    if let Some(o) = &note.origin {
        el.push_attribute(("origin", o.as_str()));
    }
    write(w, Event::Start(el), state);
    for block in &note.content {
        emit_block(w, block, state);
    }
    write(w, Event::End(BytesEnd::new("note")), state);
}

fn emit_ref(w: &mut Writer<Cursor<Vec<u8>>>, r: &CrossReference, state: &mut EmitState) {
    let mut el = BytesStart::new("xref");
    el.push_attribute(("caller", r.caller.as_str()));
    if let Some(o) = &r.origin {
        el.push_attribute(("origin", o.as_str()));
    }
    write(w, Event::Start(el), state);
    emit_inlines(w, &r.content, state);
    write(w, Event::End(BytesEnd::new("xref")), state);
}

fn emit_milestone(w: &mut Writer<Cursor<Vec<u8>>>, m: &Milestone, state: &mut EmitState) {
    let mut el = BytesStart::new("milestone");
    el.push_attribute(("marker", m.marker.as_str()));
    el.push_attribute(("start", if m.start { "true" } else { "false" }));
    for Attribute { key, value } in &m.attributes {
        el.push_attribute((key.as_str(), value.as_str()));
    }
    write(w, Event::Empty(el), state);
}

/// The whole security argument, in one function.
///
/// `BytesText::new` escapes on write, so a backslash stays a backslash and a
/// brace stays a brace. There is no escaping logic of ours to get wrong.
fn text_node(w: &mut Writer<Cursor<Vec<u8>>>, s: &str, state: &mut EmitState) {
    if s.is_empty() {
        return;
    }
    write(w, Event::Text(BytesText::new(s)), state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use biblecompose_scripture::fixtures;

    #[test]
    fn root_carries_the_contract_version() {
        let out = emit(&fixtures::john_1_1_5(), &[]);
        assert!(out
            .xml
            .starts_with("<biblecompose version=\"1\" class=\"biblecompose\">"));
        assert!(out.xml.ends_with("</biblecompose>\n"));
    }

    /// ADR-002, and the reason for the whole decision. Every one of these is a
    /// character that means something in SIL syntax.
    #[test]
    fn sil_syntax_in_scripture_becomes_inert_text() {
        let out = emit(&fixtures::adversarial(), &[]);

        // A backslash survives as a backslash — it is not a command.
        assert!(out.xml.contains(r"Backslash \bd and \par"));
        assert!(out.xml.contains(r"\skip[height=40pt]"));
        // Braces and percent are not special in XML and must pass through.
        assert!(out.xml.contains("{like this}"));
        assert!(out.xml.contains("100%"));
        // The two that ARE special in XML must be escaped, by the serializer.
        assert!(out.xml.contains("&amp;"));
        assert!(out.xml.contains("&lt;angle&gt;"));
        assert!(!out.xml.contains("<angle>"));
    }

    /// The corollary: nothing in Scripture can close an element and open a new
    /// one, which is the XML equivalent of command injection.
    #[test]
    fn scripture_cannot_forge_markup() {
        use biblecompose_scripture::{
            Block, Book, BookNames, Inline, ParaStyle, ScriptureDocument,
        };
        let hostile = "</para><para style=\"p\">forged";
        let doc = ScriptureDocument::new(vec![Book::new(
            biblecompose_scripture::BookCode::parse("MAT").unwrap(),
            BookNames::named("Matthew"),
            vec![Block::Paragraph {
                style: ParaStyle::P,
                content: vec![Inline::Text(hostile.to_owned())],
            }],
        )]);
        let out = emit(&doc, &[]);
        assert_eq!(
            out.xml.matches("<para").count(),
            1,
            "hostile text must not open a second paragraph"
        );
        assert!(out.xml.contains("&lt;/para&gt;"));
    }

    #[test]
    fn emission_is_byte_identical_across_runs() {
        for (name, doc) in fixtures::all() {
            let first = emit(&doc, &[]).xml;
            for _ in 0..32 {
                assert_eq!(
                    first,
                    emit(&doc, &[]).xml,
                    "fixture {name} is not deterministic"
                );
            }
        }
    }

    #[test]
    fn line_endings_are_lf_only() {
        for (name, doc) in fixtures::all() {
            let xml = emit(&doc, &[]).xml;
            assert!(!xml.contains('\r'), "fixture {name} emitted a CR");
        }
    }

    /// F-9: values SILE will later stringify must already be strings. Numbers
    /// are formatted here, at the boundary, not left to the class.
    #[test]
    fn numeric_attributes_are_written_as_strings() {
        let out = emit(&fixtures::john_1_1_5(), &[]);
        assert!(out.xml.contains(r#"<chapter n="1"/>"#));
        assert!(out.xml.contains(r#"<verse n="1" start="1" end="1"/>"#));
    }

    #[test]
    fn line_map_resolves_a_backend_line_to_a_reference() {
        let out = emit(&fixtures::john_1_1_5(), &[]);
        assert!(!out.line_map.is_empty());
        let last = out
            .line_map
            .resolve(u32::MAX)
            .expect("a reference for any line");
        assert_eq!(last.book, "John");
        assert_eq!(last.chapter, 1);
        assert_eq!(last.verse, Some(5));
    }

    #[test]
    fn unsupported_markers_are_reported_not_silently_dropped() {
        let out = emit(&fixtures::kitchen_sink(), &[]);
        assert!(out.dropped.iter().any(|u| u.marker == "zmystery"));
        assert!(out.xml.contains(r#"<unsupported marker="zmystery">"#));
    }

    #[test]
    fn every_block_and_inline_variant_emits() {
        let out = emit(&fixtures::kitchen_sink(), &[]);
        for expected in [
            "<para",
            "<poetry",
            "<heading",
            "<item",
            "<table",
            "<row",
            "<cell",
            "<figure",
            "<break/>",
            "<chapter",
            "<verse",
            "<char",
            "<note",
            "<xref",
            "<milestone",
            "<unsupported",
        ] {
            assert!(
                out.xml.contains(expected),
                "kitchen_sink did not emit {expected}"
            );
        }
    }

    /// Both names, because a running head slot can ask for either.
    #[test]
    fn book_names_are_carried_for_the_running_head() {
        let out = emit(&fixtures::two_books(), &[]);
        assert!(
            out.xml.contains(r#"<book code="GEN" name="Genesis""#),
            "{}",
            out.xml
        );
        assert!(
            out.xml.contains(r#"<book code="JHN" name="John""#),
            "{}",
            out.xml
        );
        // A book that gives one name gives the same string to both slots, which
        // is right: the alternative is a head that empties when a project has
        // no `\toc1`.
        assert_eq!(out.xml.matches("altname=").count(), 2, "{}", out.xml);
        // Canonical order, not insertion order.
        let gen = out.xml.find("GEN").unwrap();
        let jhn = out.xml.find("JHN").unwrap();
        assert!(gen < jhn);
    }
}
