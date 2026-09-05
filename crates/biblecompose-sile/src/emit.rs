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
    // Here, once, rather than at each of the places a string enters the
    // document. There are several of those and a new one is one line of a
    // future change away; there is exactly one finished document, and a
    // well-formedness guarantee that can be read off one line is worth more
    // than the allocation it costs in the case that needs it.
    let xml = without_forbidden(&xml).into_owned();

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
    /// A chapter has begun and its opening initial has not yet been marked.
    ///
    /// Set by the chapter anchor and consumed by the first paragraph after
    /// it. Not by a heading — a psalm's superscription carries the anchor and
    /// is not where the psalm starts — and not by poetry, where each line is
    /// its own paragraph and an initial spanning three of them would hang
    /// over the next two with nothing making room for it. Those clear it, so
    /// a paragraph halfway through the chapter does not get one.
    awaiting_initial: bool,
    /// Whether the text being emitted is a paragraph's own — the only place an
    /// initial may be marked. A heading's text follows the chapter anchor too,
    /// and is not the chapter's opening.
    in_paragraph: bool,
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
/// The chapter label joined them for a different reason: `\cl` is where the
/// chapter anchor lives. USJ puts the `\c` inside the label's paragraph —
/// `<para style="cl"><chapter n="1"/>Chapter One</para>` — so hiding the
/// paragraph anywhere except here would take the chapter with it, and every
/// running head asking for a reference would go blank. [`emit_anchors`] is
/// what makes that safe, and it is only reachable from this side.
///
/// [ADR-002]: ../../../docs/adr/002-sile-interface.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hidden {
    pub book_introductions: bool,
    pub introductory_outlines: bool,
    pub section_headings: bool,
    /// USFM's `\cl` — the words an edition prints for a chapter, either
    /// before the first `\c` to name every chapter or after one to name that
    /// chapter alone.
    pub chapter_labels: bool,
    /// Figures the backend must not be asked to draw, by their `src`.
    ///
    /// Named one at a time rather than switched as a class, because this is
    /// not a decision about what a publication contains: each of these is a
    /// figure the project does want and whose file is not there yet. The
    /// application has already said so, once each (SCR-006); withholding them
    /// here is what stops the backend dying on the first one.
    pub figures: Vec<camino::Utf8PathBuf>,
}

impl Hidden {
    /// Everything printed, which is what a golden file wants.
    pub fn nothing() -> Hidden {
        Hidden::default()
    }

    /// The same, with named figures withheld.
    pub fn without_figures(mut self, figures: Vec<camino::Utf8PathBuf>) -> Hidden {
        self.figures = figures;
        self
    }

    fn hides_figure(&self, src: &camino::Utf8Path) -> bool {
        self.figures.iter().any(|f| f == src)
    }

    /// Whether a paragraph marker is one of the parts being withheld.
    fn hides_para(&self, marker: &str) -> bool {
        const INTRO: [&str; 9] = ["ip", "ipi", "im", "imi", "ipq", "imq", "ipr", "iex", "ie"];
        const OUTLINE: [&str; 6] = ["io1", "io2", "io3", "io4", "ili1", "ili2"];
        (self.book_introductions && INTRO.contains(&marker))
            || (self.introductory_outlines && OUTLINE.contains(&marker))
            || (self.chapter_labels && marker == "cl")
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
            // A chapter label — `\cl`, "Chapter One" — is a paragraph that
            // carries the anchor the way a heading does, and is no more the
            // chapter's opening than a heading is. It neither takes the
            // initial nor clears it; the text after it does.
            let label = style.marker() == "cl";
            let mut el = BytesStart::new("para");
            el.push_attribute(("style", style.marker()));
            write(w, Event::Start(el), state);
            let was = std::mem::replace(&mut state.in_paragraph, !label);
            emit_inlines(w, content, state);
            state.in_paragraph = was;
            write(w, Event::End(BytesEnd::new("para")), state);
            // Consumed by the text above if there was any; cleared either way,
            // so a later paragraph does not open the chapter twice.
            if !label {
                state.awaiting_initial = false;
            }
        }
        Block::Poetry {
            style,
            level,
            content,
        } => {
            let mut el = BytesStart::new("poetry");
            el.push_attribute(("style", style.marker()));
            el.push_attribute(("level", level.to_string().as_str()));
            // Poetry does not take the initial, and does not leave it for the
            // prose after the stanza either — see `awaiting_initial`.
            state.awaiting_initial = false;
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
        Block::ListItem {
            style,
            level,
            content,
        } => {
            let mut el = BytesStart::new("item");
            el.push_attribute(("style", style.marker()));
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
        Block::Figure(f) => {
            if !state.hidden.hides_figure(&f.src) {
                emit_figure(w, f, state);
            }
        }
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
    el.push_attribute(("span", cell.span.max(1).to_string().as_str()));
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
        Inline::Text(t) if state.awaiting_initial && state.in_paragraph => {
            match initial(t) {
                Some((lead, first, rest)) => {
                    state.awaiting_initial = false;
                    // Whitespace before the initial is dropped rather than
                    // set: the class would discard it at the start of a
                    // paragraph anyway, and inside `<initial>` it would drop
                    // a space three lines tall.
                    let _ = lead;
                    write(w, Event::Start(BytesStart::new("initial")), state);
                    text_node(w, first, state);
                    write(w, Event::End(BytesEnd::new("initial")), state);
                    text_node(w, rest, state);
                }
                // Nothing but space: pass it through and keep waiting.
                None => text_node(w, t, state),
            }
        }
        Inline::Text(t) => text_node(w, t, state),

        Inline::Chapter {
            number,
            published,
            alternate,
        } => {
            state.chapter = *number;
            state.record(None);
            state.awaiting_initial = true;
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
    // A footnote's text is not the chapter's opening, however early it falls.
    // The flag is put aside for the note's own blocks and restored after.
    let awaiting = std::mem::replace(&mut state.awaiting_initial, false);
    emit_note_body(w, note, state);
    state.awaiting_initial = awaiting;
}

fn emit_note_body(w: &mut Writer<Cursor<Vec<u8>>>, note: &Note, state: &mut EmitState) {
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
    // As for a note: a cross-reference set before the first word is not the
    // first word.
    let awaiting = std::mem::replace(&mut state.awaiting_initial, false);
    emit_ref_body(w, r, state);
    state.awaiting_initial = awaiting;
}

fn emit_ref_body(w: &mut Writer<Cursor<Vec<u8>>>, r: &CrossReference, state: &mut EmitState) {
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

/// The opening initial of a run of text: `(leading whitespace, initial, rest)`.
///
/// **The initial is a grapheme cluster, not a `char`.** In Tamil, "தி" is a
/// consonant followed by a vowel sign, two code points that draw as one
/// syllable; a drop cap of the consonant alone would set "த" three lines tall
/// with the vowel sign stranded at body size on the next run, which is not a
/// syllable of anything. Devanagari conjuncts are consonant, virama,
/// consonant, and Unicode has treated those as one cluster since 15.1. So the
/// split follows UAX #29's extended grapheme clusters, and a script's own
/// idea of a letter comes with it.
///
/// An opening quotation mark or bracket is taken *with* the letter: "“In the
/// beginning" drops as “I, since a body-size quote hanging off a three-line
/// initial reads as a mistake either way and the larger one at least reads
/// as deliberate.
///
/// `None` when there is nothing to open with — a run that is only whitespace.
fn initial(text: &str) -> Option<(&str, &str, &str)> {
    use unicode_segmentation::UnicodeSegmentation;

    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let lead = &text[..text.len() - trimmed.len()];

    // Clusters that are only punctuation an opening leans on. Unicode's
    // *Ps* and *Pi* categories in the forms Scripture actually uses, spelled
    // rather than looked up so the rule is readable here.
    const OPENERS: &[&str] = &["“", "‘", "\"", "'", "(", "[", "«", "‹", "¿", "¡", "„", "‚"];

    let mut end = 0;
    let mut took_letter = false;
    for cluster in trimmed.graphemes(true) {
        if !took_letter && OPENERS.contains(&cluster) {
            end += cluster.len();
            continue;
        }
        end += cluster.len();
        took_letter = true;
        break;
    }
    if !took_letter {
        // Only openers, no letter: nothing to make an initial of.
        return None;
    }
    Some((lead, &trimmed[..end], &trimmed[end..]))
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

/// The same text with the characters XML 1.0 has no way of carrying removed.
///
/// **Escaping is not an option for these and that is the point.** XML 1.0
/// permits tab, line feed and carriage return out of the whole C0 range, and
/// forbids the rest *including as numeric references* — there is no spelling
/// of U+000B that a conforming parser will accept. So a source file with a
/// stray control byte, which is what a bad export or a truncated copy leaves
/// behind, cannot be represented and can only be dropped.
///
/// Found by accident and worth keeping: a single vertical tab in one book made
/// the backend fail with `not well-formed (invalid token)` and no file, line or
/// character named. The emitter is the last place that can still tell the
/// difference, so it is the place that has to.
///
/// Returns `Cow` because the case this exists for is vanishingly rare and the
/// case it runs in is every text node in a Bible.
fn without_forbidden(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.chars().any(is_forbidden) {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.chars().filter(|c| !is_forbidden(*c)).collect())
}

/// Whether XML 1.0 forbids this character outright.
fn is_forbidden(c: char) -> bool {
    match c {
        '\t' | '\n' | '\r' => false,
        // C0, and the two permanently unassigned code points at the end of the
        // BMP, which are not characters in any XML version.
        '\u{0}'..='\u{1f}' => true,
        '\u{fffe}' | '\u{ffff}' => true,
        _ => false,
    }
}

#[cfg(test)]
mod forbidden {
    use super::*;

    /// The three C0 characters XML allows survive; the rest go.
    #[test]
    fn only_the_three_permitted_control_characters_survive() {
        assert_eq!(without_forbidden("plain\text\n"), "plain\text\n");
        assert_eq!(without_forbidden("a\u{b}b"), "ab", "a vertical tab");
        assert_eq!(without_forbidden("a\u{0}b"), "ab", "a NUL");
        assert_eq!(without_forbidden("a\u{1b}b"), "ab", "an escape");
        assert_eq!(without_forbidden("a\u{fffe}b"), "ab", "a non-character");
        // Everything a Bible is actually made of, untouched.
        assert_eq!(without_forbidden("ஆதியாகமம்"), "ஆதியாகமம்");
        assert_eq!(without_forbidden("\u{2014}\u{a0}"), "\u{2014}\u{a0}");
    }

    /// **And the emitted document parses.** The defect this exists for was not
    /// a wrong character in the output — it was a backend that refused the
    /// whole file with `not well-formed (invalid token)`, naming no book, no
    /// line and no character.
    #[test]
    fn a_control_character_in_the_source_does_not_break_the_document() {
        use biblecompose_scripture::{
            Block, Book, BookCode, BookNames, Inline, ParaStyle, ScriptureDocument,
        };
        let doc = ScriptureDocument::new(vec![Book::new(
            BookCode::parse("MRK").expect("a book code"),
            BookNames::named("Mark\u{b}"),
            vec![Block::Paragraph {
                style: ParaStyle::P,
                content: vec![Inline::Text(
                    "In the beginning\u{b} was the Word.".to_owned(),
                )],
            }],
        )]);
        let xml = emit(&doc, &[]).xml;

        assert!(
            !xml.contains('\u{b}'),
            "the document still carries it: {xml}"
        );
        assert!(xml.contains("In the beginning was the Word."));
        // Parsed rather than eyeballed: a well-formedness claim that nothing
        // parses is a claim about a string.
        let mut reader = quick_xml::Reader::from_str(&xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("the emitted document is not well-formed: {e}"),
            }
        }
    }
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

        // A backslash survives as a backslash — it is not a command. (The
        // paragraph opens the chapter, so its first letter is the initial; the
        // backslashes after it are as inert as they were before it.)
        assert!(out
            .xml
            .contains(r"<initial>B</initial>ackslash \bd and \par"));
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

    // ----------------------------------------------------------- hiding
    //
    // What a project chooses not to print, and the one property every one of
    // these has to keep: the Scripture anchors stay. A hidden part that took
    // its `\c` with it would empty the running head of every page it fell on,
    // and would do it silently.

    /// A book shaped the way USJ delivers one: an introduction, an outline, a
    /// chapter label with the chapter anchor *inside* it, a section heading,
    /// and a verse.
    fn withholdable() -> biblecompose_scripture::ScriptureDocument {
        use biblecompose_scripture::{
            canon::BookCode, Block, Book, BookNames, HeadingStyle, Inline, ParaStyle, VerseId,
        };
        let words = |s: &str| Inline::Text(s.to_owned());
        biblecompose_scripture::ScriptureDocument::new(vec![Book::new(
            BookCode::parse("1JN").expect("a real book code"),
            BookNames::named("1 John"),
            vec![
                Block::Heading {
                    style: HeadingStyle::Is,
                    level: 1,
                    content: vec![words("Introduction")],
                },
                Block::Paragraph {
                    style: ParaStyle::Ip,
                    content: vec![words("Written to assure believers.")],
                },
                Block::Heading {
                    style: HeadingStyle::Iot,
                    level: 1,
                    content: vec![words("Outline")],
                },
                Block::Paragraph {
                    style: ParaStyle::Io1,
                    content: vec![words("Fellowship with God")],
                },
                Block::Paragraph {
                    style: ParaStyle::Cl,
                    content: vec![
                        Inline::Chapter {
                            number: 1,
                            published: None,
                            alternate: None,
                        },
                        words("Chapter One"),
                    ],
                },
                Block::Heading {
                    style: HeadingStyle::S,
                    level: 1,
                    content: vec![words("The Word of Life")],
                },
                Block::Paragraph {
                    style: ParaStyle::P,
                    content: vec![
                        Inline::Verse {
                            id: VerseId::single(1),
                            published: None,
                            alternate: None,
                        },
                        words("That which was from the beginning."),
                    ],
                },
            ],
        )])
    }

    #[test]
    fn nothing_is_withheld_by_default() {
        let out = emit(&withholdable(), &[]);
        for kept in ["Introduction", "Outline", "Chapter One", "The Word of Life"] {
            assert!(out.xml.contains(kept), "{kept} should be printed");
        }
    }

    #[test]
    fn each_part_can_be_withheld_on_its_own() {
        let cases: [(Hidden, &str, &str); 4] = [
            (
                Hidden {
                    book_introductions: true,
                    ..Hidden::nothing()
                },
                "Written to assure believers.",
                "Outline",
            ),
            (
                Hidden {
                    introductory_outlines: true,
                    ..Hidden::nothing()
                },
                "Fellowship with God",
                "Written to assure believers.",
            ),
            (
                Hidden {
                    section_headings: true,
                    ..Hidden::nothing()
                },
                "The Word of Life",
                "Chapter One",
            ),
            (
                Hidden {
                    chapter_labels: true,
                    ..Hidden::nothing()
                },
                "Chapter One",
                "The Word of Life",
            ),
        ];

        for (hidden, gone, kept) in cases {
            let out = emit_hiding(&withholdable(), &[], hidden.clone());
            assert!(
                !out.xml.contains(gone),
                "{hidden:?} left {gone:?} on the page"
            );
            assert!(out.xml.contains(kept), "{hidden:?} also took {kept:?}");
        }
    }

    /// The reason `emit_anchors` exists, asserted rather than remembered.
    ///
    /// The chapter anchor is inside the label's paragraph and the verse anchor
    /// is inside the ordinary one; withholding everything must leave both.
    #[test]
    fn withholding_a_part_keeps_the_anchors_inside_it() {
        let all = Hidden {
            book_introductions: true,
            introductory_outlines: true,
            section_headings: true,
            chapter_labels: true,
            figures: Vec::new(),
        };
        let out = emit_hiding(&withholdable(), &[], all);
        assert!(
            out.xml.contains(r#"<chapter n="1"/>"#),
            "the chapter anchor went with the label: {}",
            out.xml
        );
        assert!(
            out.xml.contains(r#"<verse n="1""#),
            "the verse anchor is gone too: {}",
            out.xml
        );
        assert!(!out.xml.contains("Chapter One"), "{}", out.xml);
    }
}

/// The chapter's opening initial: which run it is, and where it is not.
#[cfg(test)]
mod initial_tests {
    use super::*;
    use biblecompose_scripture::fixtures;

    // ------------------------------------------------------- the split ----

    #[test]
    fn a_latin_opening_is_one_letter() {
        assert_eq!(
            initial("In the beginning"),
            Some(("", "I", "n the beginning"))
        );
    }

    /// Whitespace before the opening is handed back separately, and dropped by
    /// the caller: inside `<initial>` it would be a space three lines tall.
    #[test]
    fn leading_whitespace_is_set_aside() {
        assert_eq!(initial("  In the"), Some(("  ", "I", "n the")));
        assert_eq!(initial("   "), None);
        assert_eq!(initial(""), None);
    }

    /// An opening quotation mark goes with the letter it opens.
    #[test]
    fn an_opening_quote_is_taken_with_the_letter() {
        assert_eq!(
            initial("“In the beginning"),
            Some(("", "“I", "n the beginning"))
        );
        assert_eq!(initial("\"In the"), Some(("", "\"I", "n the")));
        assert_eq!(initial("(For the"), Some(("", "(F", "or the")));
        // Only openers and nothing to open: no initial.
        assert_eq!(initial("“"), None);
    }

    /// **A syllable, not a code point.** தி is த followed by the vowel sign ி;
    /// an initial of த alone would strand the vowel sign at body size on the
    /// next run, which is not a syllable of anything.
    #[test]
    fn a_tamil_opening_is_the_whole_syllable() {
        let (_, first, rest) = initial("திருவிவிலியம்").expect("an opening");
        assert_eq!(first, "தி");
        assert_eq!(first.chars().count(), 2, "consonant and vowel sign");
        assert_eq!(rest, "ருவிவிலியம்");
    }

    /// A Devanagari conjunct — consonant, virama, consonant — is one cluster
    /// since Unicode 15.1, and the segmenter in the tree implements 16.
    #[test]
    fn a_devanagari_conjunct_stays_together() {
        let (_, first, _) = initial("क्षमा").expect("an opening");
        assert_eq!(
            first, "क्ष",
            "the conjunct is one initial, not a dead consonant"
        );
    }

    // ------------------------------------------------- in the document ----

    #[test]
    fn a_chapters_first_paragraph_marks_its_initial() {
        let out = emit(&fixtures::john_1_1_5(), &[]);
        assert!(
            out.xml.contains(r#"<chapter n="1"/><verse n="1" start="1" end="1"/><initial>I</initial>n the beginning"#),
            "{}",
            out.xml
        );
        // Once per chapter, however many paragraphs follow.
        assert_eq!(out.xml.matches("<initial>").count(), 1, "{}", out.xml);
    }

    /// The anchor lives in a psalm's superscription; the psalm starts after
    /// it. The heading does not take the initial, and the first paragraph
    /// does.
    #[test]
    fn a_heading_carrying_the_anchor_leaves_the_initial_for_the_text() {
        let out = emit(&fixtures::headings(), &[]);
        assert!(
            !out.xml
                .contains("<heading style=\"d\" level=\"1\"><chapter n=\"3\"/><initial>"),
            "the superscription is not the opening: {}",
            out.xml
        );
        assert!(
            out.xml
                .contains(r#"<verse n="1" start="1" end="1"/><initial>O</initial> LORD, how many"#),
            "{}",
            out.xml
        );
        assert_eq!(out.xml.matches("<initial>").count(), 1);
    }

    /// Text inside a footnote or a cross-reference is never the opening, and
    /// the initial waits for the first word outside them.
    #[test]
    fn a_note_before_the_first_word_is_not_the_first_word() {
        let out = emit(&fixtures::kitchen_sink(), &[]);
        assert!(
            out.xml.contains(r#"<initial>P</initial>lain text"#),
            "{}",
            out.xml
        );
        assert_eq!(out.xml.matches("<initial>").count(), 1, "{}", out.xml);
    }

    /// A chapter label is not the chapter's opening. `\cl` is a paragraph in
    /// the model and a heading in every other respect: it carries the anchor,
    /// and the text starts after it.
    #[test]
    fn a_chapter_label_leaves_the_initial_for_the_text() {
        use biblecompose_scripture::{Block, Inline, ParaStyle};
        let mut doc = fixtures::john_1_1_5();
        doc.books[0].blocks = vec![
            Block::Paragraph {
                style: ParaStyle::Cl,
                content: vec![
                    Inline::Chapter {
                        number: 1,
                        published: None,
                        alternate: None,
                    },
                    Inline::Text("Chapter One".to_owned()),
                ],
            },
            Block::Paragraph {
                style: ParaStyle::P,
                content: vec![Inline::Text("In the beginning".to_owned())],
            },
        ];
        let out = emit(&doc, &[]);
        assert!(
            !out.xml.contains("<initial>C</initial>hapter"),
            "{}",
            out.xml
        );
        assert!(
            out.xml.contains("<initial>I</initial>n the beginning"),
            "{}",
            out.xml
        );
        assert_eq!(out.xml.matches("<initial>").count(), 1);
    }

    /// Every fixture still emits deterministically with the initial marked.
    #[test]
    fn the_initial_does_not_disturb_determinism() {
        for (name, doc) in fixtures::all() {
            let a = emit(&doc, &[]).xml;
            let b = emit(&doc, &[]).xml;
            assert_eq!(a, b, "fixture {name} is not deterministic");
        }
    }
}
