//! What a style says, and the built-in set that covers every marker.
//!
//! STY-001: every marker BibleCompose claims to support renders without a
//! project override. That is asserted rather than asserted-to: the selector
//! list is generated from the model's marker enums, and a test walks all of
//! them through this sheet.
//!
//! # A style names only what it changes
//!
//! Every property is optional, and most built-in styles set none. That is not
//! an incomplete table — it is the table saying "this marker is supported and
//! renders as body text", which is the correct answer for `\p`, `\m` and most
//! of the introduction markers. The cascade at P3.2 is what turns a stack of
//! partial styles into a complete one; this is the bottom of that stack.

use std::collections::BTreeMap;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, SourceLoc};

use crate::document::{ConfigDocument, Node};
use crate::selector::StyleSelector;
use crate::value::{self, Length};

/// The built-in styles, as the TOML a project overrides.
///
/// A file for the same reasons `defaults.toml` is one: read by the code that
/// reads a project's sheet, shown by the inspector as the built-in value, and
/// legible as documentation of every property that exists.
pub const BUILTIN_STYLES_TOML: &str = include_str!("../styles.toml");

/// How a run of text is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Justify,
}

impl Align {
    const NAMES: [(&'static str, Align); 4] = [
        ("start", Align::Start),
        ("center", Align::Center),
        ("end", Align::End),
        ("justify", Align::Justify),
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Align::Start => "start",
            Align::Center => "center",
            Align::End => "end",
            Align::Justify => "justify",
        }
    }
}

/// Every property a style may set.
///
/// One flat set rather than per-selector shapes. A `space_above` on a
/// character style is meaningless, but expressing that in the type system
/// means a type per selector class and a schema nobody can read — and the
/// harm is bounded, because a property the class cannot use is a property the
/// class ignores. STY-004 reports a *misspelled* property, which is the case
/// that matters.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Style {
    pub font_size: Option<Length>,
    /// 400 is regular, 700 bold — the OpenType scale SILE takes.
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    pub smallcaps: Option<bool>,
    pub space_above: Option<Length>,
    pub space_below: Option<Length>,
    /// Indent of the whole block from the leading margin.
    pub indent: Option<Length>,
    /// Vertical shift, for a verse number set as a superior figure.
    pub raise: Option<Length>,
    pub align: Option<Align>,
}

impl Style {
    /// Whether this style says anything at all.
    pub fn is_empty(&self) -> bool {
        *self == Style::default()
    }

    /// `other`'s properties over this one's, property by property (STY-002).
    ///
    /// The whole cascade in one function: an override changes only what it
    /// names. P3.2 applies it along the inheritance chain; P3.1 needs it to
    /// state what "override" means.
    pub fn overlaid_with(self, other: Style) -> Style {
        Style {
            font_size: other.font_size.or(self.font_size),
            weight: other.weight.or(self.weight),
            italic: other.italic.or(self.italic),
            smallcaps: other.smallcaps.or(self.smallcaps),
            space_above: other.space_above.or(self.space_above),
            space_below: other.space_below.or(self.space_below),
            indent: other.indent.or(self.indent),
            raise: other.raise.or(self.raise),
            align: other.align.or(self.align),
        }
    }
}

/// The key naming a style's parent. Not a property — it says where the other
/// properties come from.
pub const INHERITS: &str = "inherits";

/// The property names, in one place.
///
/// Used to read a style and to detect a misspelled property, so the two
/// cannot disagree about what is legal.
pub const PROPERTIES: [&str; 9] = [
    "font_size",
    "weight",
    "italic",
    "smallcaps",
    "space_above",
    "space_below",
    "indent",
    "raise",
    "align",
];

/// One selector's entry in a sheet: what it says, what it inherits from, and
/// where each of those was written.
///
/// The locations are kept per property rather than per entry because that is
/// the granularity the inspector answers at (STY-008) and the granularity the
/// cascade decides at — two properties of one selector routinely come from two
/// different files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleEntry {
    pub style: Style,
    /// STY-007's single parent, named explicitly. `None` means the implicit
    /// one — the level below, for a levelled family — or nothing.
    pub inherits: Option<StyleSelector>,
    /// Where `inherits` itself was written, for the cycle diagnostic.
    pub inherits_at: Option<SourceLoc>,
    pub locations: BTreeMap<&'static str, SourceLoc>,
}

/// A whole sheet: every selector that has something to say, and what.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleSheet {
    entries: BTreeMap<StyleSelector, StyleEntry>,
}

impl StyleSheet {
    /// What this sheet says about a selector on its own, with no cascade.
    pub fn get(&self, selector: StyleSelector) -> Style {
        self.entries
            .get(&selector)
            .map(|e| e.style)
            .unwrap_or_default()
    }

    pub fn entry(&self, selector: StyleSelector) -> Option<&StyleEntry> {
        self.entries.get(&selector)
    }

    pub fn contains(&self, selector: StyleSelector) -> bool {
        self.entries.contains_key(&selector)
    }

    pub fn set(&mut self, selector: StyleSelector, entry: StyleEntry) {
        self.entries.insert(selector, entry);
    }

    pub fn iter(&self) -> impl Iterator<Item = (StyleSelector, &StyleEntry)> {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    /// Every selector either sheet mentions, in key order.
    pub fn selectors(&self) -> impl Iterator<Item = StyleSelector> + '_ {
        self.entries.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The built-in sheet.
///
/// Panics only on a file compiled into the executable, and
/// `the_built_in_sheet_is_clean` proves it parses and reads without a single
/// diagnostic before any of it is released — the same bargain as
/// `defaults.toml`.
pub fn builtin() -> StyleSheet {
    let doc = ConfigDocument::parse("<built-in styles>", BUILTIN_STYLES_TOML.to_owned())
        .expect("the built-in styles are valid TOML");
    let (sheet, diagnostics) = read(&doc);
    debug_assert!(
        diagnostics.is_empty(),
        "the built-in styles produced diagnostics: {:?}",
        diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    sheet
}

/// Read a sheet, reporting what cannot be used rather than dropping it.
///
/// Two shapes of complaint, both STY-004's: a selector this release has no
/// element for, and a property it has no meaning for. Each is reported at its
/// own line and the rest of the sheet is still read, because a typo in one
/// heading is not a reason to lose a publisher's whole design.
pub fn read(doc: &ConfigDocument) -> (StyleSheet, Diagnostics) {
    let mut sheet = StyleSheet::default();
    let mut diagnostics = Diagnostics::new();

    let root = doc.root().table().expect("a document root is a table");
    for class in root.names() {
        let Some(node) = root.get(class) else {
            continue;
        };

        // `[chapter]` is a style; `[paragraph]` is a group of them. Which one
        // a class is, is decided by whether it parses as a whole selector.
        if let Some(selector) = StyleSelector::parse(class) {
            read_into(&mut sheet, selector, &node, &mut diagnostics);
            continue;
        }

        let Ok(group) = node.table() else {
            diagnostics.push(unknown_selector(&node));
            continue;
        };
        for name in group.names() {
            let Some(entry) = group.get(name) else {
                continue;
            };
            match StyleSelector::parse(&format!("{class}.{name}")) {
                Some(selector) => read_into(&mut sheet, selector, &entry, &mut diagnostics),
                None => diagnostics.push(unknown_selector(&entry)),
            }
        }
    }

    (sheet, diagnostics)
}

fn read_into(
    sheet: &mut StyleSheet,
    selector: StyleSelector,
    node: &Node<'_>,
    diagnostics: &mut Diagnostics,
) {
    let table = match node.table() {
        Ok(table) => table,
        Err(d) => {
            diagnostics.push(d);
            return;
        }
    };

    let mut entry = sheet.entry(selector).cloned().unwrap_or_default();
    let style = &mut entry.style;
    let locations = &mut entry.locations;

    let mut read = |key: &'static str, f: &mut dyn FnMut(&Node<'_>) -> Result<(), Diagnostic>| {
        if let Some(value) = table.get(key) {
            match f(&value) {
                // Recorded only when the value was usable: a property that was
                // rejected is not in force, and pointing the inspector at the
                // line it was written on would say it is.
                Ok(()) => {
                    locations.insert(key, value.loc());
                }
                Err(d) => diagnostics.push(d),
            }
        }
    };

    read("font_size", &mut |n| {
        style.font_size = Some(value::length(n)?.value);
        Ok(())
    });
    read("weight", &mut |n| {
        style.weight = Some(value::integer_in(n, 100, 900)?.value as u16);
        Ok(())
    });
    read("italic", &mut |n| {
        style.italic = Some(n.boolean()?.value);
        Ok(())
    });
    read("smallcaps", &mut |n| {
        style.smallcaps = Some(n.boolean()?.value);
        Ok(())
    });
    read("space_above", &mut |n| {
        style.space_above = Some(value::length_or_zero(n)?.value);
        Ok(())
    });
    read("space_below", &mut |n| {
        style.space_below = Some(value::length_or_zero(n)?.value);
        Ok(())
    });
    read("indent", &mut |n| {
        style.indent = Some(value::length_or_zero(n)?.value);
        Ok(())
    });
    read("raise", &mut |n| {
        style.raise = Some(value::length_or_zero(n)?.value);
        Ok(())
    });
    read("align", &mut |n| {
        style.align = Some(value::choice(n, &Align::NAMES)?.value);
        Ok(())
    });

    // STY-007: a single named parent.
    if let Some(node) = table.get(INHERITS) {
        match node.string() {
            Ok(named) => match StyleSelector::parse(&named.value) {
                Some(parent) if parent == selector => diagnostics.push(
                    Diagnostic::warning(
                        code::INHERITANCE_CYCLE,
                        format!("`{selector}` inherits from itself"),
                    )
                    .at(node.loc()),
                ),
                Some(parent) => {
                    entry.inherits = Some(parent);
                    entry.inherits_at = Some(node.loc());
                }
                None => diagnostics.push(
                    Diagnostic::warning(
                        code::UNKNOWN_SELECTOR,
                        format!(
                            "`{selector}` inherits from `{}`, which is not an element this                              release can style",
                            named.value
                        ),
                    )
                    .at(node.loc()),
                ),
            },
            Err(d) => diagnostics.push(d),
        }
    }

    // STY-004: anything left is a property this release has no meaning for.
    for name in table.names() {
        if name != INHERITS && !PROPERTIES.contains(&name) {
            if let Some(stray) = table.get(name) {
                diagnostics.push(
                    Diagnostic::warning(
                        code::UNKNOWN_PROPERTY,
                        format!("`{}` is not a style property", stray.dotted_path()),
                    )
                    .at(stray.loc())
                    .help(format!("the properties are: {}", PROPERTIES.join(", "))),
                );
            }
        }
    }

    sheet.set(selector, entry);
}

fn unknown_selector(node: &Node<'_>) -> Diagnostic {
    Diagnostic::warning(
        code::UNKNOWN_SELECTOR,
        format!(
            "`{}` is not an element this release can style",
            node.dotted_path()
        ),
    )
    .at(node.loc())
    .help("styles are written as `[class.marker]`, for example `[poetry.q1]` or `[character.bd]`")
}
