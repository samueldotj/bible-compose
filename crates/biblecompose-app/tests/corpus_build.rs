//! P5.3 — the corpus, through the whole application, onto real pages.
//!
//! Thirteen whole books in fourteen scripts. P1.6 asserts they *normalize*;
//! this asserts they *compose* — that nothing between the model and the PDF
//! refuses a real book, and that the pages it produces are structurally sound.
//!
//! **A book no available font can draw is not a failure of this suite.** The
//! coverage pre-flight blocking it is the correct answer (FONT-002), and the
//! font that would unblock it is a licensing decision waiting on P6.2. What
//! this asserts is the useful half: that *nothing else* stops a corpus book,
//! and that every book a font does cover produces sound pages. A second
//! diagnostic hiding behind the font one is exactly what that would otherwise
//! conceal.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use std::collections::BTreeMap;

use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::{BookCode, ScriptureDocument};
use biblecompose_testkit::corpus;
use biblecompose_testkit::pdf::Pdf;
use camino::{Utf8Path, Utf8PathBuf};
use common::{attempt, have_backend};

/// The fonts committed to this repository, and the family name each answers to.
///
/// Two, which is what there is. Everything else the corpus needs is waiting on
/// the same decision P6.2 is.
fn vendored() -> Vec<(&'static str, Vec<Utf8PathBuf>)> {
    let spike = biblecompose_testkit::repo_root().join("spike/assets/fonts");
    vec![
        // The backend's own, which needs no copying and covers Latin, Greek
        // and Cyrillic.
        ("DejaVu Serif", Vec::new()),
        (
            "Noto Serif Tamil",
            vec![
                spike.join("NotoSerifTamil-Regular.ttf"),
                spike.join("NotoSerifTamil-Bold.ttf"),
            ],
        ),
    ]
}

/// One corpus book, as a document.
fn read(entry: &corpus::Entry) -> ScriptureDocument {
    let source = corpus::read(entry);
    let code =
        BookCode::parse(&entry.book).unwrap_or_else(|| panic!("{} is not a book code", entry.book));
    let parsed = biblecompose_scripture::usfm::parse(entry.path.as_str(), source);
    let (book, _) = normalize(code, Utf8Path::new(entry.path.as_str()), &parsed.document);
    ScriptureDocument::new(vec![book])
}

/// **Nothing but the fonts stops a real book from composing.**
///
/// Every book is attempted against every font this repository holds. A book
/// that builds under any of them is composable; a book that builds under none
/// must have been stopped by coverage alone, and the assertion is that its
/// diagnostics say nothing else.
#[test]
fn no_corpus_book_is_blocked_by_anything_but_a_font() {
    if !have_backend() {
        return;
    }
    let mut built: Vec<String> = Vec::new();
    let mut uncovered: Vec<String> = Vec::new();

    for entry in corpus::books() {
        let doc = read(&entry);
        let mut done = false;
        let mut complaints: Vec<String> = Vec::new();

        for (family, files) in vendored() {
            let refs: Vec<&Utf8Path> = files.iter().map(Utf8PathBuf::as_path).collect();
            let (_g, report) = attempt(
                &doc,
                &format!(
                // The corpus commits USFM and not artwork, so a book that
                // names a plate has no plate. That is a missing asset and
                // not a composition defect.
                "[assets]\nmissing_figure = \"omit\"\n[typography]\nfont_family = \"{family}\"\n"
            ),
                &refs,
            );
            if report.output.is_some() {
                built.push(format!("{} in {family}", entry.book));
                done = true;
                break;
            }
            complaints.extend(
                report
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity.blocks())
                    .map(|d| format!("{}: {}", d.code, d.message)),
            );
        }

        if !done {
            // Coverage, and coverage only. Anything else is a defect this
            // suite exists to find, and the font gap must not hide it.
            let other: Vec<&String> = complaints
                .iter()
                .filter(|c| !c.starts_with("FONT-002"))
                .collect();
            assert!(
                other.is_empty(),
                "{} was stopped by more than a font: {other:?}",
                entry.book
            );
            uncovered.push(entry.book.clone());
        }
    }

    println!("composed {}: {}", built.len(), built.join(", "));
    println!(
        "no font for {}: {} — see P6.2",
        uncovered.len(),
        uncovered.join(", ")
    );
    assert!(
        !built.is_empty(),
        "at least one corpus book must reach a PDF, or this asserts nothing"
    );
}

/// **No glyph is set outside the measure.**
///
/// The assertion the roadmap has owed since a line-breaking defect was found
/// by reading glyph positions out of a PDF by hand: Scripture in a script that
/// does not hyphenate ran off the column and off the paper — 20.6% of lines in
/// one Tamil book, the worst 113pt past a 432pt page. Nothing in the suite
/// would have caught it.
///
/// Origins, which is what the roadmap asks for and what is honestly available:
/// a PDF records where each run of glyphs was placed and not how wide it is.
/// An overfull line puts its later runs past the margin, so origins find it.
///
/// Horizontal only. The running head and the folio are *deliberately* outside
/// the text block vertically, which is what a margin is for.
#[test]
fn no_glyph_is_set_outside_a_full_measure() {
    if !have_backend() {
        return;
    }
    for (book, family, fonts) in books() {
        let refs: Vec<&Utf8Path> = fonts.iter().map(Utf8PathBuf::as_path).collect();
        let over = overhangs(&book, family, &refs, 1);
        assert!(
            over.is_empty(),
            "{book} in {family}, one column: {} glyph runs outside the measure, \
             worst {:.1}pt",
            over.len(),
            worst(&over)
        );
    }
}

/// **In two columns, a word can be wider than the column, and then it hangs.**
///
/// Measured, and this is the whole of what is left: two lines of the Tamil
/// book — five glyph runs of 18,696 — each the tail of a single Tamil word
/// about 164pt wide in a 155pt column. There is no legal break inside a Tamil word — Tamil is
/// space-separated and UAX-14 offers nothing within one — and the emergency
/// stretch can widen spaces but cannot narrow a word. The typesetter's only
/// remaining choices are to overhang or to leave the column half empty, and it
/// overhangs.
///
/// So this is a ceiling rather than a zero, and the numbers are in it on
/// purpose: a regression shows up as one of them moving. The fix is break
/// opportunities inside Indic words, which is a typographic decision about a
/// script rather than a defect in a page builder.
#[test]
fn in_two_columns_an_unbreakable_word_hangs_by_a_bounded_amount() {
    if !have_backend() {
        return;
    }
    for (book, family, fonts) in books() {
        let refs: Vec<&Utf8Path> = fonts.iter().map(Utf8PathBuf::as_path).collect();
        let over = overhangs(&book, family, &refs, 2);
        assert!(
            over.len() <= 8,
            "{book} in {family}: {} glyph runs hang outside the column, which \
             was 5 — on two lines — when this was measured",
            over.len()
        );
        assert!(
            worst(&over) < 12.0,
            "{book} in {family}: worst overhang {:.1}pt, which was 9.8 when \
             this was measured",
            worst(&over)
        );
    }
}

/// The books this repository has a font for, with that font.
fn books() -> Vec<(String, &'static str, Vec<Utf8PathBuf>)> {
    let spike = biblecompose_testkit::repo_root().join("spike/assets/fonts");
    vec![
        (
            "LAM".to_owned(),
            "Noto Serif Tamil",
            vec![
                spike.join("NotoSerifTamil-Regular.ttf"),
                spike.join("NotoSerifTamil-Bold.ttf"),
            ],
        ),
        ("MRK".to_owned(), "DejaVu Serif", Vec::new()),
    ]
}

/// How far past the measure each glyph run starts, for the ones that do.
///
/// Symmetric margins, so the recto and verso masters put the text block in the
/// same place and one pair of bounds describes every page.
fn overhangs(book: &str, family: &str, fonts: &[&Utf8Path], columns: u8) -> Vec<f64> {
    let entry = corpus::books()
        .into_iter()
        .find(|e| e.book == book)
        .unwrap_or_else(|| panic!("{book} is not in the corpus"));
    let overrides = format!(
        "[page]\nsize = \"6in x 9in\"\ncolumns = {columns}\n\
         margin_inner = \"0.75in\"\nmargin_outer = \"0.75in\"\n\
         [typography]\nfont_family = \"{family}\"\n"
    );
    let (_g, report) = attempt(&read(&entry), &overrides, fonts);
    let path = report.output.as_ref().unwrap_or_else(|| {
        panic!(
            "{book} should build in {family}: {:?}",
            report
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
        )
    });
    let raw = std::fs::read(path.as_std_path()).expect("read the PDF");

    let left = 0.75 * 72.0;
    let right = (6.0 - 0.75) * 72.0;
    Pdf::marks(&raw)
        .into_iter()
        // A quarter point of slack: the margins are derived through
        // percentages of the page and back, and a rounding step is not an
        // overfull line.
        .map(|m| (left - m.x).max(m.x - right))
        .filter(|over| *over > 0.25)
        .collect()
}

fn worst(over: &[f64]) -> f64 {
    over.iter().copied().fold(0.0, f64::max)
}

/// The structural assertions PDF-001 – PDF-003 ask for, over every fixture:
/// one page geometry throughout, fonts embedded and subset, and no page that
/// carries nothing.
#[test]
fn every_page_of_every_fixture_is_sound() {
    if !have_backend() {
        return;
    }
    let mut sizes: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for (name, doc) in biblecompose_scripture::fixtures::all() {
        let (_g, report) = attempt(&doc, "", &[]);
        let Some(path) = report.output.as_ref() else {
            panic!(
                "{name} should build: {:?}",
                report
                    .diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
            );
        };
        let raw = std::fs::read(path.as_std_path()).expect("read the PDF");
        let pdf = Pdf::parse(&raw);

        // One trim size for the whole document (PDF-002). A publication whose
        // pages are not all the same size cannot be bound.
        let size = pdf
            .uniform_page_size_inches()
            .unwrap_or_else(|| panic!("{name} has pages of differing sizes"));
        sizes.insert(name.to_owned(), size);

        // The body font, embedded and subset rather than referenced (PDF-003).
        assert!(
            pdf.has_font("DejaVuSerif"),
            "{name} should embed its body font: {:?}",
            pdf.fonts
        );

        // **No page carries nothing.** A blank page in the middle of a
        // publication is a defect nobody notices until it is printed, and one
        // at the end is the trailing-blank-page defect P5.3 owes an assertion
        // to. Both are the same check.
        let marks = Pdf::marks(&raw);
        for page in 1..=pdf.pages {
            let on_page = marks.iter().filter(|m| m.page == page).count();
            assert!(on_page > 0, "{name} page {page} of {} is blank", pdf.pages);
        }
    }

    println!("{} fixtures, page sizes {sizes:?}", sizes.len());
}

/// **A paragraph long enough to run past ten pages leaves no blank ones.**
///
/// The assertion P5.3 has owed since the defect was measured: five blank pages
/// at the end of a document made from a 28,000-character paragraph, and none
/// at 11,000. It reproduced on the class as it stood before P4.4, so it was
/// nothing P4 introduced.
///
/// **It no longer reproduces.** 28,820 characters now set to 26 pages with
/// none blank, so what fixed it is somewhere in P4 — the likeliest candidate
/// being the removal of `balanced-frames`, which is the package that was
/// re-constraining both columns to the height of whatever was left in the
/// queue. This exists so that it stays fixed.
///
/// No real Scripture paragraph is that long, which is why it waited. It is
/// built here rather than added to the fixture set for the same reason: every
/// fixture is typeset by several suites, and a ten-page paragraph is a cost
/// each of them would pay for a case none of them is about.
#[test]
fn a_paragraph_that_runs_for_pages_leaves_none_blank() {
    if !have_backend() {
        return;
    }
    use biblecompose_scripture::{Block, Book, BookNames, Inline, ParaStyle};

    let sentence = "and they went out and preached everywhere while the Lord \
                    worked with them and confirmed the word by the signs that \
                    accompanied it. ";
    for repeats in [70usize, 220] {
        let body = sentence.repeat(repeats);
        let doc = ScriptureDocument::new(vec![Book::new(
            BookCode::parse("MRK").expect("a book code"),
            BookNames::named("Mark"),
            vec![Block::Paragraph {
                style: ParaStyle::P,
                content: vec![Inline::Text(body.clone())],
            }],
        )]);

        let (_g, report) = attempt(&doc, "", &[]);
        let path = report.output.as_ref().unwrap_or_else(|| {
            panic!(
                "{} characters should build: {:?}",
                body.len(),
                report
                    .diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
            )
        });
        let raw = std::fs::read(path.as_std_path()).expect("read the PDF");
        let pdf = Pdf::parse(&raw);
        let marks = Pdf::marks(&raw);

        println!("{} characters -> {} pages", body.len(), pdf.pages);
        // The case only exists past about ten pages, so a run that does not
        // get there is asserting nothing.
        if repeats == 220 {
            assert!(
                pdf.pages > 10,
                "the long case should run past ten pages, and ran to {}",
                pdf.pages
            );
        }

        let blank: Vec<usize> = (1..=pdf.pages)
            .filter(|p| !marks.iter().any(|m| m.page == *p))
            .collect();
        assert!(
            blank.is_empty(),
            "{} characters over {} pages left {} blank: {blank:?}",
            body.len(),
            pdf.pages,
            blank.len()
        );
    }
}
