//! Finding the Scripture files in a project folder, and working out which
//! book each one is.
//!
//! PRJ-002 through PRJ-006. Two of those are the interesting ones:
//!
//! **A book is identified by its `\id` marker, not its filename** (PRJ-003).
//! Filenames in real projects are `41MATengwebp.usfm`, `04-LEVheb.usfm`,
//! `Matthew.SFM`, or whatever the translation team's tooling produced. Reading
//! the marker is the only identification that survives a rename, and a rename
//! is exactly what happens when a file is emailed around.
//!
//! **Duplicates block the build** (PRJ-004). Two files claiming `\id MAT` is
//! not a situation to resolve by picking one — either could be the current
//! draft, and silently choosing is how a publisher discovers at proof stage
//! that they typeset last month's Matthew.
//!
//! Nothing here writes. BLD-004 and NFR-007 are architectural rather than
//! promised: this module opens files for reading and the crate has no code
//! path that opens a `.usfm` for writing at all.

use std::collections::BTreeMap;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, SourceLoc};
use biblecompose_scripture::canon::BookCode;
use camino::{Utf8Path, Utf8PathBuf};

/// Extensions a Scripture file may have. `.SFM` in any case, because Paratext
/// wrote them uppercase for years and those files are still in circulation.
const SCRIPTURE_EXTENSIONS: [&str; 2] = ["usfm", "sfm"];

/// Directory names never descended into.
///
/// PRJ-006. `output/` and `.biblecompose/` hold what a *previous* build
/// produced; discovering them would feed the build its own output. The others
/// are here because a project folder is usually also a working directory, and
/// a `.git` object store full of loose files is a slow way to find nothing.
const EXCLUDED_DIRS: [&str; 5] = ["output", ".biblecompose", ".git", "node_modules", "target"];

/// One Scripture file, identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredBook {
    pub book: BookCode,
    pub path: Utf8PathBuf,
    /// The file's contents, read once during discovery.
    ///
    /// Held rather than re-read: identification has already paid for the read,
    /// and reading a second time invites the file changing in between — which
    /// would mean the book we validated is not the book we typeset.
    pub source: String,
}

/// What a scan of a project folder found.
#[derive(Debug, Default)]
pub struct Discovery {
    /// Identified books, in canonical order.
    pub books: Vec<DiscoveredBook>,
    pub diagnostics: Diagnostics,
}

impl Discovery {
    /// Whether anything found makes a build impossible.
    pub fn blocked(&self) -> bool {
        self.diagnostics.blocking().next().is_some()
    }
}

/// Scan `root` recursively for Scripture files and identify each one.
pub fn discover(root: &Utf8Path) -> Discovery {
    let mut diagnostics = Diagnostics::new();
    let mut files = Vec::new();
    walk(root, &mut files, &mut diagnostics);

    // By path, so a directory listing arriving in a different order on a
    // different filesystem cannot change which duplicate is reported first.
    files.sort();

    // Every path claiming each book, so a duplicate can name all of them
    // rather than just the second one — "MAT is declared twice" is not
    // actionable, and "these two files both declare MAT" is.
    let mut claims: BTreeMap<BookCode, Vec<(Utf8PathBuf, String)>> = BTreeMap::new();

    for path in files {
        let source = match std::fs::read_to_string(path.as_std_path()) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(
                    Diagnostic::error(code::UNREADABLE_FILE, "could not read this Scripture file")
                        .at(SourceLoc::file(path.clone()))
                        .detail(e.to_string()),
                );
                continue;
            }
        };

        match identify(&source) {
            Some(book) => claims.entry(book).or_default().push((path, source)),
            None => diagnostics.push(
                Diagnostic::error(
                    code::UNIDENTIFIED_BOOK,
                    "this file has no usable \\id marker, so it cannot be placed in the canon",
                )
                .at(SourceLoc::file(path))
                .help("add an \\id line naming the book, for example \\id MAT"),
            ),
        }
    }

    let mut books = Vec::new();
    for (book, mut found) in claims {
        if found.len() > 1 {
            let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
            diagnostics.push(
                Diagnostic::error(
                    code::DUPLICATE_BOOK_ID,
                    format!("{} is declared by {} files", book.as_str(), found.len()),
                )
                .at(SourceLoc::file(found[0].0.clone()))
                .help(
                    "remove or move all but one; the build cannot choose, and choosing \
                     wrongly would typeset the wrong draft",
                )
                .detail(paths.join("\n")),
            );
            continue;
        }
        let (path, source) = found.remove(0);
        books.push(DiscoveredBook { book, path, source });
    }

    // PRJ-005 is satisfied by saying nothing: a project of one book is a
    // project. Only an empty one is a problem.
    if books.is_empty() && diagnostics.blocking().next().is_none() {
        diagnostics.push(
            Diagnostic::error(
                code::NO_BOOKS_FOUND,
                format!("no .usfm or .sfm files under {root}"),
            )
            .at(SourceLoc::file(root.to_owned())),
        );
    }

    books.sort_by_key(|b| b.book.order());

    Discovery { books, diagnostics }
}

/// The book a file declares, from its `\id` marker.
///
/// PRJ-003. The marker's first token is the book code; anything after it is a
/// free-text description the specification allows and translators use.
///
/// Deliberately a scan of the leading lines rather than a parse: discovery
/// runs over every file in the project before any of them is known to be
/// wanted, and parsing a 2 MB book to learn three letters is work thrown away.
/// The parse happens later, once, on the files that survive.
pub fn identify(source: &str) -> Option<BookCode> {
    // A byte-order mark is common and is not part of the marker.
    let source = source.strip_prefix('\u{FEFF}').unwrap_or(source);

    for line in source.lines() {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        let Some(rest) = line.strip_prefix("\\id") else {
            // Content before `\id` is a diagnostic upstream reports
            // (`USFM-E019`), not a reason to stop looking: the file may still
            // say which book it is, and refusing to place it would turn one
            // recoverable problem into two.
            continue;
        };
        // `\identification` must not read as `\id entification`.
        if !rest.is_empty() && !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        if let Some(token) = rest.split_whitespace().next() {
            if let Some(book) = BookCode::parse(token) {
                return Some(book);
            }
        }
    }
    None
}

fn walk(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>, diagnostics: &mut Diagnostics) {
    let entries = match std::fs::read_dir(dir.as_std_path()) {
        Ok(e) => e,
        Err(e) => {
            diagnostics.push(
                Diagnostic::error(code::UNREADABLE_FILE, "could not read this directory")
                    .at(SourceLoc::file(dir.to_owned()))
                    .detail(e.to_string()),
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            // A non-UTF-8 path cannot be reported through `SourceLoc`, which
            // is `Utf8PathBuf`, and inventing a lossy name for a diagnostic
            // would point at a file that does not exist. Skipping is honest;
            // it is also unreachable for anything a translation tool writes.
            continue;
        };

        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let name = path.file_name().unwrap_or_default();
            if EXCLUDED_DIRS.contains(&name) {
                continue;
            }
            walk(&path, out, diagnostics);
            continue;
        }

        let is_scripture = path.extension().is_some_and(|e| {
            SCRIPTURE_EXTENSIONS
                .iter()
                .any(|s| e.eq_ignore_ascii_case(s))
        });
        if is_scripture {
            out.push(path);
        }
    }
}
