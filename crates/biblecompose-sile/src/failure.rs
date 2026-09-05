//! What a backend failure means, when it means anything (SILE-007, DIA-005).
//!
//! SILE fails by writing a Lua error to stderr and exiting non-zero. Handed
//! straight through, that is `exited with status 1` plus forty lines of stack
//! trace naming files inside the typesetter — which tells a publisher nothing
//! they can act on and tells whoever they report it to almost as little.
//!
//! So the tail of the log is matched against the failures that have actually
//! been seen, and each becomes a diagnostic that says three things: what
//! happened, whose fault it is, and what to do. **Whose fault it is** is the
//! part worth insisting on. Most of these are defects in *this application* —
//! a malformed document, an option the class does not know, a Lua error inside
//! the class — and a publisher who is told to check their Scripture for one of
//! those will spend an afternoon on it.
//!
//! # Two rules
//!
//! **The raw text is never lost.** Every mapped diagnostic carries the tail of
//! the log as its detail, which the panel keeps collapsed (DIA-005). A mapping
//! that hid the evidence would be worse than no mapping, because the evidence
//! is what a bug report is made of.
//!
//! **An unmapped failure still surfaces.** The table is a list of things that
//! have been seen, not a filter: anything it does not recognise falls through
//! to the general non-zero-exit diagnostic with the same raw tail attached. A
//! table that swallowed what it did not know would turn every new failure into
//! silence.

use biblecompose_diagnostics::{code, Code, Diagnostic};

/// How much of the log a diagnostic carries.
///
/// The tail rather than the head: SILE's stack traces put the error first and
/// then unwind, but everything before it is progress output — page numbers and
/// a version banner — and the last few lines are where the failure is.
const TAIL: usize = 24;

/// One recognised failure.
struct Known {
    /// Matched case-insensitively against the log, as a plain substring. Not a
    /// regular expression: every one of these is a literal SILE writes, and a
    /// pattern language would invite matching things that only look similar.
    needle: &'static str,
    code: Code,
    message: &'static str,
    help: &'static str,
}

/// The table.
///
/// Ordered, and the order matters: a Lua error inside the class also matches
/// the generic `runtime error`, so the specific entries come first.
const KNOWN: &[Known] = &[
    Known {
        // Seen for real: one vertical tab in a source file, which XML 1.0
        // cannot carry at all. Fixed at the emitter, and still worth mapping —
        // if it ever happens again the message should say what it is.
        needle: "not well-formed",
        code: code::MALFORMED_DOCUMENT,
        message: "the document sent to the typesetter is not valid XML",
        help: "this is a defect in BibleCompose rather than in your Scripture — \
               please report it with the backend log",
    },
    Known {
        // Seen for real: a stale class in the runtime cache, shadowing the
        // one the application ships.
        needle: "undeclared class option",
        code: code::CLASS_VERSION_MISMATCH,
        message: "the typesetting class and this application are different versions",
        help: "reinstall BibleCompose — the class it ships with and the one it \
               found are not the same release",
    },
    Known {
        needle: "could not find requested font",
        code: code::FONT_UNAVAILABLE,
        message: "the typesetter could not load a font the styles asked for",
        help: "check the font family in Typography and in any style that sets \
               one of its own",
    },
    Known {
        needle: "couldn't find face",
        code: code::FONT_UNAVAILABLE,
        message: "the typesetter could not load a font the styles asked for",
        help: "check the font family in Typography and in any style that sets \
               one of its own",
    },
    Known {
        needle: "can't find frame",
        code: code::CLASS_DEFECT,
        message: "the page layout the class built does not have the frame it asked for",
        help: "this is a defect in BibleCompose — please report it with the \
               page settings and the backend log",
    },
    Known {
        needle: "queues are not empty",
        code: code::CLASS_DEFECT,
        message: "the typesetter finished with content it had nowhere to put",
        help: "this is a defect in BibleCompose — please report it with the \
               backend log",
    },
    Known {
        needle: "not enough memory",
        code: code::BACKEND_EXHAUSTED,
        message: "the typesetter ran out of memory",
        help: "build fewer books at a time: untick some on the Scripture tab",
    },
    Known {
        // Last, because a Lua error is how *every* SILE failure arrives and
        // the entries above are all more specific than it.
        needle: "runtime error",
        code: code::CLASS_DEFECT,
        message: "the typesetting class failed while composing the document",
        help: "this is a defect in BibleCompose rather than in your Scripture — \
               please report it with the backend log",
    },
];

/// The diagnostic this log deserves, if the table recognises it.
///
/// `None` means the caller's own general failure — which still carries
/// [`tail`], so nothing is lost either way.
pub fn classify(log: &str) -> Option<Diagnostic> {
    let haystack = log.to_lowercase();
    KNOWN.iter().find(|k| haystack.contains(k.needle)).map(|k| {
        Diagnostic::error(k.code, k.message)
            .help(k.help)
            .detail(tail(log))
    })
}

/// The last few lines, which is where a Lua failure says what went wrong.
pub fn tail(log: &str) -> String {
    let lines: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = lines.len().saturating_sub(TAIL);
    lines[from..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_malformed_document_blames_the_application_and_not_the_scripture() {
        let d = classify("SILE v0.15\n! not well-formed (invalid token)\n")
            .expect("this is in the table");
        assert_eq!(d.code, code::MALFORMED_DOCUMENT);
        assert!(d
            .help
            .as_deref()
            .unwrap_or_default()
            .contains("defect in BibleCompose"));
    }

    #[test]
    fn a_stale_class_is_named_as_a_version_mismatch() {
        let d = classify("! Attempted to set an undeclared class option 'fontfamily'")
            .expect("this is in the table");
        assert_eq!(d.code, code::CLASS_VERSION_MISMATCH);
    }

    /// **The specific entries win.** Every SILE failure is also a Lua runtime
    /// error, so a table that matched the general case first would map all of
    /// them to "the class failed" and none of them usefully.
    #[test]
    fn a_more_specific_match_beats_the_general_one() {
        let log = "Error: runtime error: inputters/xml.lua:90:\n! not well-formed";
        let d = classify(log).expect("this is in the table");
        assert_eq!(
            d.code,
            code::MALFORMED_DOCUMENT,
            "matched the generic runtime error instead"
        );
    }

    /// **And an unrecognised failure is not swallowed.** The table says what it
    /// knows; it does not decide what is worth reporting.
    #[test]
    fn a_failure_nobody_has_seen_before_is_not_recognised() {
        assert!(classify("! Something entirely new went wrong").is_none());
    }

    /// The evidence survives the mapping (DIA-005).
    #[test]
    fn the_raw_text_comes_with_it() {
        let d = classify("! not well-formed\nat line 42 of the document\n").expect("mapped");
        let detail = d.detail.as_deref().unwrap_or_default();
        assert!(
            detail.contains("line 42"),
            "the evidence is gone: {detail:?}"
        );
    }

    /// A long log is cut from the end, because that is where SILE puts the
    /// failure — and blank lines are dropped so the cut is worth its length.
    #[test]
    fn the_tail_is_the_end_and_not_the_beginning() {
        let mut log = String::new();
        for i in 0..200 {
            log.push_str(&format!("line {i}\n\n"));
        }
        let tail = tail(&log);
        assert!(tail.contains("line 199"));
        assert!(!tail.contains("line 100"));
        assert_eq!(tail.lines().count(), TAIL);
    }
}
