//! Turning a stack of partial styles into one complete one (STY-002, STY-007).
//!
//! Three things stack, nearest wins:
//!
//! 1. what the project's sheet says about this selector,
//! 2. what the built-in sheet says about it,
//! 3. what its parent resolves to — the named `inherits`, or the level below.
//!
//! **An override changes only what it names.** A project that sets
//! `[chapter] weight` keeps the built-in `font_size`, because the cascade is
//! per property and not per selector. That is the whole of STY-002 and it is
//! one line: [`Style::overlaid_with`].
//!
//! # A single parent, named or implied
//!
//! STY-007 allows a style to inherit from another. Where none is named, a
//! levelled family implies one: `q2`'s parent is `q1`. That is what keeps the
//! built-in table finite — `\q7` is not in it, and resolves by walking down to
//! the deepest level that is — and it is what a publisher means when they
//! restyle `qa1` and expect `\qa2` to follow.
//!
//! Only one parent, so the chain is a walk and not a graph. It can still be a
//! *cycle*, which a walk finds in one pass and reports as one diagnostic
//! naming the cycle rather than as a stack overflow.

use std::collections::{BTreeMap, BTreeSet};

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, Severity};

use crate::document::ConfigDocument;
use crate::provenance::{Origin, Provenance};
use crate::selector::StyleSelector;
use crate::style::{self, Style, StyleSheet, PROPERTIES};

/// One element's finished appearance, and where each part of it came from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedStyle {
    /// Plain values. The emitter takes these and never the provenance beside
    /// them ([ADR-005]) — a file path that can influence output is a file path
    /// that can reach a golden file.
    ///
    /// [ADR-005]: ../../../docs/adr/005-provenance.md
    pub style: Style,
    /// Property name → where that property's value was decided. STY-008's
    /// inspector is a read of this.
    pub provenance: Provenance,
}

impl ResolvedStyle {
    /// Where one property came from, for the inspector.
    pub fn origin_of(&self, property: &str) -> Option<&Origin> {
        self.provenance.get(property)
    }
}

/// Every selector's finished appearance.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedStyles {
    entries: BTreeMap<StyleSelector, ResolvedStyle>,
}

impl ResolvedStyles {
    /// What an element looks like.
    ///
    /// A selector deeper than the built-in table goes — `\q7` — is not in the
    /// map, because the map cannot be infinite. It resolves to the deepest
    /// level that is, which is the same rule the cascade applies to a level it
    /// *does* know, and the reason a seventh-level poetry line is still
    /// indented.
    pub fn get(&self, selector: StyleSelector) -> ResolvedStyle {
        let mut step = Some(selector);
        while let Some(current) = step {
            if let Some(found) = self.entries.get(&current) {
                return found.clone();
            }
            step = current.shallower();
        }
        ResolvedStyle::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (StyleSelector, &ResolvedStyle)> {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Resolve the built-in sheet against a project's, if it has one.
///
/// Never fails, for the same reason settings resolution never fails: there is
/// always a built-in answer. What it produces is a list of diagnostics the
/// caller decides whether to block on.
pub fn resolve(project: Option<&ConfigDocument>, strict: bool) -> (ResolvedStyles, Diagnostics) {
    let builtin = style::builtin();

    // CFG-004's `strict` covers styles too. A publisher who asked to be
    // stopped by a settings key this release does not recognise did not mean
    // "except for the ones that decide what the page looks like".
    let severity = if strict {
        Severity::Error
    } else {
        Severity::Warning
    };

    let mut diagnostics = Diagnostics::new();
    let overrides = match project {
        Some(doc) => {
            let (sheet, d) = style::read(doc, severity);
            diagnostics.extend(d);
            sheet
        }
        None => StyleSheet::default(),
    };

    (
        resolve_sheets(&builtin, &overrides, &mut diagnostics),
        diagnostics,
    )
}

/// The cascade proper, over two sheets that have already been read.
pub fn resolve_sheets(
    builtin: &StyleSheet,
    overrides: &StyleSheet,
    diagnostics: &mut Diagnostics,
) -> ResolvedStyles {
    // Every selector this release knows, plus anything either sheet mentions
    // that `all()` does not — a level deeper than the built-in table goes.
    let mut wanted: BTreeSet<StyleSelector> = StyleSelector::all().into_iter().collect();
    wanted.extend(builtin.selectors());
    wanted.extend(overrides.selectors());

    let mut reported: BTreeSet<StyleSelector> = BTreeSet::new();
    let mut entries = BTreeMap::new();
    for selector in wanted {
        entries.insert(
            selector,
            resolve_one(selector, builtin, overrides, diagnostics, &mut reported),
        );
    }

    ResolvedStyles { entries }
}

/// Walk one selector's chain, nearest first.
fn resolve_one(
    selector: StyleSelector,
    builtin: &StyleSheet,
    overrides: &StyleSheet,
    diagnostics: &mut Diagnostics,
    reported: &mut BTreeSet<StyleSelector>,
) -> ResolvedStyle {
    let mut resolved = ResolvedStyle::default();
    // A path rather than a set: when the walk re-enters a selector, the loop
    // is the tail of the path from that point, which is both what the message
    // has to draw and what identifies the cycle.
    let mut path: Vec<StyleSelector> = Vec::new();
    let mut step = Some(selector);

    while let Some(current) = step {
        if let Some(at) = path.iter().position(|s| *s == current) {
            let loop_ = &path[at..];
            // One diagnostic per *cycle*, not per selector that can reach one.
            // Four levels of `q` and four of `qr` all walk into the same loop;
            // eight identical complaints about it would bury the one fact.
            let identity = loop_.iter().copied().min().expect("a loop has members");
            if reported.insert(identity) {
                diagnostics.push(cycle(loop_, overrides, builtin));
            }
            break;
        }
        path.push(current);

        // Project over built-in, at this link in the chain.
        for (name, sheet, from_file) in [(current, overrides, true), (current, builtin, false)]
            .map(|(sel, sheet, is_file)| (sel, sheet, is_file))
        {
            let Some(entry) = sheet.entry(name) else {
                continue;
            };
            take(&mut resolved, &entry.style, |property| {
                // Inherited only once the walk has left the selector that
                // was asked for; ADR-005 wants "why does this look like
                // this" answered by the inheritance where that is the
                // answer.
                if name != selector {
                    return Origin::Inherited { from: name };
                }
                match (from_file, entry.locations.get(property)) {
                    (true, Some(loc)) => Origin::File(loc.clone()),
                    _ => Origin::Builtin,
                }
            });
        }

        step = parent(current, overrides, builtin);
    }

    resolved
}

/// The parent of a selector: the one it names, or the level below it.
fn parent(
    selector: StyleSelector,
    overrides: &StyleSheet,
    builtin: &StyleSheet,
) -> Option<StyleSelector> {
    overrides
        .entry(selector)
        .and_then(|e| e.inherits)
        .or_else(|| builtin.entry(selector).and_then(|e| e.inherits))
        .or_else(|| selector.shallower())
}

/// Fill in every property this style sets that is not already decided.
///
/// Generated from the property list, so a property added to [`Style`] and to
/// `PROPERTIES` cannot be left out of the cascade — which would show as a
/// setting a publisher can write and the page ignores.
fn take(into: &mut ResolvedStyle, from: &Style, origin: impl Fn(&'static str) -> Origin) {
    macro_rules! cascade {
        ($($field:ident = $name:literal),* $(,)?) => {
            $(
                if into.style.$field.is_none() {
                    if let Some(value) = from.$field {
                        into.style.$field = Some(value);
                        into.provenance.record($name, origin($name));
                    }
                }
            )*
            // Keeps `PROPERTIES` and this macro from drifting apart: the two
            // lists are the same length only if every property is cascaded.
            const _: () = assert!(
                PROPERTIES.len() == [$($name),*].len(),
                "a property exists that the cascade does not carry",
            );
        };
    }

    cascade! {
        font_size = "font_size",
        weight = "weight",
        italic = "italic",
        smallcaps = "smallcaps",
        space_above = "space_above",
        space_below = "space_below",
        indent = "indent",
        raise = "raise",
        align = "align",
    }
}

/// One diagnostic naming the cycle, not a stack overflow.
///
/// `loop_` is the members in the order the walk met them, so the message draws
/// the circle and closes it on the one it came back to.
fn cycle(loop_: &[StyleSelector], overrides: &StyleSheet, builtin: &StyleSheet) -> Diagnostic {
    let mut rendered: Vec<String> = loop_.iter().map(|s| s.key()).collect();
    if let Some(first) = loop_.first() {
        rendered.push(first.key());
    }

    let mut d = Diagnostic::error(
        code::INHERITANCE_CYCLE,
        format!(
            "style inheritance goes in a circle: {}",
            rendered.join(" → ")
        ),
    )
    .help("remove one of the `inherits` keys in the loop");

    // Point at the first `inherits` in the loop that a publisher can actually
    // edit — a built-in one has no line in their file to go to.
    let written = loop_
        .iter()
        .find_map(|s| overrides.entry(*s).and_then(|e| e.inherits_at.clone()))
        .or_else(|| {
            loop_
                .iter()
                .find_map(|s| builtin.entry(*s).and_then(|e| e.inherits_at.clone()))
        });
    if let Some(loc) = written {
        d = d.at(loc);
    }
    d
}
