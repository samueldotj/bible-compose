//! Resolved configuration, translated into what the SILE class takes.
//!
//! This is the one place the two vocabularies meet, and it lives here rather
//! than in either of them on purpose. `biblecompose-config` must not know
//! there is a SILE, and `biblecompose-sile` must not know there is a
//! configuration layer (ARCHITECTURE §2); the app is the only crate that has
//! heard of both, so the translation is its job (ADR-004).
//!
//! Two rules hold here:
//!
//! * **Provenance stops at this function.** It takes `&Settings`, whose fields
//!   are `Sourced`, and returns plain strings. There is no way to pass an
//!   origin further down, which is what [ADR-005] asks for: a file path that
//!   can influence the output is a file path that can reach a golden file.
//! * **Lengths cross as points.** `Length::to_sile` formats at fixed
//!   precision, so `0.55in`, `39.6pt` and `13.97mm` produce the same argument
//!   and therefore the same build (DET-001).
//!
//! [ADR-005]: ../../../docs/adr/005-provenance.md

use std::collections::BTreeMap;

use biblecompose_config::style::PROPERTIES;
use biblecompose_config::{ResolvedStyles, Settings};
use biblecompose_sile::StyleRule;

/// The `-O key=value` pairs for one build, in a fixed order.
///
/// Ordered because the argument list is part of what makes two runs of the
/// same build identical. Grouped the way the settings file is, so a reader
/// comparing the two can follow.
pub fn class_options(s: &Settings) -> Vec<(String, String)> {
    class_options_with(s, None, None)
}

/// The same, told which file the body font resolved to.
///
/// A project font crosses as a path (FONT-003): fontconfig has never heard of
/// it, so a family name would silently resolve to something else — which is
/// the failure the coverage check exists to catch, reintroduced one step later.
pub fn class_options_with(
    s: &Settings,
    body_font: Option<&crate::font::ResolvedFont>,
    hyphenation: Option<&crate::hyphenation::Hyphenation>,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut put = |key: &str, value: String| out.push((key.to_owned(), value));

    // Geometry.
    put("papersize", s.page.size.to_sile());
    put("columns", s.page.columns.to_string());
    put("margintop", s.page.margin_top.to_sile());
    put("marginbottom", s.page.margin_bottom.to_sile());
    put("margininner", s.page.margin_inner.to_sile());
    put("marginouter", s.page.margin_outer.to_sile());
    put("gutter", s.page.column_gap.to_sile());
    put("headsep", s.page.header_gap.to_sile());
    put("footsep", s.page.footer_gap.to_sile());

    // Typography.
    put("fontfamily", s.typography.font_family.to_string());
    put(
        "fontfile",
        match body_font {
            Some(font) if font.from_project => font.path.to_string(),
            // A system font is named, so the backend picks the face for the
            // weight and style each run asks for rather than being pinned to
            // the one file this check happened to read.
            _ => String::new(),
        },
    );
    put("fontsize", s.typography.font_size.to_sile());
    put("leading", s.typography.leading.to_sile());
    put("language", s.project.language.to_string());
    // What pre-flight decided, not what the file asked for. The setting is
    // the request; this is the answer, and FONT-004's diagnostic is where the
    // difference is explained.
    put(
        "hyphenate",
        flag(match hyphenation {
            Some(h) => h.enabled,
            None => *s.typography.hyphenation,
        }),
    );

    // What the file says about itself (PDF-005). Each crosses as the empty
    // string when unset, and the class writes no property for an empty one —
    // a `/Title ()` in the properties panel is worse than no title, because it
    // looks like an answer.
    let said = |v: &Option<biblecompose_config::Sourced<String>>| {
        v.as_ref().map(|n| n.to_string()).unwrap_or_default()
    };
    put("anchors", s.output.anchors.to_string());
    put("title", said(&s.project.name));
    put("author", said(&s.project.author));
    put("subject", said(&s.project.subject));

    // What appears on the page.
    put("chapternumbers", flag(*s.numbering.show_chapter_numbers));
    put("versenumbers", flag(*s.numbering.show_verse_numbers));
    put("hidefirstverse", flag(*s.numbering.hide_first_verse_number));
    // The initial itself is marked in the document (`<initial>`), since what
    // a chapter's first syllable *is* takes Unicode segmentation the class
    // has not got; whether it drops, and how far, is the class's to decide.
    put("dropcaps", flag(*s.contents.drop_caps));
    put("dropcaplines", s.contents.drop_cap_lines.to_string());
    put("justify", flag(*s.typography.justify));
    put("poetryindent", flag(*s.typography.keep_poetry_indentation));
    put("footnotes", flag(*s.notes.show_footnotes));
    put("crossrefs", flag(*s.notes.show_cross_references));
    put("footnotecallers", s.notes.footnote_callers.to_string());
    put(
        "crossrefcallers",
        s.notes.cross_reference_callers.to_string(),
    );
    put("restartnotes", s.notes.restart_numbering.to_string());
    put(
        "crossrefplacement",
        s.notes.cross_reference_placement.to_string(),
    );
    // Twelve slots rather than four switches: where a thing goes is as much
    // a decision as whether it is there, and which side of the spread it is
    // on is part of where. The class picks a side by the page's parity.
    for (prefix, side) in [
        ("verso", &s.headers.left_page),
        ("recto", &s.headers.right_page),
    ] {
        put(&format!("{prefix}headerleft"), side.header_left.to_string());
        put(
            &format!("{prefix}headercenter"),
            side.header_center.to_string(),
        );
        put(
            &format!("{prefix}headerright"),
            side.header_right.to_string(),
        );
        put(&format!("{prefix}footerleft"), side.footer_left.to_string());
        put(
            &format!("{prefix}footercenter"),
            side.footer_center.to_string(),
        );
        put(
            &format!("{prefix}footerright"),
            side.footer_right.to_string(),
        );
    }

    out
}

/// The parts of a book this project does not print.
///
/// Emission rather than a class option, which every other "what appears"
/// setting is — see [`Hidden`] for the measurement that decided it.
///
/// [`Hidden`]: biblecompose_sile::Hidden
pub fn hidden(s: &Settings) -> biblecompose_sile::Hidden {
    biblecompose_sile::Hidden {
        book_introductions: !*s.contents.show_book_introductions,
        introductory_outlines: !*s.contents.show_introductory_outlines,
        section_headings: !*s.contents.show_section_headings,
        // The one that is a `numbering` setting rather than a `contents` one:
        // it is hidden here because of where the chapter anchor lives, not
        // because of which group a publisher finds it under.
        chapter_labels: !*s.numbering.show_chapter_labels,
        // Filled in by the caller from the asset pre-flight, which is the only
        // thing that knows which files are actually there.
        figures: Vec::new(),
    }
}

/// `"true"` / `"false"`, which is what the class's `SU.boolean` reads. Spelled
/// out rather than `to_string()` so the wire form is stated once here and
/// cannot drift with a `Display` impl somewhere else.
fn flag(on: bool) -> String {
    if on { "true" } else { "false" }.to_owned()
}

/// The resolved styles, as the rules the emitter writes into the document.
///
/// Only selectors with something to say. An empty entry is the built-in
/// sheet saying "this marker is supported and renders as body text", and
/// writing it out would be a line of XML per paragraph marker that means
/// nothing.
///
/// Property names cross unchanged — the TOML key, the XML attribute and the
/// Lua field are one word, so there is no translation table between the file a
/// publisher writes and the table the class reads, and therefore nothing for
/// the two to disagree about.
pub fn style_rules(styles: &ResolvedStyles) -> Vec<StyleRule> {
    style_rules_with(styles, &BTreeMap::new())
}

/// The same, told which file each font a style names resolved to.
///
/// A style's font crosses as a path when the project ships it, for exactly the
/// reason the body font does (FONT-003): fontconfig has never heard of a file
/// in `assets/fonts/`, so a family name would resolve to something else or to
/// nothing at all — and "nothing at all" is a nil font handed to the shaper,
/// which dies several frames from anything that names a font.
///
/// Two functions rather than one with an always-empty argument, so [`emit`]
/// stays hermetic: the golden path must not depend on which fonts the machine
/// running the test happens to have.
///
/// [`emit`]: crate::emit
pub fn style_rules_with(
    styles: &ResolvedStyles,
    fonts: &BTreeMap<String, crate::font::ResolvedFont>,
) -> Vec<StyleRule> {
    let mut out = Vec::new();
    for (selector, resolved) in styles.iter() {
        let properties = properties_of(resolved, fonts);
        if properties.is_empty() {
            continue;
        }
        out.push(StyleRule {
            selector: selector.key(),
            properties,
        });
    }
    out
}

/// One style's properties, in `PROPERTIES` order.
///
/// Lengths cross as points, for the same reason the class options do: the text
/// has to be a function of the value alone, or two runs that resolved the same
/// page by different routes would produce different bytes (DET-001).
fn properties_of(
    resolved: &biblecompose_config::ResolvedStyle,
    fonts: &BTreeMap<String, crate::font::ResolvedFont>,
) -> Vec<(String, String)> {
    let s = &resolved.style;
    let mut out: Vec<(String, String)> = Vec::new();
    let mut put = |name: &str, value: Option<String>| {
        if let Some(value) = value {
            out.push((name.to_owned(), value));
        }
    };

    // One or the other, never both: the class would have to choose, and a
    // class choosing between two ways of naming the same font is a second
    // place for the answer to be decided.
    if let Some(family) = s.font_family.as_deref() {
        match fonts.get(family) {
            Some(found) if found.from_project => put("font_file", Some(found.path.to_string())),
            _ => put("font_family", Some(family.to_owned())),
        }
    }
    put("font_size", s.font_size.map(|l| l.to_sile()));
    put("weight", s.weight.map(|w| w.to_string()));
    put("italic", s.italic.map(flag));
    put("smallcaps", s.smallcaps.map(flag));
    put("space_above", s.space_above.map(|l| l.to_sile()));
    put("space_below", s.space_below.map(|l| l.to_sile()));
    put("indent", s.indent.map(|l| l.to_sile()));
    put("raise", s.raise.map(|l| l.to_sile()));
    put("align", s.align.map(|a| a.as_str().to_owned()));
    put("color", s.color.map(|c| c.to_string()));

    debug_assert!(
        out.len() <= PROPERTIES.len(),
        "a property was written twice, or one exists that PROPERTIES does not list"
    );
    out
}
