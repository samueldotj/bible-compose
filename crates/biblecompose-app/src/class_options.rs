//! Resolved settings, translated into what the SILE class takes.
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

use biblecompose_config::Settings;

/// The `-O key=value` pairs for one build, in a fixed order.
///
/// Ordered because the argument list is part of what makes two runs of the
/// same build identical. Grouped the way the settings file is, so a reader
/// comparing the two can follow.
pub fn class_options(s: &Settings) -> Vec<(String, String)> {
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
    put("fontsize", s.typography.font_size.to_sile());
    put("leading", s.typography.leading.to_sile());
    put("language", s.project.language.to_string());
    put("hyphenate", flag(*s.typography.hyphenation));

    // What appears on the page.
    put("chapternumbers", flag(*s.numbering.show_chapter_numbers));
    put("versenumbers", flag(*s.numbering.show_verse_numbers));
    put("footnotes", flag(*s.notes.show_footnotes));
    put("crossrefs", flag(*s.notes.show_cross_references));
    put("runningheads", flag(*s.headers.enabled));
    put("headbook", flag(*s.headers.show_book_name));
    put("headref", flag(*s.headers.show_reference_range));
    put("folio", flag(*s.headers.show_page_number));

    out
}

/// `"true"` / `"false"`, which is what the class's `SU.boolean` reads. Spelled
/// out rather than `to_string()` so the wire form is stated once here and
/// cannot drift with a `Display` impl somewhere else.
fn flag(on: bool) -> String {
    if on { "true" } else { "false" }.to_owned()
}
