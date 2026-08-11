//! Whether to hyphenate, and saying so when the answer is no (FONT-004).
//!
//! # What the spike found, and what it actually was
//!
//! [F-11] recorded a Tamil page broken as `ந-கரம்`, `வருபவர்-கள்` — Latin-style
//! hyphenation applied to a script that does not use it — and concluded that
//! "SILE has no Tamil patterns and does not say so", i.e. that asking for a
//! language it cannot hyphenate gets you *another language's* patterns.
//!
//! Measured again against the pinned backend, that inference is wrong and the
//! observation is right. SILE 0.15.13 ships `languages/ta/hyphens-tex.lua` —
//! Tamil patterns exist, they are auto-generated from TeX, and they fire. On
//! one book of Lamentations:
//!
//! | `project.language` | patterns shipped | hyphens drawn |
//! |---|---|---|
//! | `ta` | yes | **510** |
//! | `am` | no | 7 |
//! | `zz` (not a language) | no | 7 |
//! | `en` | yes | 7 |
//!
//! Seven is the number of hyphens in the source text. So a language with no
//! patterns gets *no* hyphenation rather than somebody else's, and English
//! patterns do not match Tamil letters. The defect is narrower and sharper
//! than the spike thought: **the backend hyphenates Tamil because it has Tamil
//! patterns**, and hyphenating Tamil is wrong however good the patterns are.
//!
//! # So the rule is about the script, not about the patterns
//!
//! A table of "languages the backend has patterns for" would have passed `ta`
//! straight through, which is the bug. What decides it is whether the script
//! is one that hyphenates at all — a typographic fact about writing systems,
//! not a judgement about SILE.
//!
//! The script is read from the text rather than from the language tag,
//! because the text is what gets set and a tag can be absent, wrong, or
//! describe a book that is mostly in another script anyway.
//!
//! [F-11]: ../../../spike/NOTES.md

use std::collections::BTreeMap;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics};
use biblecompose_scripture::ScriptureDocument;

/// Scripts in which words are not broken across lines.
///
/// Not a preference. Hyphenation is a convention of the Latin, Greek and
/// Cyrillic traditions and a few others; in these writing systems a hyphen
/// mid-word is an error, and a Bible carrying five hundred of them is a Bible
/// that will be sent back.
///
/// Ranges rather than names, because the question is "which of these
/// characters is the text made of" and Unicode blocks answer it directly.
const NON_HYPHENATING: [(&str, u32, u32); 21] = [
    ("Hebrew", 0x0590, 0x05FF),
    ("Arabic", 0x0600, 0x06FF),
    ("Syriac", 0x0700, 0x074F),
    ("Thaana", 0x0780, 0x07BF),
    ("Devanagari", 0x0900, 0x097F),
    ("Bengali", 0x0980, 0x09FF),
    ("Gurmukhi", 0x0A00, 0x0A7F),
    ("Gujarati", 0x0A80, 0x0AFF),
    ("Oriya", 0x0B00, 0x0B7F),
    ("Tamil", 0x0B80, 0x0BFF),
    ("Telugu", 0x0C00, 0x0C7F),
    ("Kannada", 0x0C80, 0x0CFF),
    ("Malayalam", 0x0D00, 0x0D7F),
    ("Sinhala", 0x0D80, 0x0DFF),
    ("Thai", 0x0E00, 0x0E7F),
    ("Lao", 0x0E80, 0x0EFF),
    ("Tibetan", 0x0F00, 0x0FFF),
    ("Myanmar", 0x1000, 0x109F),
    ("Ethiopic", 0x1200, 0x137F),
    ("Khmer", 0x1780, 0x17FF),
    ("Han", 0x3400, 0x9FFF),
];

/// What the backend will actually be asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyphenation {
    /// Whether the backend is allowed to hyphenate at all.
    pub enabled: bool,
    /// The language tag it is given. Unchanged — the language drives more than
    /// hyphenation, and rewriting it to hide a hyphenation decision would be
    /// solving one problem by lying about another.
    pub language: String,
}

/// The script most of the text is written in, if it is one that does not
/// hyphenate.
///
/// Most, not any: a Tamil Bible with an English title page is a Tamil Bible.
/// The threshold is a simple majority of the letters that belong to any script
/// named above or to none — anything less confident than that should leave the
/// decision alone.
pub fn non_hyphenating_script(doc: &ScriptureDocument) -> Option<&'static str> {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut total = 0usize;

    for (c, (count, _)) in crate::font::codepoints(doc) {
        if !c.is_alphabetic() {
            continue;
        }
        total += count;
        let point = c as u32;
        if let Some((name, _, _)) = NON_HYPHENATING
            .iter()
            .find(|(_, lo, hi)| (*lo..=*hi).contains(&point))
        {
            *counts.entry(name).or_default() += count;
        }
    }

    let (name, count) = counts.into_iter().max_by_key(|(_, n)| *n)?;
    (total > 0 && count * 2 > total).then_some(name)
}

/// Decide, and say why when the answer is no.
///
/// A diagnostic rather than silence, and information rather than a warning:
/// nothing is wrong with the project, and the outcome — a Tamil Bible without
/// hyphens — is the one the publisher wanted. What they would not want is to
/// find out from the page that a setting they turned on did nothing.
pub fn decide(
    language: &str,
    requested: bool,
    doc: &ScriptureDocument,
    diagnostics: &mut Diagnostics,
) -> Hyphenation {
    let language = language.trim().to_owned();

    if !requested {
        return Hyphenation {
            enabled: false,
            language,
        };
    }

    match non_hyphenating_script(doc) {
        Some(script) => {
            diagnostics.push(
                Diagnostic::info(
                    code::NO_HYPHENATION_PATTERNS,
                    format!("hyphenation is off: this Scripture is set in {script}, which does not break words across lines"),
                )
                .help(
                    "the backend ships patterns that would hyphenate it anyway — one book of \
                     Tamil came out with five hundred hyphens in it. Turn off \
                     `typography.hyphenation` to stop this being mentioned",
                ),
            );
            Hyphenation {
                enabled: false,
                language,
            }
        }
        None => Hyphenation {
            enabled: true,
            language,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biblecompose_scripture::fixtures;

    #[test]
    fn latin_scripture_hyphenates() {
        let mut d = Diagnostics::new();
        let plan = decide("en", true, &fixtures::kitchen_sink(), &mut d);
        assert!(plan.enabled);
        assert!(d.is_empty(), "nothing to say about the ordinary case");
    }

    #[test]
    fn asking_for_no_hyphenation_says_nothing() {
        let mut d = Diagnostics::new();
        let plan = decide("en", false, &fixtures::kitchen_sink(), &mut d);
        assert!(!plan.enabled);
        assert!(
            d.is_empty(),
            "the publisher already knows: they turned it off"
        );
    }

    /// The language is passed through whatever is decided. It drives more than
    /// hyphenation, and rewriting it here would hide the decision in a value
    /// something else reads.
    #[test]
    fn the_language_is_never_rewritten() {
        let mut d = Diagnostics::new();
        assert_eq!(
            decide("ta", true, &fixtures::kitchen_sink(), &mut d).language,
            "ta"
        );
        assert_eq!(
            decide("ta", false, &fixtures::kitchen_sink(), &mut d).language,
            "ta"
        );
    }

    #[test]
    fn a_document_with_no_letters_is_left_alone() {
        let mut d = Diagnostics::new();
        let empty = ScriptureDocument::new(Vec::new());
        assert!(decide("en", true, &empty, &mut d).enabled);
    }
}
