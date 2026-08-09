//! A project folder becoming a `ScriptureDocument`.
//!
//! P1.7's other half. Everything below this was built and tested in isolation
//! — discovery (P1.3), the canon plan (P1.4), normalization (P1.5) — and this
//! is the first thing that runs them in order against a real folder.
//!
//! Orchestration lives here rather than in any of those crates because none of
//! them may know about the others: `biblecompose-project` does not know what a
//! publication is, and `biblecompose-scripture` does not know what a folder
//! is. [ARCHITECTURE §2](../../../docs/ARCHITECTURE.md) puts the arrow from
//! `biblecompose-app` to both of them and nowhere else.

use biblecompose_diagnostics::{Diagnostic, Diagnostics};
use biblecompose_project::discover;
use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::plan::BookPlan;
use biblecompose_scripture::{BookSource, ScriptureDocument};
use camino::Utf8Path;

/// What a project folder holds, as a publication.
pub struct Loaded {
    pub document: ScriptureDocument,
    pub diagnostics: Diagnostics,
}

impl Loaded {
    /// Whether anything found makes a build impossible.
    pub fn blocked(&self) -> bool {
        self.diagnostics.blocking().next().is_some()
    }
}

/// Read a project folder into a publication, applying `plan`.
///
/// **Every stage runs even when an earlier one produced errors** (DIA-002). A
/// publisher fixing a folder wants the whole list, not the first item — so a
/// duplicate book code does not stop the other books from being parsed and
/// reported on.
pub fn load(root: &Utf8Path, plan: &BookPlan) -> Loaded {
    let found = discover(root);
    let mut diagnostics = found.diagnostics;

    // Selection before parsing: a book the project excludes should not spend
    // time being parsed, and should not contribute diagnostics about a file
    // nobody asked to publish.
    let selected = plan.arrange(found.books, |b| b.book);

    let present = selected.iter().map(|b| b.book).collect();
    for absent in plan.configured_but_absent(&present) {
        diagnostics.push(
            Diagnostic::warning(
                biblecompose_diagnostics::code::UNKNOWN_BOOK_CODE,
                format!("{absent} is configured but no file in the project declares it"),
            )
            .help("add the file, or remove the book from the project's order"),
        );
    }

    let mut books = Vec::new();
    let mut provenance = Vec::new();

    for found in selected {
        let parsed = biblecompose_scripture::usfm::parse(found.path.clone(), found.source);
        diagnostics.extend(parsed.diagnostics);

        let (book, normalized) = normalize(found.book, &found.path, &parsed.document);
        diagnostics.extend(normalized);

        provenance.push(BookSource {
            code: found.book,
            path: found.path,
        });
        books.push(book);
    }

    let mut document = ScriptureDocument::new(books);
    document.provenance = provenance;

    Loaded {
        document,
        diagnostics,
    }
}
