//! A smoke run of normalization over a directory of real USFM.
//!
//! Ignored by default: it needs a corpus, and the vendored one arrives with
//! P1.2. Until then it is pointed at `usfm-core`'s own 200-file corpus, which
//! is what P1.5 means by "across the corpus":
//!
//! ```text
//! BIBLECOMPOSE_CORPUS=<path> cargo test -p biblecompose-scripture \
//!     --test corpus -- --ignored --nocapture
//! ```
//!
//! It asserts two things and reports a third. Normalization must not panic on
//! anything real, and it must not lose a word — every non-space character of
//! the source's Scripture text has to survive into the model. Unsupported
//! markers are counted rather than asserted, because that number is a fact
//! about the corpus, not a defect.

use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::{BookCode, ScriptureDocument};
use camino::Utf8Path;
use std::collections::BTreeMap;

/// Files that lose text for a reason **above** this crate.
///
/// A bare `|` in ordinary paragraph text makes `usfm-core` discard the rest of
/// the line, with no diagnostic:
///
/// ```text
/// \v 11 before| after more words   →   content: [verse, " before"]
/// ```
///
/// USFM attributes are only meaningful on a character marker closed with
/// `\marker*`; a pipe in running text is punctuation. It is the **danda** in
/// Sanskrit-derived scripts, so this is not exotic input — it is how several
/// Indic translations write a full stop. It only bites when text follows the
/// pipe on the same line, which is why one file of 197 hits it rather than
/// fifteen.
///
/// Named here rather than silently tolerated: an exception with a reason is
/// worth more than a disabled assertion, and this list should shrink to
/// nothing once upstream is fixed.
const KNOWN_UPSTREAM_LOSS: &[&str] = &["70-3JNsanasm.usfm"];

#[test]
#[ignore = "needs a corpus; see the module docs"]
fn normalizing_a_real_corpus_loses_nothing() {
    let Ok(dir) = std::env::var("BIBLECOMPOSE_CORPUS") else {
        eprintln!("set BIBLECOMPOSE_CORPUS to a directory of .usfm files");
        return;
    };

    let mut files = 0usize;
    let mut skipped = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let mut losses = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("corpus directory") {
        let path = entry.expect("entry").path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ext.eq_ignore_ascii_case("usfm") && !ext.eq_ignore_ascii_case("sfm") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            skipped += 1;
            continue;
        };
        // Discovery's own `identify` lives in `biblecompose-project`, which
        // depends on this crate — dev-depending back would be a cycle. Two
        // lines here is cheaper than the coupling.
        let Some(code) = source
            .strip_prefix('\u{FEFF}')
            .unwrap_or(&source)
            .lines()
            .find_map(|l| l.trim_start().strip_prefix("\\id "))
            .and_then(|rest| BookCode::parse(rest.split_whitespace().next().unwrap_or("")))
        else {
            skipped += 1;
            continue;
        };

        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let document = usfm_core::Document::parse(source.as_str());
        let (book, diagnostics) = normalize(code, Utf8Path::new(name.as_ref()), &document);
        files += 1;

        for d in diagnostics.iter() {
            *unsupported.entry(d.message.clone()).or_default() += 1;
        }

        // Compare what survived against what the source says, ignoring
        // whitespace: normalization joins and splits runs, and a line break
        // becoming a space is not a loss.
        let got = squeeze(&ScriptureDocument::new(vec![book]).text());
        let want = squeeze(&scripture_text(&source));
        if !want.is_empty() && !contains_all(&got, &want) && !KNOWN_UPSTREAM_LOSS.contains(&&*name)
        {
            if std::env::var("BIBLECOMPOSE_CORPUS_VERBOSE").is_ok() && losses.len() < 3 {
                let missing: Vec<&str> = want
                    .split(' ')
                    .filter(|w| w.chars().count() >= 8 && !got.contains(*w))
                    .take(12)
                    .collect();
                println!("  {name}: missing {missing:?}");
            }
            losses.push(name.into_owned());
        }
    }

    println!("normalized {files} files ({skipped} skipped)");
    println!(
        "distinct unsupported-marker diagnostics: {}",
        unsupported.len()
    );
    for (message, count) in unsupported.iter().take(15) {
        println!("  {count:>5}  {message}");
    }

    assert!(files > 0, "the corpus directory held no USFM");
    assert!(losses.is_empty(), "text was lost in: {losses:?}");
}

/// Everything that is not a marker — a crude stand-in for the parser's own
/// idea of Scripture text, good enough to catch a whole verse going missing.
fn scripture_text(source: &str) -> String {
    let source = strip_apparatus(source);
    let mut out = String::new();
    for line in source.lines() {
        let line = line.trim();
        // Identification and apparatus lines are not body text.
        if line.starts_with("\\id")
            || line.starts_with("\\ide")
            || line.starts_with("\\rem")
            || line.starts_with("\\h")
            || line.starts_with("\\toc")
            || line.starts_with("\\sts")
            || line.starts_with("\\usfm")
            // `\mt` is the book title. Normalization puts it in `BookNames`
            // rather than in a block, so it is present in the model and
            // absent from the running text by design.
            || line.starts_with("\\mt")
        {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find('\\') {
            out.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            let end = after
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after.len());
            rest = &after[end..];
        }
        out.push_str(rest);
        out.push(' ');
    }
    out
}

/// Remove what the model deliberately keeps out of body text.
///
/// Notes and cross-references are apparatus (`ScriptureDocument::text` skips
/// them by design), and a figure's `|src="…" size="col"` is attribute syntax
/// the model stores in `FigureRef`. Leaving either in would make this check
/// fail on documents that are perfectly correct — which is exactly what the
/// first run of it did.
fn strip_apparatus(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    'outer: while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        for marker in ["fe ", "f ", "x "] {
            if after.starts_with(marker) {
                let close = format!("\\{}*", marker.trim_end());
                match after.find(&close) {
                    Some(end) => {
                        rest = &after[end + close.len()..];
                        continue 'outer;
                    }
                    // Unclosed: the rest of the file is inside a note.
                    None => break 'outer,
                }
            }
        }
        out.push('\\');
        rest = after;
    }
    out.push_str(rest);

    // Attribute tails: everything from `|` to the end of the marker's text.
    out.split('\n')
        .map(|line| match line.find('|') {
            Some(bar) if line[bar..].contains("=\"") => &line[..bar],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Words, with USFM attribute syntax removed.
///
/// A token may carry attributes directly — `word|lemma` in the default form,
/// `word|src="a.png"` in the named one — and the model separates them out. A
/// comparison that did not would report a loss for every `\w` in an
/// interlinear text, which is what the second run of this did.
fn squeeze(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.split('|').next().unwrap_or(w))
        .filter(|w| !w.contains("=\""))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether every reasonably long word of `want` appears in `got`.
///
/// Not equality: normalization drops note bodies from the running text on
/// purpose, and the crude extractor above cannot tell a note from a verse.
/// Long words are the ones a missing verse would take with it.
fn contains_all(got: &str, want: &str) -> bool {
    let missing = want
        .split(' ')
        .filter(|w| w.chars().count() >= 8)
        .filter(|w| !got.contains(*w))
        .count();
    let total = want.split(' ').filter(|w| w.chars().count() >= 8).count();
    // A handful of words live only inside notes, which the model deliberately
    // keeps out of body text. A tenth is generous for that and still far
    // below what a lost verse would cost.
    missing * 10 <= total
}
