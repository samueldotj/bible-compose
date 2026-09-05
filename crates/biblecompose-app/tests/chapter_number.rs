//! The chapter number's own decisions, read off the page (GUI-004, STY-005).
//!
//! Every one of these is a claim about *where* something is — the number on
//! a line by itself, a rule round it, the text starting further along, the
//! next chapter at the top of the next column or page — so every one is
//! measured rather than asserted from the argument list. And each is measured
//! in one column and in two, because a column break and a page break are
//! the same command in one and different commands in two, and the difference
//! is where the bugs live.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, project, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_testkit::pdf::{Line, Mark, Pdf, Rule};
use camino::Utf8PathBuf;
use common::{have_backend, BODY};

/// Two short chapters: enough lines for an initial to drop into, few enough
/// that both fit on one page in either column count, and no notes, so the
/// only rules on a page are the ones a border draws.
const TWO_CHAPTERS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.\n",
    "\\v 2 Now the earth was formless and void, and darkness was over the ",
    "surface of the deep. And the Spirit of God was hovering over the ",
    "surface of the waters.\n",
    "\\v 3 And God said, Let there be light, and there was light.\n",
    "\\v 4 God saw that the light was good, and He separated the light from ",
    "the darkness.\n",
    "\\v 5 God called the light day, and the darkness He called night. And ",
    "there was evening, and there was morning, the first day.\n",
    "\\c 2\n\\p\n",
    "\\v 1 Thus the heavens and the earth were completed in all their vast ",
    "array.\n",
    "\\v 2 And by the seventh day God had finished the work He had been ",
    "doing; so on that day He rested from all His work.\n",
    "\\v 3 Then God blessed the seventh day and sanctified it, because on ",
    "that day He rested from all the work of creation that He had ",
    "accomplished.\n",
);

/// The size the built-in sheet gives the number, which is how it is found.
const NUMBER: f64 = 21.0;

/// One build, with what the page looks like.
struct Built {
    _guard: tempfile::TempDir,
    lines: Vec<Line>,
    rules: Vec<Rule>,
    /// The text block's left and right edges on page 1, a recto.
    left: f64,
    right: f64,
    gap: f64,
    columns: usize,
    top: f64,
}

impl Built {
    /// The left and right edges of column `i` (from 0) on page 1.
    fn column(&self, i: usize) -> (f64, f64) {
        let n = self.columns as f64;
        let width = (self.right - self.left - self.gap * (n - 1.0)) / n;
        let left = self.left + i as f64 * (width + self.gap);
        (left, left + width)
    }

    /// The mark that is chapter `n`'s number, at whatever size it was set.
    fn number(&self, n: &str) -> &Mark {
        self.lines
            .iter()
            .flat_map(|l| &l.marks)
            .filter(|m| m.text == n && m.size > BODY * 1.5)
            .min_by(|a, b| a.page.cmp(&b.page).then(b.y.partial_cmp(&a.y).unwrap()))
            .unwrap_or_else(|| panic!("chapter {n}'s number is on a page"))
    }

    /// The line a mark sits on.
    fn line_of(&self, mark: &Mark) -> &Line {
        self.lines
            .iter()
            .find(|l| l.page == mark.page && l.y.to_bits() == mark.y.to_bits())
            .expect("a mark is on a line")
    }

    /// Where body text starts on the line a chapter's number is on, or the
    /// line after when the number has one to itself.
    fn text_after(&self, n: &str) -> f64 {
        let number = self.number(n);
        let same = self.line_of(number);
        let body = |l: &Line| {
            l.marks
                .iter()
                .filter(|m| (m.size - BODY).abs() < 0.01)
                .map(|m| m.x)
                .fold(f64::INFINITY, f64::min)
        };
        let on_line = body(same);
        if on_line.is_finite() {
            return on_line;
        }
        self.lines
            .iter()
            .filter(|l| l.page == number.page && l.y < number.y)
            .map(body)
            .find(|x| x.is_finite())
            .expect("text follows the number")
    }

    /// Pages that carry body text.
    fn pages_with_text(&self) -> Vec<usize> {
        let mut pages: Vec<usize> = self
            .lines
            .iter()
            .filter(|l| l.sizes().iter().any(|s| (s - BODY).abs() < 0.01))
            .map(|l| l.page)
            .collect();
        pages.dedup();
        pages
    }
}

/// A bare project of two chapters, with the styles given.
fn built(columns: usize, styles: &str) -> Built {
    built_with(columns, styles, "drop_caps = false")
}

/// The same, with a line of the `[contents]` settings chosen.
fn built_with(columns: usize, styles: &str, contents: &str) -> Built {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm").as_std_path(), TWO_CHAPTERS).expect("the book");
    std::fs::write(
        root.join("biblecompose.toml").as_std_path(),
        format!("schema_version = 1\n[page]\ncolumns = {columns}\n[contents]\n{contents}\n"),
    )
    .expect("the settings");
    std::fs::write(root.join("styles.toml").as_std_path(), styles).expect("the styles");

    let opened = project::open(&root);
    assert!(
        !opened.blocked(),
        "{:?}",
        opened
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    let mut request = BuildRequest::new(root.clone(), root.join("out.pdf"));
    request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
    request.settings = opened.settings.clone();
    request.styles = opened.styles.clone();
    request.prior = opened.diagnostics.clone();

    let (mut reporter, _events) = BuildReporter::new();
    let report = build(
        &opened.document,
        &request,
        &CancelToken::new(),
        &mut reporter,
    );
    assert_eq!(
        report.state,
        BuildState::Succeeded,
        "{:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    let raw =
        std::fs::read(report.output.as_ref().expect("a PDF").as_std_path()).expect("read the PDF");

    let page = &opened.settings.page;
    Built {
        _guard: guard,
        lines: Pdf::lines(&raw),
        rules: Pdf::rules(&raw),
        left: page.margin_inner.points(),
        right: page.size.width.points() - page.margin_outer.points(),
        gap: page.column_gap.points(),
        columns,
        top: -page.margin_top.points(),
    }
}

/// Both column counts, because that is the whole point of testing twice.
const COLUMNS: [usize; 2] = [1, 2];

/// **On its own line, where the alignment says.** The number's line holds
/// nothing else, and centred means centred in the *column*, not the page.
#[test]
fn the_number_takes_its_own_line_where_it_is_told() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        for (align, expect) in [("start", "left"), ("center", "middle"), ("end", "right")] {
            let b = built(
                columns,
                &format!("[chapter]\nown_line = true\nalign = \"{align}\"\n"),
            );
            let number = b.number("1");
            let line = b.line_of(number);
            assert_eq!(
                line.sizes(),
                vec![NUMBER],
                "{columns} columns, {align}: the number shares its line with nothing: {:?}",
                line.text()
            );
            let (l, r) = b.column(0);
            let middle = (l + r) / 2.0;
            match expect {
                "left" => assert!(
                    (number.x - l).abs() < 2.0,
                    "{columns} columns: at the margin: {} against {l}",
                    number.x
                ),
                "middle" => assert!(
                    (number.x - middle).abs() < 15.0,
                    "{columns} columns: centred in the column: {} against {middle}",
                    number.x
                ),
                _ => assert!(
                    number.x > r - 25.0 && number.x < r,
                    "{columns} columns: against the right edge: {} against {r}",
                    number.x
                ),
            }
            // And the text begins on the next line, at the margin — the
            // verse number first, which is the line's leftmost mark.
            let next = b
                .lines
                .iter()
                .filter(|x| x.page == number.page && x.y < number.y)
                .max_by(|a, c| a.y.partial_cmp(&c.y).unwrap())
                .expect("a line follows the number");
            let text = next.left();
            assert!(
                (text - l).abs() < 2.0,
                "{columns} columns: the line after an own-line number starts at the margin: {text} against {l}"
            );
        }
    }
}

/// **The gaps are the widths they say.** Measured as a difference against a
/// build without them, so the glyph's own width cancels out.
#[test]
fn the_gaps_before_and_after_are_what_they_say() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        let plain = built(columns, "");
        let after = built(columns, "[chapter]\ngap_after = \"30pt\"\n");
        // The default after-gap is 4pt, so 30pt moves the text 26pt.
        let moved = after.text_after("1") - plain.text_after("1");
        assert!(
            (moved - 26.0).abs() < 1.5,
            "{columns} columns: gap_after 30pt moves the text 26pt further than the 4pt default: {moved}"
        );

        let before = built(columns, "[chapter]\ngap_before = \"20pt\"\n");
        let shifted = before.number("1").x - plain.number("1").x;
        assert!(
            (shifted - 20.0).abs() < 1.0,
            "{columns} columns: gap_before 20pt moves the number 20pt: {shifted}"
        );
    }
}

/// **A border is four rules round the number, as thick as it says.** The
/// fixture has no notes, so a page without a border has no rules at all.
#[test]
fn a_border_is_drawn_round_the_number() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        let plain = built(columns, "");
        assert!(
            plain.rules.iter().all(|r| r.page != 1),
            "{columns} columns: nothing draws a rule on an unbordered page: {:?}",
            plain.rules
        );

        let b = built(
            columns,
            "[chapter]\nborder = true\nborder_width = \"1.5pt\"\n",
        );
        let number = b.number("1");
        let near: Vec<&Rule> = b
            .rules
            .iter()
            .filter(|r| {
                r.page == number.page
                    && (r.x1 - number.x).abs() < 40.0
                    && (r.y1 - number.y).abs() < 40.0
            })
            .collect();
        assert_eq!(
            near.len(),
            4,
            "{columns} columns: four rules round the number at ({}, {}): near {near:?}; all {:?}",
            number.x,
            number.y,
            b.rules
        );
        // Two of them run across and two stand, every one as thick as asked
        // and longer than the number is wide.
        assert!(
            near.iter()
                .all(|r| (r.thickness - 1.5).abs() < 0.05 && r.length() > 10.0),
            "{columns} columns: {near:?}"
        );
        assert_eq!(
            near.iter().filter(|r| r.is_horizontal()).count(),
            2,
            "{near:?}"
        );
        assert_eq!(
            near.iter().filter(|r| r.is_vertical()).count(),
            2,
            "{near:?}"
        );
        // And the number is inside them: a rule stands to its left, a rule
        // runs above its baseline and one below.
        let left = near
            .iter()
            .map(|r| r.x1.min(r.x2))
            .fold(f64::INFINITY, f64::min);
        let top = near
            .iter()
            .map(|r| r.y1.max(r.y2))
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = near
            .iter()
            .map(|r| r.y1.min(r.y2))
            .fold(f64::INFINITY, f64::min);
        assert!(
            left < number.x && top > number.y && bottom < number.y,
            "{columns} columns: the rules ({left}, {top}..{bottom}) enclose the number ({}, {})",
            number.x,
            number.y
        );
    }
}

/// **As a drop cap, the number drops** — set large, with the first lines
/// indented past it and a later one back at the margin — and the text's own
/// initial does not drop beside it.
#[test]
fn the_number_can_drop_into_the_text() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        let b = built(columns, "[chapter]\ndrop_cap = true\n");
        let number = b.number("1");
        assert!(
            number.size > BODY * 2.5,
            "{columns} columns: a dropped number is set large: {}pt",
            number.size
        );

        // Lines of body text on page 1, top down.
        let mut body: Vec<&Line> = b
            .lines
            .iter()
            .filter(|l| l.page == 1 && l.sizes().iter().any(|s| (s - BODY).abs() < 0.01))
            .collect();
        body.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap());
        let text_left = |l: &Line| {
            l.marks
                .iter()
                .filter(|m| m.size <= BODY + 0.1)
                .map(|m| m.x)
                .fold(f64::INFINITY, f64::min)
        };
        let (l, _) = b.column(0);
        assert!(
            text_left(body[0]) > l + 5.0 && text_left(body[1]) > l + 5.0,
            "{columns} columns: the first lines are indented past the number: {} and {} against {l}",
            text_left(body[0]),
            text_left(body[1])
        );
        assert!(
            body.iter()
                .skip(3)
                .any(|line| (text_left(line) - l).abs() < 0.5),
            "{columns} columns: a later line returns to the margin"
        );

        // With the Contents tab's drop caps on as well, the number is the one
        // dropped thing: no large `I` from "In the beginning".
        let both = built_with(columns, "[chapter]\ndrop_cap = true\n", "drop_caps = true");
        let big: Vec<&Mark> = both
            .lines
            .iter()
            .flat_map(|l| &l.marks)
            .filter(|m| m.size > BODY * 2.0)
            .collect();
        assert!(
            big.iter().all(|m| m.text != "I"),
            "{columns} columns: only the number drops: {:?}",
            big.iter().map(|m| &m.text).collect::<Vec<_>>()
        );
    }
}

/// **A chapter can open a new column.** In two columns that is the top of
/// column B of the same page; in one column the next frame is the next page.
#[test]
fn a_chapter_can_open_a_new_column() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        let plain = built(columns, "");
        let b = built(columns, "[chapter]\nnew_column = true\n");
        let one = b.number("1");
        let two = b.number("2");
        assert_eq!(one.page, 1);
        match columns {
            2 => {
                let (bl, _) = b.column(1);
                assert_eq!(two.page, 1, "two columns: chapter 2 is still on page 1");
                assert!(
                    two.x >= bl - 1.0,
                    "two columns: chapter 2 opens column B: x={} y={} (page {}) against {bl}; chapter 1 at x={} y={}; plain chapter 2 at x={} y={} page {}",
                    two.x,
                    two.y,
                    two.page,
                    one.x,
                    one.y,
                    plain.number("2").x,
                    plain.number("2").y,
                    plain.number("2").page
                );
                assert!(
                    two.y > b.top - 40.0,
                    "two columns: at the top of the column: y={} (top {})",
                    two.y,
                    b.top
                );
                // Whereas left alone, chapter 2 follows chapter 1 in column A.
                assert!(
                    plain.number("2").x < bl,
                    "two columns: without the setting chapter 2 stays in column A"
                );
            }
            _ => {
                assert_eq!(two.page, 2, "one column: a new column is a new page");
                assert!(
                    two.y > b.top - 40.0,
                    "one column: at the top of it: y={}",
                    two.y
                );
                assert_eq!(
                    plain.number("2").page,
                    1,
                    "one column: without the setting, page 1"
                );
            }
        }
    }
}

/// **A chapter can open a new page, on the side it asks for.** `next` is
/// page 2; `left` is page 2, an even page; `right` is page 3, with page 2
/// left blank.
#[test]
fn a_chapter_can_open_a_new_page_on_a_side() {
    if !have_backend() {
        return;
    }
    for columns in COLUMNS {
        for (where_, page, blank) in [("next", 2, None), ("left", 2, None), ("right", 3, Some(2))] {
            let b = built(columns, &format!("[chapter]\nnew_page = \"{where_}\"\n"));
            let two = b.number("2");
            assert_eq!(
                two.page, page,
                "{columns} columns, new_page = {where_}: chapter 2 opens page {page}"
            );
            assert!(
                two.y > b.top - 40.0,
                "{columns} columns, {where_}: at the top of the page: y={}",
                two.y
            );
            assert_eq!(b.number("1").page, 1);
            if let Some(empty) = blank {
                assert!(
                    !b.pages_with_text().contains(&empty),
                    "{columns} columns, {where_}: page {empty} is left blank: {:?}",
                    b.pages_with_text()
                );
            }
        }
    }
}
