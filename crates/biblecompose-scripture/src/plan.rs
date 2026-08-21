//! Which books go into the publication, and in what order.
//!
//! BOOK-001 through BOOK-003. The canon table ([`crate::canon`]) says what the
//! canonical order *is*; this says what a particular publication does with it.
//!
//! Three rules, in this order:
//!
//! 1. **Canonical order by default** (BOOK-001) — never filesystem order. A
//!    project whose files sort `01-genesis`, `02-exodus` gets the same result
//!    as one whose files are `GEN.usfm`, `EXO.usfm`, because neither is
//!    consulted.
//! 2. **A configured order wins where it speaks** (BOOK-002). It may be
//!    partial: naming three books puts those three first and leaves everything
//!    else canonical behind them. A lectionary edition that opens with John
//!    should not have to list all 66.
//! 3. **Inclusion and exclusion are settings, not deletions** (BOOK-003). A
//!    single-Gospel proof is a configuration change, and the other books stay
//!    on disk where the next build can have them back.
//!
//! Filling this from `biblecompose.toml` is M2's job; the policy lives here
//! because it is canon knowledge, and because M1 needs to order books before
//! there is a settings file to order them from.

use std::collections::BTreeSet;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics};

use crate::canon::BookCode;

/// What a project's settings say about book selection and order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookPlan {
    /// Explicit order, possibly partial. Empty means "canonical throughout".
    order: Vec<BookCode>,
    /// A whitelist. `None` means every discovered book is a candidate;
    /// `Some(empty)` is a project that has deliberately selected nothing,
    /// which is different and is left to the caller to notice.
    include: Option<BTreeSet<BookCode>>,
}

impl BookPlan {
    /// Canonical order, everything included. What a project with no book
    /// settings gets.
    pub fn canonical() -> Self {
        BookPlan::default()
    }

    /// Build a plan from what settings said, reporting anything unusable.
    ///
    /// Codes are taken as strings because the settings layer reads TOML and
    /// has no business knowing the canon — resolving them is this crate's job,
    /// and so is saying which ones do not exist.
    ///
    /// An unresolvable code is reported and dropped rather than failing the
    /// whole plan: `include = ["MAT", "TYPO"]` should still include Matthew,
    /// and a build blocked because one line of settings has a typo in it
    /// helps nobody.
    ///
    /// There is no exclusion list. There was one, and two settings that can
    /// contradict each other about the same book need a rule for which wins,
    /// which is a rule a publisher has to learn for no benefit — the set of
    /// books in the publication is one fact and `include` states it.
    pub fn from_settings(order: &[String], include: Option<&[String]>) -> (Self, Diagnostics) {
        let mut diagnostics = Diagnostics::new();

        let mut resolve_all = |field: &str, raw: &[String]| -> Vec<BookCode> {
            let mut seen = BTreeSet::new();
            let mut out = Vec::new();
            for s in raw {
                let Some(book) = BookCode::parse(s) else {
                    diagnostics.push(
                        Diagnostic::warning(
                            code::UNKNOWN_BOOK_CODE,
                            format!("books.{field} names {s:?}, which is not a book code"),
                        )
                        .help("use a USFM book code, for example MAT or 1CO"),
                    );
                    continue;
                };
                // A repeat in `order` would otherwise place the book twice.
                if seen.insert(book) {
                    out.push(book);
                } else {
                    diagnostics.push(Diagnostic::warning(
                        code::UNKNOWN_BOOK_CODE,
                        format!("books.{field} names {book} more than once"),
                    ));
                }
            }
            out
        };

        let order = resolve_all("order", order);
        let include: Option<BTreeSet<BookCode>> =
            include.map(|raw| resolve_all("include", raw).into_iter().collect());
        (BookPlan { order, include }, diagnostics)
    }

    /// Whether this book belongs in the publication (BOOK-003).
    pub fn includes(&self, book: BookCode) -> bool {
        match &self.include {
            Some(whitelist) => whitelist.contains(&book),
            None => true,
        }
    }

    /// Select and order `items` for the publication.
    ///
    /// Generic over the item rather than taking `BookCode` directly, because
    /// the caller has whole discovered books and would otherwise have to
    /// reorder a second collection to match — which is the kind of parallel
    /// bookkeeping that eventually disagrees with itself.
    pub fn arrange<T>(&self, items: Vec<T>, book_of: impl Fn(&T) -> BookCode) -> Vec<T> {
        let kept: Vec<T> = items
            .into_iter()
            .filter(|i| self.includes(book_of(i)))
            .collect();
        self.in_order(kept, book_of)
    }

    /// The same order, without applying the selection.
    ///
    /// For the window, which has to show a book the settings leave out — one
    /// that vanished from the list is one nobody can put back, and where it
    /// sits among the others is the whole of what "order" means.
    pub fn in_order<T>(&self, items: Vec<T>, book_of: impl Fn(&T) -> BookCode) -> Vec<T> {
        let mut items = items;
        // Configured books first in the order given; everything else after,
        // canonically. `position` over a short explicit list is cheaper than
        // building a map, and keeps the rule visible.
        items.sort_by_key(|i| {
            let book = book_of(i);
            match self.order.iter().position(|b| *b == book) {
                Some(rank) => (0usize, rank, 0u16),
                None => (1, 0, book.order()),
            }
        });
        items
    }

    /// Books named in the order or the whitelist that the project does not
    /// contain.
    ///
    /// Not an error: a project may configure an order for a whole Bible and
    /// build a subset of it (PRJ-005). Worth reporting, because a book missing
    /// from the output when the settings ask for it is otherwise a silent
    /// surprise.
    pub fn configured_but_absent(&self, present: &BTreeSet<BookCode>) -> Vec<BookCode> {
        let mut missing: Vec<BookCode> = self
            .order
            .iter()
            .chain(self.include.iter().flatten())
            .copied()
            .filter(|b| !present.contains(b))
            .collect();
        missing.sort();
        missing.dedup();
        missing
    }
}
