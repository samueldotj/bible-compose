//! P1.4 — BOOK-001 through BOOK-003.

use biblecompose_scripture::canon::{BookCode, Testament};
use biblecompose_scripture::plan::BookPlan;
use std::collections::BTreeSet;

fn book(code: &str) -> BookCode {
    BookCode::parse(code).unwrap_or_else(|| panic!("{code} is a book code"))
}

fn books(codes: &[&str]) -> Vec<BookCode> {
    codes.iter().map(|c| book(c)).collect()
}

fn strings(codes: &[&str]) -> Vec<String> {
    codes.iter().map(|c| (*c).to_owned()).collect()
}

fn arranged(plan: &BookPlan, discovered: &[&str]) -> Vec<String> {
    plan.arrange(books(discovered), |b| *b)
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

/// BOOK-001. The filenames in this project would sort EXO before GEN; the
/// filesystem is not consulted, so it does not matter.
#[test]
fn canonical_order_beats_the_order_files_arrive_in() {
    let plan = BookPlan::canonical();
    assert_eq!(
        arranged(&plan, &["EXO", "GEN", "REV", "MAT"]),
        ["GEN", "EXO", "MAT", "REV"]
    );
}

/// BOOK-002.
#[test]
fn a_configured_order_is_reflected() {
    let (plan, d) = BookPlan::from_settings(&strings(&["JHN", "LUK", "MRK", "MAT"]), None, &[]);
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(
        arranged(&plan, &["MAT", "MRK", "LUK", "JHN"]),
        ["JHN", "LUK", "MRK", "MAT"]
    );
}

/// A partial order is the common case: an edition that opens with John should
/// not have to list all 66 books to say so.
#[test]
fn a_partial_order_leaves_the_rest_canonical() {
    let (plan, d) = BookPlan::from_settings(&strings(&["JHN"]), None, &[]);
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(
        arranged(&plan, &["GEN", "MAT", "JHN", "REV"]),
        ["JHN", "GEN", "MAT", "REV"]
    );
}

/// BOOK-003. The excluded book is still on disk; it is the settings that
/// changed, which is what makes a single-Gospel proof a one-line edit.
#[test]
fn an_excluded_book_does_not_appear() {
    let (plan, d) = BookPlan::from_settings(&[], None, &strings(&["GEN"]));
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(arranged(&plan, &["GEN", "MAT"]), ["MAT"]);
    assert!(!plan.includes(book("GEN")));
}

#[test]
fn an_include_list_is_a_whitelist() {
    let (plan, d) = BookPlan::from_settings(&[], Some(&strings(&["JHN"])), &[]);
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(arranged(&plan, &["GEN", "MAT", "JHN", "REV"]), ["JHN"]);
}

#[test]
fn exclude_wins_over_include_and_says_so() {
    let (plan, d) =
        BookPlan::from_settings(&[], Some(&strings(&["MAT", "JHN"])), &strings(&["JHN"]));
    assert_eq!(arranged(&plan, &["MAT", "JHN"]), ["MAT"]);

    let warned: Vec<&str> = d.iter().map(|x| x.code.as_str()).collect();
    assert_eq!(warned, ["CFG-005"]);
    assert!(d.blocking().next().is_none(), "a conflict is not fatal");
}

/// A typo in one entry must not discard the entries around it, and must not
/// block a build.
#[test]
fn an_unknown_code_is_reported_and_skipped() {
    let (plan, d) = BookPlan::from_settings(&[], None, &strings(&["MAT", "TYPO", "JHN"]));

    assert_eq!(
        d.iter().map(|x| x.code.as_str()).collect::<Vec<_>>(),
        ["CFG-005"]
    );
    assert!(d.blocking().next().is_none(), "settings typos do not block");
    // The usable entries still took effect.
    assert_eq!(arranged(&plan, &["MAT", "JHN", "LUK"]), ["LUK"]);
}

#[test]
fn a_repeated_book_is_placed_once() {
    let (plan, d) = BookPlan::from_settings(&strings(&["JHN", "JHN", "MAT"]), None, &[]);
    assert_eq!(
        d.iter().map(|x| x.code.as_str()).collect::<Vec<_>>(),
        ["CFG-005"]
    );
    assert_eq!(arranged(&plan, &["MAT", "JHN"]), ["JHN", "MAT"]);
}

/// PRJ-005 again, from the other side: configuring an order for a whole Bible
/// and building a Gospel is ordinary, not an error — but it is worth saying.
#[test]
fn books_configured_but_not_present_are_listed_without_blocking() {
    let (plan, _) = BookPlan::from_settings(&strings(&["MAT", "MRK", "LUK", "JHN"]), None, &[]);
    let present: BTreeSet<BookCode> = books(&["JHN"]).into_iter().collect();

    let absent: Vec<String> = plan
        .configured_but_absent(&present)
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    assert_eq!(absent, ["MAT", "MRK", "LUK"]);
}

/// An excluded book is not "missing" — it was asked to be gone.
#[test]
fn an_excluded_book_is_not_reported_as_absent() {
    let (plan, _) = BookPlan::from_settings(&strings(&["MAT", "JHN"]), None, &strings(&["MAT"]));
    let present: BTreeSet<BookCode> = books(&["JHN"]).into_iter().collect();
    assert!(plan.configured_but_absent(&present).is_empty());
}

/// The reason SRS-REVIEW put the deuterocanon in the table rather than behind
/// a setting: including one is a row, not a schema change.
#[test]
fn deuterocanonical_books_order_like_any_other() {
    let plan = BookPlan::canonical();
    assert_eq!(
        arranged(&plan, &["TOB", "REV", "GEN", "1MA"]),
        ["GEN", "REV", "TOB", "1MA"]
    );
    assert_eq!(book("TOB").testament(), Testament::Deuterocanon);

    // And a project that wants them interleaved says so, without the table
    // having to have an opinion about which tradition is right.
    let (plan, d) = BookPlan::from_settings(&strings(&["GEN", "TOB", "MAT"]), None, &[]);
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(
        arranged(&plan, &["MAT", "TOB", "GEN"]),
        ["GEN", "TOB", "MAT"]
    );
}

/// `arrange` is generic so a caller can order whole discovered books without
/// keeping a second collection in step.
#[test]
fn arrange_carries_the_callers_own_type() {
    struct Discovered {
        book: BookCode,
        path: &'static str,
    }

    let (plan, _) = BookPlan::from_settings(&strings(&["JHN"]), None, &strings(&["GEN"]));
    let items = vec![
        Discovered {
            book: book("GEN"),
            path: "genesis.usfm",
        },
        Discovered {
            book: book("MAT"),
            path: "01-matthew.usfm",
        },
        Discovered {
            book: book("JHN"),
            path: "zz-john.usfm",
        },
    ];

    let ordered: Vec<&str> = plan
        .arrange(items, |d| d.book)
        .iter()
        .map(|d| d.path)
        .collect();
    assert_eq!(ordered, ["zz-john.usfm", "01-matthew.usfm"]);
}
