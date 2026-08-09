//! The composition corpus: whole books, pinned, and checked against itself.
//!
//! P1.2. `usfm-core`'s corpus exists to exercise a *parser*, so it is chosen
//! for coverage per file and includes fragments and front matter. A compositor
//! needs something different: **whole books**, because the failures that
//! matter here — a verse stranded at a column foot, a running head that stops
//! updating, a note that collides with the one below it — only appear over
//! pages of continuous text.
//!
//! Thirteen books, about 1.2 MB, chosen by set cover so that every required
//! script and feature class appears at least once in as few files as possible.
//! Small enough that cloning stays pleasant; a corpus nobody wants to clone is
//! a corpus nobody runs.
//!
//! # What the manifest is for
//!
//! `sha256` makes drift detectable. If a corpus file changes silently, every
//! downstream failure becomes ambiguous — and these files are the input to the
//! text-loss assertion (P1.6), which is meaningless if the input can move.
//!
//! The recorded `scripts` and `features` are **not** taken on trust:
//! [`verify`] re-derives both from the bytes and compares. A manifest that
//! describes what it wishes were true is worse than no manifest.

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

/// Scripts the compositor must be exercised against.
///
/// Chosen for what they demand of shaping rather than for speaker numbers:
/// combining marks, conjunct formation, visual reordering, right-to-left, and
/// the absence of word spacing.
pub const REQUIRED_SCRIPTS: &[&str] = &[
    "Latin",
    "Greek",
    "Cyrillic",
    "Hebrew",
    "Arabic",
    "Devanagari",
    "Tamil",
    "Bengali",
    "Thai",
    "Khmer",
    "Myanmar",
    "Han",
];

/// Feature classes that occur in complete published books.
///
/// `milestones`, `sidebars` and custom `\z` markers are deliberately absent:
/// no whole book in the pool contains one. They are covered by `usfm-core`'s
/// authored fixtures, which is the right place for them — this corpus is for
/// what real books do.
pub const REQUIRED_FEATURES: &[&str] = &[
    "alt_numbering",
    "attributes",
    "char_styles",
    "figures",
    "introductions",
    "lists",
    "nested_markers",
    "notes",
    "poetry",
    "tables",
    "titles",
    "verse_ranges",
];

/// A script counts for a file when it is at least this share of its letters,
/// so a stray character in a quotation cannot claim coverage.
const SCRIPT_SHARE: f64 = 0.01;

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub path: Utf8PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub book: String,
    pub chapters: u32,
    pub verses: u32,
    #[serde(default)]
    pub translation: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub copyright: String,
    #[serde(default)]
    pub redistributable: String,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub traits: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Manifest {
    #[serde(default, rename = "file")]
    files: Vec<Entry>,
}

/// Where the corpus lives, relative to the workspace.
pub fn root() -> Utf8PathBuf {
    let manifest_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Utf8Path::parent)
        .expect("the crate sits two levels below the workspace root")
        .join("corpus")
}

/// Every book in the manifest.
pub fn books() -> Vec<Entry> {
    let path = root().join("manifest.toml");
    let text = std::fs::read_to_string(path.as_std_path())
        .unwrap_or_else(|e| panic!("reading {path}: {e}"));
    let manifest: Manifest =
        toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"));
    manifest.files
}

/// A book's source text.
pub fn read(entry: &Entry) -> String {
    let path = root().join(&entry.path);
    std::fs::read_to_string(path.as_std_path()).unwrap_or_else(|e| panic!("reading {path}: {e}"))
}

/// Everything wrong with the corpus, as a list rather than the first failure —
/// a checksum mismatch and a coverage hole are different problems and both are
/// worth seeing at once.
pub fn verify() -> Vec<String> {
    let mut problems = Vec::new();
    let entries = books();

    if entries.is_empty() {
        return vec!["the manifest lists no files".to_owned()];
    }

    let mut scripts_seen: BTreeSet<String> = BTreeSet::new();
    let mut features_seen: BTreeSet<String> = BTreeSet::new();
    let mut listed: BTreeSet<Utf8PathBuf> = BTreeSet::new();

    for e in &entries {
        listed.insert(e.path.clone());
        let path = root().join(&e.path);

        let Ok(raw) = std::fs::read(path.as_std_path()) else {
            problems.push(format!(
                "{}: listed in the manifest but not on disk",
                e.path
            ));
            continue;
        };

        let actual = sha256_hex(&raw);
        if actual != e.sha256 {
            problems.push(format!(
                "{}: sha256 mismatch\n    expected {}\n    actual   {}",
                e.path, e.sha256, actual
            ));
            continue;
        }
        if raw.len() as u64 != e.bytes {
            problems.push(format!(
                "{}: recorded {} bytes, found {}",
                e.path,
                e.bytes,
                raw.len()
            ));
        }

        // Provenance. The redistribution flag is the whole basis for these
        // files being in a public repository, so a missing one is an error
        // rather than an untidiness.
        if e.source.is_empty() {
            problems.push(format!("{}: no source recorded", e.path));
        }
        if e.copyright.is_empty() {
            problems.push(format!("{}: no copyright line recorded", e.path));
        }
        if !e.redistributable.eq_ignore_ascii_case("true") {
            problems.push(format!(
                "{}: redistributable is {:?} — this file must not be committed",
                e.path, e.redistributable
            ));
        }

        let text = String::from_utf8_lossy(&raw);

        // A whole book, not a fragment. This is the property that distinguishes
        // this corpus from `usfm-core`'s.
        if e.chapters == 0 || e.verses < 20 {
            problems.push(format!(
                "{}: {} chapters and {} verses is not a whole book",
                e.path, e.chapters, e.verses
            ));
        }

        let found_scripts = detect_scripts(&text);
        let found_features = detect_features(&text);

        for claimed in &e.scripts {
            if REQUIRED_SCRIPTS.contains(&claimed.as_str()) && !found_scripts.contains(claimed) {
                problems.push(format!(
                    "{}: manifest claims script {claimed}, which is not in the file",
                    e.path
                ));
            }
        }
        for claimed in &e.features {
            if REQUIRED_FEATURES.contains(&claimed.as_str()) && !found_features.contains(claimed) {
                problems.push(format!(
                    "{}: manifest claims feature {claimed}, which is not in the file",
                    e.path
                ));
            }
        }

        scripts_seen.extend(found_scripts);
        features_seen.extend(found_features);
    }

    // Anything on disk the manifest does not know about — an unpinned file is
    // one that can change without anyone noticing.
    let dir = root().join("books");
    if let Ok(entries) = std::fs::read_dir(dir.as_std_path()) {
        for f in entries.flatten() {
            let Ok(p) = Utf8PathBuf::from_path_buf(f.path()) else {
                continue;
            };
            let ext = p.extension().unwrap_or_default().to_ascii_lowercase();
            if ext != "usfm" && ext != "sfm" {
                continue;
            }
            let rel = Utf8PathBuf::from(format!("books/{}", p.file_name().unwrap_or_default()));
            if !listed.contains(&rel) {
                problems.push(format!("{rel}: present on disk but not in the manifest"));
            }
        }
    }

    for s in REQUIRED_SCRIPTS {
        if !scripts_seen.contains(*s) {
            problems.push(format!("no book covers the script {s}"));
        }
    }
    for f in REQUIRED_FEATURES {
        if !features_seen.contains(*f) {
            problems.push(format!("no book covers the feature class {f}"));
        }
    }

    problems
}

/// Scripts making up at least [`SCRIPT_SHARE`] of a text's letters.
pub fn detect_scripts(text: &str) -> BTreeSet<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut total = 0usize;

    for ch in text.chars() {
        let Some(script) = script_of(ch) else {
            continue;
        };
        *counts.entry(script).or_default() += 1;
        total += 1;
    }

    if total == 0 {
        return BTreeSet::new();
    }
    counts
        .into_iter()
        .filter(|(_, n)| (*n as f64) / (total as f64) >= SCRIPT_SHARE)
        .map(|(s, _)| s.to_owned())
        .collect()
}

/// By Unicode block. Enough for the twelve scripts named above, and
/// deliberately not a general answer — a full implementation is a dependency,
/// and this only has to tell these twelve apart.
fn script_of(ch: char) -> Option<&'static str> {
    let c = ch as u32;
    Some(match c {
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F => "Latin",
        0x0370..=0x03FF | 0x1F00..=0x1FFF => "Greek",
        0x0400..=0x052F => "Cyrillic",
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => "Hebrew",
        0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => "Arabic",
        0x0900..=0x097F => "Devanagari",
        0x0980..=0x09FF => "Bengali",
        0x0B80..=0x0BFF => "Tamil",
        0x0E00..=0x0E7F => "Thai",
        0x1780..=0x17FF => "Khmer",
        0x1000..=0x109F => "Myanmar",
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => "Han",
        _ => return None,
    })
}

/// Feature classes present, by the markers that introduce them.
pub fn detect_features(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut add = |f: &str| {
        found.insert(f.to_owned());
    };

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("\\q") {
            add("poetry");
        }
        if trimmed.starts_with("\\li") {
            add("lists");
        }
        if trimmed.starts_with("\\tr") {
            add("tables");
        }
        if trimmed.starts_with("\\mt") || trimmed.starts_with("\\s") {
            add("titles");
        }
        if trimmed.starts_with("\\ip")
            || trimmed.starts_with("\\is")
            || trimmed.starts_with("\\io")
            || trimmed.starts_with("\\imt")
            || trimmed.starts_with("\\iot")
        {
            add("introductions");
        }
        // `\v 1-2`, a verse spanning more than one number.
        if let Some(rest) = trimmed.strip_prefix("\\v ") {
            let number = rest.split_whitespace().next().unwrap_or_default();
            if number.contains('-') {
                add("verse_ranges");
            }
        }
    }

    if text.contains("\\f ") || text.contains("\\fe ") || text.contains("\\x ") {
        add("notes");
    }
    if text.contains("\\fig") {
        add("figures");
    }
    if text.contains("\\+") {
        add("nested_markers");
    }
    if text.contains("\\va")
        || text.contains("\\vp")
        || text.contains("\\ca")
        || text.contains("\\cp")
    {
        add("alt_numbering");
    }
    for marker in [
        "\\add", "\\nd", "\\wj", "\\bd", "\\it", "\\sc", "\\w ", "\\tl", "\\k ", "\\em", "\\qt",
    ] {
        if text.contains(marker) {
            add("char_styles");
            break;
        }
    }
    // An attribute block: a pipe followed by `key="value"`.
    if text.match_indices('|').any(|(i, _)| {
        text[i..]
            .split('\\')
            .next()
            .unwrap_or_default()
            .contains("=\"")
    }) {
        add("attributes");
    }

    found
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
