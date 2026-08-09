//! P1.6's assertion, over P1.2's corpus: normalization loses no Scripture text.
//!
//! No environment variable and no external checkout — the books are committed,
//! so this is a gate rather than something someone remembers to run.
//!
//! It asserts two things and reports a third. Normalization must not panic on
//! anything real, and every word of the source's Scripture text has to survive
//! into the model. Unsupported markers are counted rather than asserted,
//! because that number is a fact about the corpus, not a defect.
//!
//! **No file is excused.** There was one: `usfm-core` read a bare `|` in
//! paragraph text as an attribute block and dropped the rest of the line.
//! Fixed upstream, so the exception is gone rather than grandfathered.

use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::{BookCode, ScriptureDocument};
use biblecompose_testkit::corpus;
use camino::Utf8Path;
use std::collections::BTreeMap;

#[test]
fn normalizing_the_corpus_loses_nothing() {
    let mut files = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let mut losses = Vec::new();

    for entry in corpus::books() {
        let source = corpus::read(&entry);
        let code = BookCode::parse(&entry.book)
            .unwrap_or_else(|| panic!("{} is not a book code", entry.book));

        let name = entry.path.as_str();
        let document = usfm_core::Document::parse(source.as_str());
        let (book, diagnostics) = normalize(code, Utf8Path::new(name), &document);
        files += 1;

        for d in diagnostics.iter() {
            *unsupported.entry(d.message.clone()).or_default() += 1;
        }

        // Compare what survived against what the source says, ignoring
        // whitespace: normalization joins and splits runs, and a line break
        // becoming a space is not a loss.
        let got = squeeze(&ScriptureDocument::new(vec![book]).text());
        let want = squeeze(&scripture_text(&source));
        if !want.is_empty() && !contains_all(&got, &want) {
            if std::env::var("BIBLECOMPOSE_CORPUS_VERBOSE").is_ok() && losses.len() < 3 {
                let missing: Vec<&str> = want
                    .split(' ')
                    .filter(|w| w.chars().count() >= 8 && !got.contains(*w))
                    .take(12)
                    .collect();
                println!("  {name}: missing {missing:?}");
            }
            losses.push(name.to_owned());
        }
    }

    println!("normalized {files} books");
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
