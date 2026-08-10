//! What a style applies to (STY-003).
//!
//! **Selectors are typed, not strings.** `[paragraph.p]` and a same-named
//! selector in another class are different values of different variants, so
//! they cannot collide however the file is written — the class is part of the
//! identity rather than a prefix on a string that somebody has to remember to
//! include.
//!
//! The marker vocabulary comes from `biblecompose-scripture` rather than being
//! restated here. A second list of every supported marker, kept in a crate
//! that cannot see the first, is a list that drifts — and the failure would be
//! a marker the model supports and the style layer has never heard of, which
//! renders as nothing with no diagnostic.

use std::fmt;

use biblecompose_scripture::{CharStyle, HeadingStyle, NoteKind, ParaStyle, PoetryStyle};

/// How deep a levelled family goes.
///
/// USFM allows more, and normalization will happily produce `q7`. Four is what
/// the built-in set defines; deeper levels inherit from the deepest defined one
/// (P3.2), which is both what a publisher expects and the only answer that does
/// not require an infinite table.
pub const MAX_LEVEL: u8 = 4;

/// An element a style can be written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StyleSelector {
    Paragraph(ParaStyle),
    /// `\q1`, `\q2` — the family and the digit. The model keeps the level on
    /// the block rather than in the style, so the selector has to carry it or
    /// `q1` and `q2` could not differ in indent.
    Poetry(PoetryStyle, u8),
    Heading(HeadingStyle, u8),
    Character(CharStyle),
    ListItem(u8),
    /// The chapter number where it is set, not the chapter as a division.
    Chapter,
    Verse,
    Note(NoteKind),
    /// A cross-reference. Its own selector rather than a kind of note, for the
    /// same reason `CrossReference` is its own type (SCR-004).
    Reference,
    Figure,
    /// The caption under a figure.
    Caption,
    TableCell,
    RunningHead,
    /// The page number.
    Folio,
}

impl StyleSelector {
    /// The class half of the key — `paragraph`, `poetry`, `character`.
    pub const fn class(self) -> &'static str {
        match self {
            StyleSelector::Paragraph(_) => "paragraph",
            StyleSelector::Poetry(..) => "poetry",
            StyleSelector::Heading(..) => "heading",
            StyleSelector::Character(_) => "character",
            StyleSelector::ListItem(_) => "list",
            StyleSelector::Chapter => "chapter",
            StyleSelector::Verse => "verse",
            StyleSelector::Note(_) => "note",
            StyleSelector::Reference => "reference",
            StyleSelector::Figure => "figure",
            StyleSelector::Caption => "caption",
            StyleSelector::TableCell => "cell",
            StyleSelector::RunningHead => "head",
            StyleSelector::Folio => "folio",
        }
    }

    /// The marker half, where there is one — `p`, `q1`, `bd`, `f`.
    pub fn name(self) -> Option<String> {
        match self {
            StyleSelector::Paragraph(s) => Some(s.marker().to_owned()),
            StyleSelector::Character(s) => Some(s.marker().to_owned()),
            StyleSelector::Poetry(s, level) => Some(format!("{}{level}", s.marker())),
            StyleSelector::Heading(s, level) => Some(format!("{}{level}", s.marker())),
            StyleSelector::ListItem(level) => Some(level.to_string()),
            StyleSelector::Note(NoteKind::Footnote) => Some("f".to_owned()),
            StyleSelector::Note(NoteKind::Endnote) => Some("fe".to_owned()),
            StyleSelector::Chapter
            | StyleSelector::Verse
            | StyleSelector::Reference
            | StyleSelector::Figure
            | StyleSelector::Caption
            | StyleSelector::TableCell
            | StyleSelector::RunningHead
            | StyleSelector::Folio => None,
        }
    }

    /// The dotted key this selector is written as in `styles.toml`.
    pub fn key(self) -> String {
        match self.name() {
            Some(name) => format!("{}.{name}", self.class()),
            None => self.class().to_owned(),
        }
    }

    /// The reverse, for reading a project's file.
    ///
    /// `None` for anything this release does not have a selector for, which
    /// STY-004 reports at the line it was written on rather than ignoring.
    pub fn parse(key: &str) -> Option<StyleSelector> {
        let (class, name) = match key.split_once('.') {
            Some((class, name)) => (class, Some(name)),
            None => (key, None),
        };

        match (class, name) {
            ("paragraph", Some(n)) => marker(ParaStyle::all(), n).map(StyleSelector::Paragraph),
            ("character", Some(n)) => marker(CharStyle::all(), n).map(StyleSelector::Character),
            ("poetry", Some(n)) => {
                let (family, level) = split_level(n)?;
                marker(PoetryStyle::all(), family).map(|s| StyleSelector::Poetry(s, level))
            }
            ("heading", Some(n)) => {
                let (family, level) = split_level(n)?;
                marker(HeadingStyle::all(), family).map(|s| StyleSelector::Heading(s, level))
            }
            ("list", Some(n)) => n.parse().ok().map(StyleSelector::ListItem),
            ("note", Some("f")) => Some(StyleSelector::Note(NoteKind::Footnote)),
            ("note", Some("fe")) => Some(StyleSelector::Note(NoteKind::Endnote)),
            ("chapter", None) => Some(StyleSelector::Chapter),
            ("verse", None) => Some(StyleSelector::Verse),
            ("reference", None) => Some(StyleSelector::Reference),
            ("figure", None) => Some(StyleSelector::Figure),
            ("caption", None) => Some(StyleSelector::Caption),
            ("cell", None) => Some(StyleSelector::TableCell),
            ("head", None) => Some(StyleSelector::RunningHead),
            ("folio", None) => Some(StyleSelector::Folio),
            _ => None,
        }
    }

    /// Every selector this release has a style for.
    ///
    /// Built from the marker enums rather than written out, so STY-001's
    /// "every supported marker renders" is a property of the code and not of
    /// somebody having remembered. Adding a marker to the model adds a
    /// selector here on the next build.
    pub fn all() -> Vec<StyleSelector> {
        let mut out = Vec::new();
        out.extend(
            ParaStyle::all()
                .iter()
                .copied()
                .map(StyleSelector::Paragraph),
        );
        out.extend(
            CharStyle::all()
                .iter()
                .copied()
                .map(StyleSelector::Character),
        );
        for level in 1..=MAX_LEVEL {
            out.extend(
                PoetryStyle::all()
                    .iter()
                    .map(|s| StyleSelector::Poetry(*s, level)),
            );
            out.extend(
                HeadingStyle::all()
                    .iter()
                    .map(|s| StyleSelector::Heading(*s, level)),
            );
            out.push(StyleSelector::ListItem(level));
        }
        out.extend([
            StyleSelector::Chapter,
            StyleSelector::Verse,
            StyleSelector::Note(NoteKind::Footnote),
            StyleSelector::Note(NoteKind::Endnote),
            StyleSelector::Reference,
            StyleSelector::Figure,
            StyleSelector::Caption,
            StyleSelector::TableCell,
            StyleSelector::RunningHead,
            StyleSelector::Folio,
        ]);
        out
    }

    /// The selector one level shallower, which is what a deeper level inherits
    /// from (STY-007). `q1` has none — it inherits from nothing in its own
    /// family.
    pub fn shallower(self) -> Option<StyleSelector> {
        match self {
            StyleSelector::Poetry(s, level) if level > 1 => {
                Some(StyleSelector::Poetry(s, level - 1))
            }
            StyleSelector::Heading(s, level) if level > 1 => {
                Some(StyleSelector::Heading(s, level - 1))
            }
            StyleSelector::ListItem(level) if level > 1 => Some(StyleSelector::ListItem(level - 1)),
            _ => None,
        }
    }
}

impl fmt::Display for StyleSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

fn marker<T: Copy + Marker>(all: &'static [T], name: &str) -> Option<T> {
    all.iter().find(|s| s.marker() == name).copied()
}

/// The one thing every marker enum has in common.
pub trait Marker {
    fn marker(&self) -> &'static str;
}

macro_rules! marker_impl {
    ($($t:ty),*) => {
        $(impl Marker for $t {
            fn marker(&self) -> &'static str {
                <$t>::marker(*self)
            }
        })*
    };
}
marker_impl!(ParaStyle, PoetryStyle, HeadingStyle, CharStyle);

/// `q2` → (`q`, 2); `q` → (`q`, 1).
///
/// The same rule normalization uses to classify a marker, and it has to be:
/// a style written for `q1` must match a block that came from `\q`.
fn split_level(name: &str) -> Option<(&str, u8)> {
    match name.chars().last() {
        Some(d) if d.is_ascii_digit() && name.len() > 1 => {
            let level = d.to_digit(10)? as u8;
            (level > 0).then(|| (&name[..name.len() - 1], level))
        }
        _ => Some((name, 1)),
    }
}
