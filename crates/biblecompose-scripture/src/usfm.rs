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
    let diagnostics = document
        .diagnostics()
        .iter()
        .map(|d| translate(&path, &document, d))
        .collect();

    ParsedFile {
        path,
        document,
        diagnostics,
    }
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
