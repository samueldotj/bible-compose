//! The editions, written out (P6.2).
//!
//! A preset is a named set of settings a publisher can start from: the
//! conventional two-column Bible, the same in one column, a reader's edition,
//! large print, a reference edition, a study Bible, a pocket Bible, a
//! journaling Bible, a modern edition and an illuminated one. Each is a TOML
//! fragment compiled into the binary and listed here with a name and a
//! sentence.
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
//! # And its look into the project's style sheet
//!
//! Some editions are a page *and* a look: the reader's edition's centred
//! psalm number, the reference edition's red letter. Those are styles, and a
//! project's styles live in `styles.toml` beside its settings, so a preset
//! may carry a second fragment that is written there the same way — key by
//! key, through [`TomlFile`], leaving every style it does not mention alone.
//! It names no font: a preset is chosen before the Scripture's script is
//! known to it, and a Latin face written into a Tamil project would be a
//! build that cannot set its own text.
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
    /// The look, for the project's `styles.toml`, when the edition has one.
    pub styles: Option<&'static str>,
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
        styles: Some(include_str!("../presets/two-column.styles.toml")),
    },
    Preset {
        id: "single-column",
        title: "Single column",
        description: "The conventional page in one column: slightly larger                       type on a longer line, with the numbers and apparatus                       kept.",
        toml: include_str!("../presets/single-column.toml"),
        styles: None,
    },
    Preset {
        id: "reader",
        title: "Reader's edition",
        description: "One column, set like a novel: no verse numbers and no \
                      apparatus, so the text reads without interruption.",
        toml: include_str!("../presets/reader.toml"),
        styles: Some(include_str!("../presets/reader.styles.toml")),
    },
    Preset {
        id: "large-print",
        title: "Large print",
        description: "14pt in one column on a larger page, set ragged right \
                      to avoid the wide word spacing justification would need.",
        toml: include_str!("../presets/large-print.toml"),
        styles: None,
    },
    Preset {
        id: "reference",
        title: "Reference",
        description: "Two dense columns on a hand-sized page, every verse                       numbered and linkable, the head giving the page's first                       and last reference.",
        toml: include_str!("../presets/reference.toml"),
        styles: Some(include_str!("../presets/reference.styles.toml")),
    },
    Preset {
        id: "study",
        title: "Study Bible",
        description: "A larger page with everything on: introductions,                       outlines, headings, footnotes and cross-references.",
        toml: include_str!("../presets/study.toml"),
        styles: Some(include_str!("../presets/study.styles.toml")),
    },
    Preset {
        id: "pocket",
        title: "Pocket Bible",
        description: "A page that fits a coat pocket: small type in one                       column, tight margins, footnotes kept and                       cross-references left out.",
        toml: include_str!("../presets/pocket.toml"),
        styles: None,
    },
    Preset {
        id: "journaling",
        title: "Journaling Bible",
        description: "One column beside a two-inch outer margin left empty                       for the reader's own notes; no apparatus at the foot.",
        toml: include_str!("../presets/journaling.toml"),
        styles: Some(include_str!("../presets/journaling.styles.toml")),
    },
    Preset {
        id: "modern",
        title: "Modern",
        description: "One column set ragged right, with strong headings and                       the chapter and verse numbers in a single accent colour.",
        toml: include_str!("../presets/modern.toml"),
        styles: Some(include_str!("../presets/modern.styles.toml")),
    },
    Preset {
        id: "illuminated",
        title: "Illuminated",
        description: "A gift edition in one column: a boxed chapter number,                       a dropped initial, and headings and numbers in red.",
        toml: include_str!("../presets/illuminated.toml"),
        styles: Some(include_str!("../presets/illuminated.styles.toml")),
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
        leaves(&format!("{}.toml", self.id), self.toml)
    }

    /// The style properties this preset sets, keyed `selector.property` —
    /// `heading.s1.align` — in file order. Empty for an edition with no look
    /// of its own.
    pub fn style_settings(&self) -> Result<Vec<(String, SettingValue)>, Diagnostic> {
        match self.styles {
            Some(styles) => leaves(&format!("{}.styles.toml", self.id), styles),
            None => Ok(Vec::new()),
        }
    }
}

/// Every leaf of a fragment, with its dotted key, in file order.
fn leaves(name: &str, toml: &str) -> Result<Vec<(String, SettingValue)>, Diagnostic> {
    let doc = ConfigDocument::parse(name.to_owned(), toml.to_owned())?;
    let root = doc.root().table()?;
    let mut out = Vec::new();
    // Tables all the way down — `[headers.left_page]` is a table under a
    // table — and a leaf wherever one is found. A leaf that is neither a
    // table nor a value the editor takes is dropped here and caught by
    // the tests below, which resolve every preset.
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
/// them afterwards. Choosing "Reader's edition" changes what a reader's
/// edition is about and does not reset the publication's name, its language
/// or its books — no edition names those.
///
/// **A key another edition sets and this one does not is reset**, so the
/// built-in value applies again. Without that, switching from one edition to
/// another kept whatever the first set and the second left unsaid — the
/// two-column edition's diamond under the title, say, on a study Bible
/// whose design has none — and the page looked like neither. A key no
/// edition sets is the publisher's own, and is left exactly as it was.
pub fn apply(file: &mut TomlFile, preset: &Preset) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    match preset.settings() {
        Ok(settings) => {
            clear_others(file, &settings, |p| p.settings().ok());
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

/// Write a preset's look into a project's style sheet.
///
/// The same bargain as [`apply`], on the other file: every style property
/// the preset names is set, every one another edition's look sets and this
/// one does not is reset, and the publisher's own are left as they had them.
/// A preset with no look clears the looks of the others, so choosing it
/// gives the plain page it describes.
pub fn apply_styles(file: &mut TomlFile, preset: &Preset) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    match preset.style_settings() {
        Ok(styles) => {
            clear_others(file, &styles, |p| p.style_settings().ok());
            for (key, value) in styles {
                file.set(&key, value);
            }
        }
        Err(d) => diagnostics.push(d),
    }
    diagnostics
}

/// Reset every key another edition set that `keeping` does not name.
///
/// Only where the file still holds that edition's value: a key that holds
/// something else was the publisher's choice, whichever edition also uses
/// it, and is theirs to keep.
fn clear_others(
    file: &mut TomlFile,
    keeping: &[(String, SettingValue)],
    keys_of: impl Fn(&Preset) -> Option<Vec<(String, SettingValue)>>,
) {
    let kept: std::collections::BTreeSet<&str> = keeping.iter().map(|(k, _)| k.as_str()).collect();
    let set_by_an_edition: Vec<String> = ALL
        .iter()
        .filter_map(&keys_of)
        .flatten()
        .filter(|(k, v)| !kept.contains(k.as_str()) && file.holds(k, v))
        .map(|(k, _)| k)
        .collect();
    for key in set_by_an_edition {
        file.reset(&key);
    }
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

    /// **Every preset's look resolves cleanly**, strictly: a misspelled
    /// selector or property in a shipped style sheet would be a look that
    /// silently does not happen.
    #[test]
    fn every_preset_look_is_valid_styling() {
        for preset in ALL {
            let Some(styles) = preset.styles else {
                continue;
            };
            let doc =
                ConfigDocument::parse(format!("{}.styles.toml", preset.id), styles.to_owned())
                    .unwrap_or_else(|d| panic!("{} styles do not parse: {d}", preset.id));
            let (_, diagnostics) = crate::cascade::resolve(Some(&doc), true);
            assert!(
                diagnostics.is_empty(),
                "{} styles produced {:?}",
                preset.id,
                diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
            );
            assert!(
                !preset.style_settings().expect("parses").is_empty(),
                "{} has an empty look",
                preset.id
            );
        }
    }

    /// A preset names only a font the application ships, so choosing it can
    /// never ask for a face the machine does not have. Which faces those are
    /// is read off the bundled folder rather than listed twice.
    #[test]
    fn a_preset_names_only_a_font_the_application_ships() {
        let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fonts");
        let shipped: Vec<String> = std::fs::read_dir(&fonts)
            .expect("the bundled fonts")
            .filter_map(|e| e.ok())
            .map(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .replace([' ', '-', '_'], "")
            })
            .collect();
        let ships = |family: &str| {
            let squashed = family.to_lowercase().replace(' ', "");
            shipped.iter().any(|f| f.starts_with(&squashed))
        };
        for preset in ALL {
            let mut named: Vec<(String, SettingValue)> = preset
                .style_settings()
                .expect("parses")
                .into_iter()
                .filter(|(k, _)| k.ends_with("font_family"))
                .collect();
            named.extend(
                preset
                    .settings()
                    .expect("parses")
                    .into_iter()
                    .filter(|(k, _)| k == "typography.font_family"),
            );
            for (key, value) in named {
                let SettingValue::Str(family) = value else {
                    panic!("{} sets `{key}` to something other than a name", preset.id)
                };
                assert!(
                    ships(&family),
                    "{} names `{family}` at `{key}`, which the application does not ship",
                    preset.id
                );
            }
        }
    }

    /// Applying a look keeps the publisher's own styles.
    #[test]
    fn applying_a_look_keeps_the_rest_of_the_sheet() {
        let existing = "# My own heading.
[heading.s1]
color = \"#123456\"

[poetry.q1]
indent = \"20pt\"
";
        let doc = ConfigDocument::parse("styles.toml", existing.to_owned()).expect("valid");
        let mut file = TomlFile::new(doc);
        let diagnostics = apply_styles(&mut file, by_id("illuminated").expect("illuminated"));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let after = file.to_toml();
        assert!(after.contains("My own heading"), "the comment went");
        assert!(
            after.contains("indent = \"20pt\""),
            "the poetry indent went"
        );
        assert!(after.contains("own_line = true"), "the look did not apply");
    }

    /// Switching editions leaves nothing of the first one's look behind: the
    /// two-column title diamond and italic headings are not the study
    /// Bible's, and its own values all arrive.
    #[test]
    fn switching_editions_clears_what_the_first_set_and_the_second_does_not() {
        let mut file = TomlFile::new(
            ConfigDocument::parse(
                "biblecompose.toml",
                "schema_version = 1\n[books]\ninclude = [\"GEN\"]\n".to_owned(),
            )
            .expect("valid"),
        );
        let mut sheet = TomlFile::new(
            ConfigDocument::parse("styles.toml", "[poetry.q3]\nindent = \"40pt\"\n".to_owned())
                .expect("valid"),
        );
        let two = by_id("two-column").expect("two-column");
        let study = by_id("study").expect("study");
        assert!(apply(&mut file, two).is_empty());
        assert!(apply_styles(&mut sheet, two).is_empty());
        assert!(file.to_toml().contains("title_ornament"));
        assert!(apply(&mut file, study).is_empty());
        assert!(apply_styles(&mut sheet, study).is_empty());

        let settings = file.to_toml();
        assert!(!settings.contains("title_ornament"), "{settings}");
        assert!(!settings.contains("column_rule"), "{settings}");
        assert!(settings.contains("include = [\"GEN\"]"), "the books went");
        assert!(
            settings.contains("font_family = \"Literata\""),
            "{settings}"
        );

        let styles = sheet.to_toml();
        assert!(
            !styles.contains("italic = true\nalign = \"start\""),
            "{styles}"
        );
        assert!(
            styles.contains("indent = \"40pt\""),
            "the publisher's own style went"
        );
        for (key, _) in study.style_settings().expect("study's look") {
            let leaf = key.rsplit('.').next().expect("a leaf");
            assert!(styles.contains(leaf), "{key} did not arrive");
        }
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
