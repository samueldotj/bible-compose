//! Reading USFM, and carrying the parser's findings across the seam.
//!
//! [ADR-001](../../../docs/adr/001-usfm-core.md) draws the line: `usfm-core`
//! answers *what does the file say*, this crate answers *what is the
//! publication*. Nothing composition-specific belongs upstream, and no parser
//! lives here.
//!
//! This module is only the crossing. Normalization — USJ to
//! [`ScriptureDocument`](crate::ScriptureDocument) — is P1.5.

use biblecompose_diagnostics::{code, Code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};
use usfm_core::{Diagnostic as UpstreamDiagnostic, DiagnosticCode as Upstream, Document};

/// One parsed USFM file, with its diagnostics already in BibleCompose terms.
pub struct ParsedFile {
    pub path: Utf8PathBuf,
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse one file's contents.
///
/// The source is passed in rather than read here: discovery owns the
/// filesystem (P1.3), and a parser that opens files is a parser that can be
/// asked to open the wrong one.
pub fn parse(path: impl Into<Utf8PathBuf>, source: impl Into<String>) -> ParsedFile {
    let path = path.into();
    let document = Document::parse(source);
    let mut diagnostics = Vec::new();
    let mut gaps = Vec::new();
    let mut gaps_at = None;
    for d in document.diagnostics() {
        match gap(d) {
            Some(missing) => {
                gaps.push(missing);
                gaps_at.get_or_insert((diagnostics.len(), d));
            }
            None => diagnostics.push(translate(&path, &document, d)),
        }
    }
    if let Some((at, first)) = gaps_at {
        let mut summary = translate(&path, &document, first);
        summary.message = missing_verses(&book_name(&path, &document), gaps);
        diagnostics.insert(at, summary);
    }

    ParsedFile {
        path,
        document,
        diagnostics,
    }
}

/// The chapter and verse of one upstream verse-gap note.
///
/// `usfm-core` reports a gap one verse at a time, which suits an editor that
/// underlines each place and does not suit a problems list: one stray
/// `\v 2017` in 1 Thessalonians was 1,998 identical-looking notes. So the
/// gaps in a file are gathered here into one note per book (see
/// [`missing_verses`]). The numbers come from upstream's message, whose
/// wording a test pins; one this does not recognise is passed through
/// unmerged rather than lost.
fn gap(d: &UpstreamDiagnostic) -> Option<(u16, u16)> {
    if d.code != Upstream::VerseGap {
        return None;
    }
    let (chapter, verse) = d
        .message
        .strip_prefix("chapter ")?
        .split_once(" has no verse ")?;
    Some((chapter.parse().ok()?, verse.parse().ok()?))
}

/// The book's code from its `\id`, or the file's name if it has none.
fn book_name(path: &Utf8Path, document: &Document) -> String {
    document
        .content()
        .iter()
        .find(|node| node.kind == usfm_core::NodeKind::Book)
        .and_then(|node| node.attribute("code"))
        .map(str::to_string)
        .unwrap_or_else(|| path.file_name().unwrap_or(path.as_str()).to_string())
}

/// `1TH is missing 1,998 verses: 2:18–2015, 3:5`. Runs become ranges, in
/// chapter order.
fn missing_verses(book: &str, mut gaps: Vec<(u16, u16)>) -> String {
    gaps.sort_unstable();
    gaps.dedup();
    let count = gaps.len();

    let mut runs: Vec<(u16, u16, u16)> = Vec::new();
    for (chapter, verse) in gaps {
        match runs.last_mut() {
            Some((c, _, end)) if *c == chapter && verse == *end + 1 => *end = verse,
            _ => runs.push((chapter, verse, verse)),
        }
    }
    let list: Vec<String> = runs
        .iter()
        .map(|&(chapter, start, end)| {
            if start == end {
                format!("{chapter}:{start}")
            } else {
                format!("{chapter}:{start}\u{2013}{end}")
            }
        })
        .collect();

    let what = if count == 1 {
        "1 verse".to_string()
    } else {
        format!("{} verses", thousands(count))
    };
    format!("{book} is missing {what}: {}", list.join(", "))
}

fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Whether a diagnostic presumes the book has chapters.
///
/// The parser reads every file as a book of Scripture, and a book of
/// Scripture with no `\c` is broken — so a front-matter file, which has
/// divisions and no chapters, is reported as missing one, as an error, and
/// an error blocks the build. The parser is right about what it was told
/// and wrong about what the file is; the caller knows which, and drops
/// these for a peripheral book. Nothing else is dropped: an unclosed marker
/// in a copyright notice is as wrong as one in Genesis.
pub fn presumes_chapters(code: Code) -> bool {
    code == code::MISSING_CHAPTER_MARKER || code == code::BODY_PARAGRAPH_BEFORE_CHAPTER
}

/// One upstream diagnostic, in BibleCompose's vocabulary.
///
/// The message and the code are carried across untouched. What is added is
/// where it happened: upstream reports a byte span, and a publisher reading a
/// build log wants `MAT.usfm:42:7`.
fn translate(path: &Utf8Path, document: &Document, d: &UpstreamDiagnostic) -> Diagnostic {
    let c = code_for(d.code);
    let message = d.message.clone();

    let diagnostic = match d.severity {
        usfm_core::Severity::Error => Diagnostic::error(c, message),
        usfm_core::Severity::Warning => Diagnostic::warning(c, message),
        usfm_core::Severity::Information => Diagnostic::info(c, message),
    };

    match document.line_col(d.span.start) {
        Some(at) => diagnostic.at(SourceLoc::at(path, at.line, at.column)),
        // Only reachable if the span is not this document's, which the
        // document owning both sides makes impossible. Name the file anyway
        // rather than dropping the location entirely.
        None => diagnostic.at(SourceLoc::file(path)),
    }
}

/// The upstream code, unchanged.
///
/// ADR-001 requires that a file reported one way in the editor is not reported
/// another way in the compositor, so this is a rename and never a re-coding —
/// `USFM-W001` stays `USFM-W001`.
///
/// **The match is exhaustive on purpose.** A code added to `usfm-core` stops
/// this build until someone decides how BibleCompose reports it. The
/// alternative — a catch-all arm — would turn a new upstream diagnostic into a
/// silently mislabelled one, which is the failure this seam exists to prevent.
fn code_for(c: Upstream) -> Code {
    match c {
        Upstream::UnknownMarker => code::UNKNOWN_MARKER,
        Upstream::DeprecatedMarker => code::DEPRECATED_MARKER,
        Upstream::UnclosedMarker => code::UNCLOSED_MARKER,
        Upstream::StrayCloseMarker => code::STRAY_CLOSE_MARKER,
        Upstream::MisnestedMarker => code::MISNESTED_MARKER,
        Upstream::MissingNestingPrefix => code::MISSING_NESTING_PREFIX,
        Upstream::ImplicitClose => code::IMPLICIT_CLOSE,
        Upstream::UnclosedNote => code::UNCLOSED_NOTE,
        Upstream::UnclosedAtEof => code::UNCLOSED_AT_EOF,
        Upstream::InvalidChapterSequence => code::INVALID_CHAPTER_SEQUENCE,
        Upstream::InvalidVerseSequence => code::INVALID_VERSE_SEQUENCE,
        Upstream::DuplicateChapter => code::DUPLICATE_CHAPTER,
        Upstream::DuplicateId => code::DUPLICATE_ID,
        Upstream::MissingIdMarker => code::MISSING_ID_MARKER,
        Upstream::InvalidBookCode => code::INVALID_BOOK_CODE,
        Upstream::NoteSubmarkerOutsideNote => code::NOTE_SUBMARKER_OUTSIDE_NOTE,
        Upstream::TextBeforeId => code::TEXT_BEFORE_ID,
        Upstream::NonAsciiVerseDigits => code::NON_ASCII_VERSE_DIGITS,
        Upstream::HeaderAfterBody => code::HEADER_AFTER_BODY,
        Upstream::MilestoneMismatch => code::MILESTONE_MISMATCH,
        Upstream::MixedNormalization => code::MIXED_NORMALIZATION,
        Upstream::JoinerInMarkerName => code::JOINER_IN_MARKER_NAME,
        Upstream::JoinerAtMarkerBoundary => code::JOINER_AT_MARKER_BOUNDARY,
        Upstream::InvalidAttributes => code::INVALID_ATTRIBUTES,
        Upstream::MissingChapterNumber => code::MISSING_CHAPTER_NUMBER,
        Upstream::MissingVerseNumber => code::MISSING_VERSE_NUMBER,
        Upstream::VerseOutsideParagraph => code::VERSE_OUTSIDE_PARAGRAPH,
        Upstream::MissingChapterMarker => code::MISSING_CHAPTER_MARKER,
        Upstream::CharCrossesVerseBoundary => code::CHAR_CROSSES_VERSE_BOUNDARY,
        Upstream::EmptyFigure => code::EMPTY_FIGURE,
        Upstream::UnquotedAttributeValue => code::UNQUOTED_ATTRIBUTE_VALUE,
        Upstream::MissingRequiredAttribute => code::MISSING_REQUIRED_ATTRIBUTE,
        Upstream::DefaultAttributeNotDefined => code::DEFAULT_ATTRIBUTE_NOT_DEFINED,
        Upstream::BodyParagraphBeforeChapter => code::BODY_PARAGRAPH_BEFORE_CHAPTER,
        Upstream::NonEmptyBlankLine => code::NON_EMPTY_BLANK_LINE,
        Upstream::LeadingZeros => code::LEADING_ZEROS,
        Upstream::EmptyWordMarker => code::EMPTY_WORD_MARKER,
        Upstream::MissingMilestoneSelfClose => code::MISSING_MILESTONE_SELF_CLOSE,
        Upstream::InvalidTableColumnSequence => code::INVALID_TABLE_COLUMN_SEQUENCE,
        Upstream::MarkerNewerThanDocument => code::MARKER_NEWER_THAN_DOCUMENT,
        Upstream::LegacyFigureSyntax => code::LEGACY_FIGURE_SYNTAX,
        Upstream::DuplicateVerse => code::DUPLICATE_VERSE,
        Upstream::VerseGap => code::VERSE_GAP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biblecompose_diagnostics::Severity;

    /// Both authorities mint `USFM-*`. They stay disjoint because every
    /// upstream code carries a severity letter and every code of ours is bare
    /// digits — asserted rather than trusted, because the day they collide,
    /// two different conditions share one identifier and suppression settings
    /// start hiding the wrong thing.
    #[test]
    fn the_two_usfm_code_namespaces_cannot_collide() {
        for c in Upstream::ALL {
            let ours = code_for(*c);
            assert_eq!(ours.as_str(), c.as_str(), "the code must pass through");

            let suffix = ours.as_str().strip_prefix("USFM-").expect("USFM- prefix");
            assert!(
                suffix.starts_with(|ch: char| ch.is_ascii_alphabetic()),
                "{} has no severity letter, so it could collide with one of ours",
                ours.as_str()
            );
        }
        assert!(code::UNSUPPORTED_MARKER
            .as_str()
            .strip_prefix("USFM-")
            .expect("USFM- prefix")
            .starts_with(|ch: char| ch.is_ascii_digit()));
    }

    fn gap_notes(source: &str) -> Vec<String> {
        parse("1TH.usfm", source)
            .diagnostics
            .into_iter()
            .filter(|d| d.code == code::VERSE_GAP)
            .map(|d| d.message)
            .collect()
    }

    /// Pins the wording [`gap`] reads. If `usfm-core` rewords it, this fails
    /// rather than the notes quietly going back to one per verse.
    #[test]
    fn upstream_still_words_a_gap_the_way_it_is_read() {
        let doc = Document::parse("\\id 1TH\n\\c 1\n\\p\n\\v 1 a\n\\v 3 c\n");
        let found: Vec<_> = doc
            .diagnostics()
            .iter()
            .filter(|d| d.code == Upstream::VerseGap)
            .map(gap)
            .collect();
        assert_eq!(found, [Some((1, 2))]);
    }

    #[test]
    fn a_book_s_missing_verses_are_one_note() {
        let notes = gap_notes(
            "\\id 1TH\n\\c 1\n\\p\n\\v 1 a\n\\v 5 b\n\\v 7 c\n\\c 2\n\\p\n\\v 1 d\n\\v 2017 e\n",
        );
        assert_eq!(
            notes,
            ["1TH is missing 2,019 verses: 1:2\u{2013}4, 1:6, 2:2\u{2013}2016"]
        );
    }

    #[test]
    fn one_missing_verse_is_said_in_the_singular() {
        assert_eq!(
            gap_notes("\\id JUD\n\\c 1\n\\p\n\\v 1 a\n\\v 3 b\n"),
            ["JUD is missing 1 verse: 1:2"]
        );
    }

    #[test]
    fn the_note_sits_at_the_first_gap_and_keeps_its_place() {
        let parsed = parse(
            "1TH.usfm",
            "\\id 1TH\n\\c 1\n\\p\n\\v 1 a\n\\v 3 b\n\\v 3 again\n\\c 2\n\\p\n\\v 2 c\n",
        );
        let gaps: Vec<_> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.code == code::VERSE_GAP)
            .collect();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, Severity::Info);
        assert!(gaps[0].location.as_ref().and_then(|at| at.line).is_some());
        // Other diagnostics are untouched: the duplicate is still reported.
        assert!(parsed
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "USFM-E042"));
    }

    #[test]
    fn a_book_with_no_gaps_has_no_gap_note() {
        assert!(gap_notes("\\id 1TH\n\\c 1\n\\p\n\\v 1 a\n\\v 2 b\n").is_empty());
    }

    /// Every mirrored code has to be in `code::ALL`, or a build log carrying
    /// one will not deserialize.
    #[test]
    fn every_upstream_code_round_trips_through_serde() {
        for c in Upstream::ALL {
            let ours = code_for(*c);
            let json = serde_json::to_string(&ours).expect("serialize");
            let back: Code = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, ours);
        }
    }

    #[test]
    fn a_diagnostic_arrives_with_a_line_and_column() {
        // `\c` with no number, on the third line.
        let parsed = parse("MAT.usfm", "\\id MAT\n\\p\n\\c\n");
        let found = parsed
            .diagnostics
            .iter()
            .find(|d| d.code == code::MISSING_CHAPTER_NUMBER)
            .expect("the missing chapter number should be reported");

        let at = found.location.as_ref().expect("a location");
        assert_eq!(at.path, "MAT.usfm");
        assert_eq!(at.line, Some(3), "reported at {at}");
    }

    #[test]
    fn a_clean_file_reports_nothing() {
        let parsed = parse(
            "MAT.usfm",
            "\\id MAT\n\\c 1\n\\p\n\\v 1 In the beginning.\n",
        );
        let errors: Vec<_> = parsed
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
