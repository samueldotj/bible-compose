//! The schema, described well enough to build a form from (GUI-002).
//!
//! This lives here rather than in the window because it *is* the schema: which
//! keys exist, what kind of value each holds, what it is currently set to and
//! where that came from. A settings pane that knew those things itself would
//! be a second copy of the schema, kept in a language that cannot be checked
//! against the first.
//!
//! What is deliberately *not* here is anything about presentation — no labels,
//! no grouping order, no help text. Those are words shown to a person and
//! belong where words shown to a person are translated.

use crate::edit::SettingValue;
use crate::provenance::Origin;
use crate::settings::Settings;
use crate::value::{CallerStyle, HeadSlot, ReferencePlacement, RestartNumbering};

/// What kind of control a key needs, and how its text is to be read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// A font family name. Text as far as the file is concerned, but a form
    /// can offer the ones that exist rather than asking a person to spell one,
    /// and this is where it learns that it may.
    Font,
    /// A BCP-47 language tag. Text too, and for the same reason: a form can
    /// offer the languages people publish in rather than asking for a tag
    /// from memory.
    Language,
    /// A length with a unit — `"0.55in"`.
    Length,
    /// `"6x9in"`, or a named size.
    PageSize,
    Integer,
    Boolean,
    /// A path relative to the project folder.
    Path,
    /// One of a closed set of spellings, carrying the set.
    ///
    /// One variant for every enum rather than one each. The form needs to know
    /// *that* the value comes from a list and *what* the list is, and neither
    /// question has a different answer for a head slot than for a caller
    /// style — a variant per enum would be a `<select>` to write per enum for
    /// no gain. The list is the resolver's own table, so a form cannot offer a
    /// spelling the file would reject, nor miss one it would accept.
    Choice(&'static [&'static str]),
    /// Book codes, comma-separated in the form.
    List,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Font => "font",
            Kind::Language => "language",
            Kind::Length => "length",
            Kind::PageSize => "page_size",
            Kind::Integer => "integer",
            Kind::Boolean => "boolean",
            Kind::Path => "path",
            Kind::Choice(_) => "choice",
            Kind::List => "list",
        }
    }

    /// The spellings a choice accepts, or nothing for every other kind.
    pub const fn choices(self) -> &'static [&'static str] {
        match self {
            Kind::Choice(names) => names,
            _ => &[],
        }
    }

    /// Turn what a form field holds into something that can be written back.
    ///
    /// Only shape, not validity: whether `"quarto"` is a page size is decided
    /// by the same reader that decides it for a hand-written file, when the
    /// edited document is resolved again. There is no second opinion here.
    pub fn read(self, text: &str) -> SettingValue {
        let text = text.trim();
        match self {
            Kind::Integer => text
                .parse::<i64>()
                .map(SettingValue::Int)
                .unwrap_or_else(|_| SettingValue::Str(text.to_owned())),
            Kind::Boolean => SettingValue::Bool(matches!(
                text.to_ascii_lowercase().as_str(),
                "true" | "yes" | "on" | "1"
            )),
            Kind::List => SettingValue::List(
                text.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            _ => SettingValue::Str(text.to_owned()),
        }
    }
}

/// One row of a settings form.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub key: &'static str,
    pub kind: Kind,
    /// The resolved value, as the form should show it — a length in the unit
    /// it was written in, a list comma-separated.
    pub value: String,
    /// Whether the project file set it, and where (ADR-005). This is what the
    /// reset control reads and what an inspector shows.
    pub origin: Origin,
}

impl Settings {
    /// Every setting, in the order a form should lay them out.
    ///
    /// Grouped the way the file is, because a publisher who has both open
    /// should be able to move between them without translating.
    pub fn fields(&self) -> Vec<Field> {
        use Kind::*;
        let mut out = Vec::new();

        let mut push = |key: &'static str, kind: Kind, value: String| {
            out.push(Field {
                key,
                kind,
                value,
                // A key with no origin was never resolved from anywhere —
                // the two optional ones, when unset. `Builtin` is the honest
                // answer for a form: it is showing the value that applies.
                origin: self.provenance.get(key).cloned().unwrap_or(Origin::Builtin),
            });
        };

        push(
            "project.name",
            Text,
            self.project
                .name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_default(),
        );
        push(
            "project.language",
            Language,
            self.project.language.to_string(),
        );

        push("books.order", List, join(&self.books.order));
        push(
            "books.include",
            List,
            self.books
                .include
                .as_ref()
                .map(|i| join(i))
                .unwrap_or_default(),
        );

        push("page.size", PageSize, self.page.size.to_string());
        push("page.columns", Integer, self.page.columns.to_string());
        push("page.margin_top", Length, self.page.margin_top.to_string());
        push(
            "page.margin_bottom",
            Length,
            self.page.margin_bottom.to_string(),
        );
        push(
            "page.margin_inner",
            Length,
            self.page.margin_inner.to_string(),
        );
        push(
            "page.margin_outer",
            Length,
            self.page.margin_outer.to_string(),
        );
        push("page.column_gap", Length, self.page.column_gap.to_string());
        push("page.header_gap", Length, self.page.header_gap.to_string());
        push("page.footer_gap", Length, self.page.footer_gap.to_string());

        push(
            "typography.font_family",
            Font,
            self.typography.font_family.to_string(),
        );
        push(
            "typography.font_size",
            Length,
            self.typography.font_size.to_string(),
        );
        push(
            "typography.leading",
            Length,
            self.typography.leading.to_string(),
        );
        push(
            "typography.hyphenation",
            Boolean,
            self.typography.hyphenation.to_string(),
        );
        push(
            "typography.justify",
            Boolean,
            self.typography.justify.to_string(),
        );
        push(
            "typography.keep_poetry_indentation",
            Boolean,
            self.typography.keep_poetry_indentation.to_string(),
        );

        push(
            "numbering.show_chapter_numbers",
            Boolean,
            self.numbering.show_chapter_numbers.to_string(),
        );
        push(
            "numbering.show_verse_numbers",
            Boolean,
            self.numbering.show_verse_numbers.to_string(),
        );
        push(
            "numbering.hide_first_verse_number",
            Boolean,
            self.numbering.hide_first_verse_number.to_string(),
        );

        push(
            "contents.show_book_introductions",
            Boolean,
            self.contents.show_book_introductions.to_string(),
        );
        push(
            "contents.show_introductory_outlines",
            Boolean,
            self.contents.show_introductory_outlines.to_string(),
        );
        push(
            "contents.show_section_headings",
            Boolean,
            self.contents.show_section_headings.to_string(),
        );

        push(
            "notes.show_footnotes",
            Boolean,
            self.notes.show_footnotes.to_string(),
        );
        push(
            "notes.show_cross_references",
            Boolean,
            self.notes.show_cross_references.to_string(),
        );
        push(
            "notes.footnote_callers",
            Choice(CallerStyle::SPELLINGS),
            self.notes.footnote_callers.to_string(),
        );
        push(
            "notes.cross_reference_callers",
            Choice(CallerStyle::SPELLINGS),
            self.notes.cross_reference_callers.to_string(),
        );
        push(
            "notes.restart_numbering",
            Choice(RestartNumbering::SPELLINGS),
            self.notes.restart_numbering.to_string(),
        );
        push(
            "notes.cross_reference_placement",
            Choice(ReferencePlacement::SPELLINGS),
            self.notes.cross_reference_placement.to_string(),
        );

        for (key, slot) in [
            ("headers.header_left", &self.headers.header_left),
            ("headers.header_center", &self.headers.header_center),
            ("headers.header_right", &self.headers.header_right),
            ("headers.footer_left", &self.headers.footer_left),
            ("headers.footer_center", &self.headers.footer_center),
            ("headers.footer_right", &self.headers.footer_right),
        ] {
            push(key, Choice(HeadSlot::SPELLINGS), slot.to_string());
        }

        push(
            "output.keep_intermediates",
            Boolean,
            self.output.keep_intermediates.to_string(),
        );

        push("strict", Boolean, self.strict.to_string());

        out
    }

    /// The kind of one key, for reading a form field back.
    pub fn kind_of(&self, key: &str) -> Option<Kind> {
        self.fields()
            .into_iter()
            .find(|f| f.key == key)
            .map(|f| f.kind)
    }
}

fn join(items: &[String]) -> String {
    items.join(", ")
}
