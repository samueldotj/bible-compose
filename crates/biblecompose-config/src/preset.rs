//! The editions, written out (P6.2).
//!
//! A preset is a named set of settings a publisher can start from: the
//! conventional two-column Bible, the same in one column, a reader's edition,
//! large print, a reference edition, a study Bible, a pocket Bible and a
//! journaling Bible. Each is a TOML fragment compiled into the binary and
//! listed here with a name and a sentence.
//!
//! # Applying one writes into the project's file
//!
//! Rather than becoming a layer of the cascade, which is the other obvious
//! design and is worse here for three reasons.
//!
//! * **Provenance stays honest.** [ADR-005] answers "where did this value come
//!   from" with a file and a line. A value that came from a preset came from
//!   the publisher's own settings file, because that is where it now is, and
//!   the inspector can point at it.
//! * **It is legible.** A preset the publisher can read, edit and disagree with
//!   one line at a time is a starting point; a preset that resolves invisibly
//!   underneath them is a thing to be reverse-engineered.
//! * **Nothing new enters the resolver.** The cascade has two layers and a
//!   third would have to be threaded through every question about precedence.
//!
//! What is given up is a preset that keeps evolving with the application. That
//! is the right thing to give up: a publisher's page should not change because
//! they updated the software.
//!
//! [ADR-005]: ../../../docs/adr/005-provenance.md

use biblecompose_diagnostics::{Diagnostic, Diagnostics};

use crate::edit::{SettingValue, TomlFile};
use crate::ConfigDocument;

/// One of the editions this release ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// The stable identifier, used in the settings file and on the wire.
    pub id: &'static str,
    /// What it is called. English; a locale translates it by id.
    pub title: &'static str,
    /// One sentence, for the person choosing.
    pub description: &'static str,
    /// The TOML itself.
    pub toml: &'static str,
}

/// Every preset, in the order they are offered.
///
/// Two-column first because it is what most Bibles are and what the built-in
/// defaults already produce, so it is the least surprising thing to land on;
/// single column beside it as the one-step departure; then the editions that
/// are each a decision about what the book is for.
pub const ALL: &[Preset] = &[
    Preset {
        id: "two-column",
        title: "Standard two-column",
        description: "The conventional Bible page: two columns, verse numbers, \
                      footnotes and cross-references at the foot.",
        toml: include_str!("../presets/two-column.toml"),
    },
    Preset {
        id: "single-column",
        title: "Single column",
        description: "The conventional page in one column: slightly larger                       type on a longer line, with the numbers and apparatus                       kept.",
        toml: include_str!("../presets/single-column.toml"),
    },
    Preset {
        id: "reader",
        title: "Reader's edition",
        description: "One column, set like a novel: no verse numbers and no \
                      apparatus, so the text reads without interruption.",
        toml: include_str!("../presets/reader.toml"),
    },
    Preset {
        id: "large-print",
        title: "Large print",
        description: "14pt in one column on a larger page, set ragged right \
                      to avoid the wide word spacing justification would need.",
        toml: include_str!("../presets/large-print.toml"),
    },
    Preset {
        id: "reference",
        title: "Reference",
        description: "Two dense columns on a hand-sized page, every verse                       numbered and linkable, the head giving the page's first                       and last reference.",
        toml: include_str!("../presets/reference.toml"),
    },
    Preset {
        id: "study",
        title: "Study Bible",
        description: "A larger page with everything on: introductions,                       outlines, headings, footnotes and cross-references.",
        toml: include_str!("../presets/study.toml"),
    },
    Preset {
        id: "pocket",
        title: "Pocket Bible",
        description: "A page that fits a coat pocket: small type in one                       column, tight margins, footnotes kept and                       cross-references left out.",
        toml: include_str!("../presets/pocket.toml"),
    },
    Preset {
        id: "journaling",
        title: "Journaling Bible",
        description: "One column beside a two-inch outer margin left empty                       for the reader's own notes; no apparatus at the foot.",
        toml: include_str!("../presets/journaling.toml"),
    },
];

/// The preset with this id.
pub fn by_id(id: &str) -> Option<&'static Preset> {
    ALL.iter().find(|p| p.id == id)
}

impl Preset {
    /// The keys this preset sets, with their values, in file order.
    ///
    /// Parsed rather than pattern-matched, so a preset is checked by the same
    /// reader that checks a publisher's file and cannot contain a key the
    /// schema has never heard of. A test below runs every one of them through
    /// resolution and fails if any produces so much as a warning.
    pub fn settings(&self) -> Result<Vec<(String, SettingValue)>, Diagnostic> {
        let doc = ConfigDocument::parse(format!("{}.toml", self.id), self.toml.to_owned())?;
        let root = doc.root().table()?;
        let mut out = Vec::new();
        // Tables all the way down — `[headers.left_page]` is a table under a
        // table — and a leaf wherever one is found. A leaf that is neither a
        // table nor a value the editor takes is dropped here and caught by
        // the test below, which resolves every preset.
        fn walk(
            table: &crate::document::Table<'_>,
            prefix: &str,
            out: &mut Vec<(String, SettingValue)>,
        ) {
            for name in table.names() {
                let Some(node) = table.get(name) else {
                    continue;
                };
                let key = if prefix.is_empty() {
                    name.to_owned()
                } else {
                    format!("{prefix}.{name}")
                };
                if let Ok(inner) = node.table() {
                    walk(&inner, &key, out);
                } else if let Some(value) = value_of(&node) {
                    out.push((key, value));
                }
            }
        }
        walk(&root, "", &mut out);
        Ok(out)
    }
}

/// One leaf, as the editor takes it.
///
/// Boolean before integer, because `true` is not a number and asking in the
/// other order would not notice. Anything else — a date, a nested table — is
/// `None` and is caught by the test that every preset round-trips.
fn value_of(node: &crate::document::Node<'_>) -> Option<SettingValue> {
    if let Ok(v) = node.boolean() {
        return Some(SettingValue::Bool(*v));
    }
    if let Ok(v) = node.integer() {
        return Some(SettingValue::Int(*v));
    }
    if let Ok(v) = node.number() {
        return Some(SettingValue::Float(*v));
    }
    if let Ok(v) = node.string() {
        return Some(SettingValue::Str(v.to_string()));
    }
    let (items, problems) = node.string_array();
    if problems.is_empty() && !items.is_empty() {
        return Some(SettingValue::List(
            items.into_iter().map(|i| i.to_string()).collect(),
        ));
    }
    None
}

/// Write a preset's settings into a project's file (CFG-006).
///
/// Through [`TomlFile`], so a file that already had comments in it still has
/// them afterwards, and a key the preset does not mention is left exactly as
/// it was. Choosing "Reader's edition" changes what a reader's edition is
/// about and does not reset the publication's name or its language.
pub fn apply(file: &mut TomlFile, preset: &Preset) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    match preset.settings() {
        Ok(settings) => {
            for (key, value) in settings {
                file.set(&key, value);
            }
        }
        // Unreachable in a shipped build — the test below parses every preset
        // — and a panic in a publisher's window is not the way to say so.
        Err(d) => diagnostics.push(d),
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings;

    /// **Every preset resolves cleanly**, which is the whole of what makes
    /// them safe to ship: a preset with a misspelled key would configure
    /// nothing and say nothing.
    #[test]
    fn every_preset_is_valid_configuration() {
        for preset in ALL {
            let toml = format!("schema_version = 1\n{}", preset.toml);
            let doc = ConfigDocument::parse(format!("{}.toml", preset.id), toml)
                .unwrap_or_else(|d| panic!("{} does not parse: {d}", preset.id));
            let (_, diagnostics) = settings::resolve(Some(&doc));
            assert!(
                diagnostics.is_empty(),
                "{} produced {:?}",
                preset.id,
                diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
            );
        }
    }

    /// And each says something different from the others. Two presets that
    /// resolve to the same page are one preset with two names.
    #[test]
    fn the_presets_differ() {
        let resolved: Vec<_> = ALL
            .iter()
            .map(|p| {
                let toml = format!("schema_version = 1\n{}", p.toml);
                let doc = ConfigDocument::parse("p.toml", toml).expect("valid");
                settings::resolve(Some(&doc)).0
            })
            .collect();

        for (i, a) in resolved.iter().enumerate() {
            for (j, b) in resolved.iter().enumerate().skip(i + 1) {
                assert_ne!(
                    (
                        a.page.columns.to_string(),
                        a.typography.font_size.to_string(),
                        a.page.size.to_string()
                    ),
                    (
                        b.page.columns.to_string(),
                        b.typography.font_size.to_string(),
                        b.page.size.to_string()
                    ),
                    "{} and {} are the same page",
                    ALL[i].id,
                    ALL[j].id
                );
            }
        }
    }

    /// Ids are what the settings file and the wire carry, so they have to be
    /// unique and stable.
    #[test]
    fn ids_are_unique() {
        let mut seen: Vec<&str> = ALL.iter().map(|p| p.id).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two presets share an id");
        assert!(ALL.iter().all(|p| by_id(p.id).is_some()));
        assert!(by_id("no-such-preset").is_none());
    }

    /// **Applying one leaves everything it did not mention alone** (CFG-006).
    #[test]
    fn applying_a_preset_keeps_the_rest_of_the_file() {
        let existing = "\
# A publisher's own note.
schema_version = 1

[project]
name = \"My Bible\"
language = \"ta\"

[page]
columns = 2
";
        let doc = ConfigDocument::parse("biblecompose.toml", existing.to_owned()).expect("valid");
        let mut file = TomlFile::new(doc);
        let diagnostics = apply(&mut file, by_id("reader").expect("the reader preset"));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let after = file.to_toml();
        assert!(after.contains("A publisher's own note"), "the comment went");
        assert!(after.contains("My Bible"), "the name went");
        assert!(after.contains("language = \"ta\""), "the language went");
        assert!(after.contains("columns = 1"), "the preset did not apply");
    }
}
