//! The page's furniture, read off the page: rules under the head, over the
//! foot and down the gutter; a frame round the page; ruled lines in the
//! outer margin; a paragraph's first-line indent; the dropped initial's own
//! style; and the head and foot each set in their own style.
//!
//! Each is a claim about what is drawn and where, so each is measured from
//! the PDF rather than read back from the settings that asked for it.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use biblecompose_app::{build, project, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_testkit::pdf::{Line, Pdf, Rule};
use camino::Utf8PathBuf;
use common::{have_backend, BODY};

/// Four paragraphs and a heading between them: the first opens the chapter,
/// the third follows the heading, and the second and fourth are the ones a
/// first-line indent is for.
const PARAGRAPHS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heaven and the earth. And the ",
    "earth was without form, and void; and darkness was upon the face of ",
    "the deep. And the Spirit of God moved upon the face of the waters.\n",
    "\\p\n",
    "\\v 2 And God said, Let there be light: and there was light. And God ",
    "saw the light, that it was good: and God divided the light from the ",
    "darkness.\n",
    "\\s1 The Firmament\n",
    "\\p\n",
    "\\v 3 And God called the light Day, and the darkness he called Night. ",
    "And the evening and the morning were the first day.\n",
    "\\p\n",
    "\\v 4 And God said, Let there be a firmament in the midst of the ",
    "waters, and let it divide the waters from the waters. And God made ",
    "the firmament, and divided the waters which were under the firmament ",
    "from the waters which were above the firmament: and it was so.\n",
);

struct Built {
    _guard: tempfile::TempDir,
    lines: Vec<Line>,
    rules: Vec<Rule>,
    raw: Vec<u8>,
    /// Page width and height, in points.
    width: f64,
    height: f64,
    /// The text block's left and right edges on page 1, a recto.
    left: f64,
    right: f64,
}

/// A bare project: the USFM, a settings file and a style sheet.
fn built(usfm: &str, settings: &str, styles: &str) -> Built {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm").as_std_path(), usfm).expect("the book");
    std::fs::write(
        root.join("biblecompose.toml").as_std_path(),
        // Genesis alone, so page 1 is the body page being measured.
        format!("schema_version = 1\n[books]\ninclude = [\"GEN\"]\n{settings}\n"),
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
        raw,
        width: page.size.width.points(),
        height: page.size.height.points(),
        left: page.margin_inner.points(),
        right: page.size.width.points() - page.margin_outer.points(),
    }
}

/// One column, no notes: the only rules on the page are the furniture.
const ONE: &str = "[page]\ncolumns = 1\n";

/// Two columns, no notes.
const TWO: &str = "[page]\ncolumns = 2\n";

/// The left edge of the body line a verse number opens, on page 1.
fn verse_left(b: &Built, n: &str) -> f64 {
    b.lines
        .iter()
        .filter(|l| l.page == 1)
        .flat_map(|l| &l.marks)
        // At the verse number's own size: the head is smaller than the body
        // too, and its range ends in a figure.
        .find(|m| m.text == n && (m.size - 6.4).abs() < 0.05)
        .unwrap_or_else(|| panic!("verse {n} is numbered on page 1"))
        .x
}

/// Rules on page 1.
fn rules_on(b: &Built, page: usize) -> Vec<&Rule> {
    b.rules.iter().filter(|r| r.page == page).collect()
}

/// **A first-line indent indents the paragraphs that need one**, and only
/// those: not the one that opens the chapter, and not the one under a
/// heading, which a book sets flush. The heading is flush too.
#[test]
fn a_paragraph_indents_its_first_line_except_where_a_book_does_not() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        ONE,
        "[paragraph.p]\nfirst_line_indent = \"14pt\"\n",
    );
    let flush = b.left;
    // Verse 2 opens the second paragraph and verse 4 the fourth: indented.
    assert!(
        (verse_left(&b, "2") - (flush + 14.0)).abs() < 0.6,
        "verse 2 at {}, the margin at {flush}",
        verse_left(&b, "2")
    );
    assert!(
        (verse_left(&b, "4") - (flush + 14.0)).abs() < 0.6,
        "verse 4 at {}, the margin at {flush}",
        verse_left(&b, "4")
    );
    // Verse 3 follows the heading: flush.
    assert!(
        (verse_left(&b, "3") - flush).abs() < 0.6,
        "under a heading the paragraph is flush: {}",
        verse_left(&b, "3")
    );
    // The heading itself is flush.
    let heading = b
        .lines
        .iter()
        .find(|l| l.text().replace(' ', "") == "TheFirmament")
        .expect("the heading");
    assert!((heading.left() - flush).abs() < 0.6, "{}", heading.left());
    // And the chapter's opening line starts at the margin.
    let opening = b
        .lines
        .iter()
        .find(|l| l.text().replace(' ', "").contains("Inthebeginning"))
        .expect("the opening line");
    assert!((opening.left() - flush).abs() < 0.6, "{}", opening.left());
}

/// **The head and foot rules span the text block, and the gutter rule runs
/// down between the columns.** None of them is drawn unless asked for.
#[test]
fn rules_under_the_head_over_the_foot_and_down_the_gutter() {
    if !have_backend() {
        return;
    }
    let bare = built(PARAGRAPHS, TWO, "");
    assert!(rules_on(&bare, 1).is_empty(), "{:?}", bare.rules);

    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 2\nheader_rule = \"0.6pt\"\nfooter_rule = \"0.4pt\"\ncolumn_rule = \"0.5pt\"\n",
        "",
    );
    let rules = rules_on(&b, 1);
    let across: Vec<_> = rules
        .iter()
        .filter(|r| r.is_horizontal() && (r.length() - (b.right - b.left)).abs() < 1.0)
        .collect();
    assert_eq!(across.len(), 2, "a head rule and a foot rule: {rules:?}");
    let thick: Vec<f64> = across.iter().map(|r| r.thickness).collect();
    assert!(thick.iter().any(|t| (t - 0.6).abs() < 0.05), "{thick:?}");
    assert!(thick.iter().any(|t| (t - 0.4).abs() < 0.05), "{thick:?}");

    // The head rule is above every line of the body and the foot rule below.
    let ys: Vec<f64> = across.iter().map(|r| r.y1).collect();
    let body: Vec<f64> = b
        .lines
        .iter()
        .filter(|l| l.page == 1 && l.sizes().contains(&BODY))
        .map(|l| l.y)
        .collect();
    let (top, bottom) = (
        ys.iter().cloned().fold(f64::MIN, f64::max),
        ys.iter().cloned().fold(f64::MAX, f64::min),
    );
    assert!(
        body.iter().all(|y| *y < top && *y > bottom),
        "{ys:?} {body:?}"
    );

    let down: Vec<_> = rules.iter().filter(|r| r.is_vertical()).collect();
    assert_eq!(down.len(), 1, "one gutter rule: {rules:?}");
    let middle = (b.left + b.right) / 2.0;
    assert!(
        (down[0].x1 - middle).abs() < 1.0,
        "{} vs {middle}",
        down[0].x1
    );
}

/// **Each rule has its own weight, ink and distance from the text**: a named
/// weight is its thickness, each is drawn in its own colour, and the head and
/// foot rules stand their gap off the text block.
#[test]
fn each_rule_has_its_own_weight_ink_and_gap() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 2\n\
         header_rule = \"thick\"\nheader_rule_color = \"#ff0000\"\nheader_rule_gap = \"12pt\"\n\
         footer_rule = \"hairline\"\nfooter_rule_color = \"#00ff00\"\nfooter_rule_gap = \"3pt\"\n\
         column_rule = \"thin\"\ncolumn_rule_color = \"#0000ff\"\n",
        "",
    );
    let rules = rules_on(&b, 1);
    let across: Vec<_> = rules
        .iter()
        .filter(|r| r.is_horizontal() && (r.length() - (b.right - b.left)).abs() < 1.0)
        .collect();
    assert_eq!(across.len(), 2, "{rules:?}");
    // PDF space runs up the page: the head rule is the higher one.
    let (head, foot) = if across[0].y1 > across[1].y1 {
        (across[0], across[1])
    } else {
        (across[1], across[0])
    };
    assert!((head.thickness - 1.2).abs() < 0.05, "thick: {head:?}");
    assert!((foot.thickness - 0.25).abs() < 0.05, "hairline: {foot:?}");
    assert_eq!(head.color, (1.0, 0.0, 0.0), "{head:?}");
    assert_eq!(foot.color, (0.0, 1.0, 0.0), "{foot:?}");
    let gutter = rules
        .iter()
        .find(|r| r.is_vertical())
        .expect("a gutter rule");
    assert!((gutter.thickness - 0.5).abs() < 0.05, "thin: {gutter:?}");
    assert_eq!(gutter.color, (0.0, 0.0, 1.0), "{gutter:?}");

    // The gap, measured from the top margin, where the text block starts —
    // not from the gutter rule, which on this page starts lower, below the
    // book's title spanning the columns. Rules are read top-down, like the
    // marks, and the built-in top margin is 0.81in.
    let top = -0.81 * 72.0;
    let gap_above = head.y1 - head.thickness / 2.0 - top;
    assert!(
        (gap_above - 12.0).abs() < 0.3,
        "head rule to text: {gap_above}"
    );

    // Moving the gap moves the rule by as much.
    let nearer = built(
        PARAGRAPHS,
        "[page]\ncolumns = 2\nheader_rule = \"thick\"\nheader_rule_gap = \"4pt\"\n",
        "",
    );
    let head_near = rules_on(&nearer, 1)
        .into_iter()
        .find(|r| r.is_horizontal())
        .expect("a head rule");
    assert!(
        (head.y1 - head_near.y1 - 8.0).abs() < 0.3,
        "{} then {}",
        head.y1,
        head_near.y1
    );
}

/// A chapter that opens on a heading, as most modern translations' do.
const HEADED: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\s1 The Creation\n\\p\n",
    "\\v 1 In the beginning God created the heaven and the earth. And the ",
    "earth was without form, and void; and darkness was upon the face of ",
    "the deep. And the Spirit of God moved upon the face of the waters.\n",
    "\\v 2 And God said, Let there be light: and there was light. And God ",
    "saw the light, that it was good: and God divided the light from the ",
    "darkness.\n",
);

/// **A dropped chapter number drops into the paragraph, not the heading**,
/// and stands the `chapter` style's `gap_after` off every line beside it —
/// the first included, which the drop-cap package would otherwise set hard
/// against the figure.
#[test]
fn a_dropped_chapter_number_sits_under_its_heading_with_its_gap() {
    if !have_backend() {
        return;
    }
    const DROP: &str = "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\n\
                        drop_cap_of = \"chapter_number\"\ndrop_cap_lines = 2\n";
    let at = |gap: &str| {
        let b = built(HEADED, DROP, &format!("[chapter]\ngap_after = \"{gap}\"\n"));
        let number = b
            .lines
            .iter()
            .flat_map(|l| &l.marks)
            .find(|m| m.text == "1" && m.size > BODY * 1.5)
            .cloned()
            .expect("the dropped figure");
        let heading = b
            .lines
            .iter()
            .find(|l| l.text().replace(' ', "") == "TheCreation")
            .expect("the heading")
            .y;
        // The body lines beside the figure: the first two at the body size.
        let beside: Vec<(f64, f64)> = b
            .lines
            .iter()
            .filter(|l| l.page == 1 && l.sizes().contains(&BODY) && l.y < heading)
            .take(2)
            .map(|l| {
                let x = l
                    .marks
                    .iter()
                    .filter(|m| (m.size - BODY).abs() < 0.05)
                    .map(|m| m.x)
                    .fold(f64::INFINITY, f64::min);
                (x, l.y)
            })
            .collect();
        (number, heading, beside)
    };

    let (number, heading, near) = at("2pt");
    assert!(
        number.y < heading,
        "the figure is below its heading: {} vs {heading}",
        number.y
    );
    assert_eq!(near.len(), 2, "two lines beside the figure: {near:?}");
    // Two lines deep: the figure stands on the second line of the paragraph,
    // not on the line after the heading's.
    assert!(
        (number.y - near[1].1).abs() < 1.0,
        "the figure's baseline is the second line's: {} vs {:?}",
        number.y,
        near
    );
    assert!(
        near.iter().all(|(x, _)| *x > number.x + 1.0),
        "both lines stand right of the figure: {near:?} vs {}",
        number.x
    );

    // Eight points more gap moves both lines eight points right.
    let (_, _, far) = at("10pt");
    for ((a, _), (b, _)) in near.iter().zip(&far) {
        assert!((b - a - 8.0).abs() < 0.3, "{near:?} then {far:?}");
    }
}

/// **In two columns the book's title spans both, with its ornament under
/// it**, and the text starts below them. The ornament is two short rules
/// and a diamond, centred on the text block rather than on the first column.
#[test]
fn the_book_title_spans_both_columns_with_its_ornament() {
    if !have_backend() {
        return;
    }
    let b = built(
        HEADED,
        "[page]\ncolumns = 2\n[contents]\ntitle_ornament = \"diamond\"\n",
        "",
    );
    let middle = (b.left + b.right) / 2.0;
    let title = b
        .lines
        .iter()
        .find(|l| l.page == 1 && l.text() == "Genesis" && l.sizes() == vec![16.0])
        .expect("the title, at the book style's built-in size");
    let first = title.marks.first().expect("a mark").x;
    // The title's width is not in the marks; its left edge is enough: in
    // the first column alone it would start left of the text block's middle
    // by less than half a column.
    let column = (b.right - b.left) / 2.0;
    assert!(
        middle - first < column / 2.0 && first < middle,
        "the title starts at {first}, the text block's middle is {middle}"
    );

    // The ornament: two short rules either side of the middle, below the
    // title, and the diamond between them.
    let rules: Vec<&Rule> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| r.is_horizontal() && r.length() < column)
        .collect();
    assert_eq!(rules.len(), 2, "{rules:?}");
    assert!(rules.iter().any(|r| r.x2 < middle) && rules.iter().any(|r| r.x1 > middle));
    let diamond = b
        .lines
        .iter()
        .flat_map(|l| &l.marks)
        .find(|m| m.text == "◆")
        .expect("the diamond");
    assert!(
        (diamond.x - middle).abs() < 4.0,
        "{} vs {middle}",
        diamond.x
    );
    assert!(diamond.y < title.y, "the ornament is under the title");

    // The text below the ornament, in either column.
    let body: Vec<_> = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .flat_map(|l| &l.marks)
        .filter(|m| (m.size - BODY).abs() < 0.05)
        .collect();
    let top_of = |right: bool| {
        body.iter()
            .filter(|m| (m.x > middle) == right)
            .map(|m| m.y)
            .fold(f64::MIN, f64::max)
    };
    let (a, c) = (top_of(false), top_of(true));
    assert!(a < diamond.y && c < diamond.y, "{a} {c} {}", diamond.y);

    // And with no ornament asked for, none is drawn.
    let plain = built(HEADED, "[page]\ncolumns = 2\n", "");
    assert!(rules_on(&plain, 1).is_empty(), "{:?}", rules_on(&plain, 1));
    assert!(!plain.lines.iter().any(|l| l.text().contains('◆')));
}

/// **A double frame is two boxes inside the trim**, on every page.
#[test]
fn a_double_frame_is_drawn_inside_the_trim() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\nframe = \"double\"\nframe_width = \"0.8pt\"\nframe_inset = \"0.25in\"\n",
        "",
    );
    let rules = rules_on(&b, 1);
    let tall: Vec<_> = rules
        .iter()
        .filter(|r| r.is_vertical() && r.length() > b.height * 0.8)
        .collect();
    let wide: Vec<_> = rules
        .iter()
        .filter(|r| r.is_horizontal() && r.length() > b.width * 0.8)
        .collect();
    assert_eq!(tall.len(), 4, "{rules:?}");
    assert_eq!(wide.len(), 4, "{rules:?}");
    // The outer box sits at the inset, a quarter inch from the left trim.
    let leftmost = tall.iter().map(|r| r.x1).fold(f64::MAX, f64::min);
    assert!((leftmost - 18.0).abs() < 1.5, "{leftmost}");
}

/// **Note lines rule the outer margin**: on a recto, to the right of the
/// text block, and nowhere over the text.
#[test]
fn note_lines_rule_the_outer_margin() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\nmargin_outer = \"1.8in\"\nnote_lines = true\nnote_line_spacing = \"18pt\"\n",
        "",
    );
    let lines: Vec<_> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| r.is_horizontal())
        .collect();
    assert!(
        lines.len() > 10,
        "a margin's worth of lines: {}",
        lines.len()
    );
    assert!(
        lines.iter().all(|r| r.x1.min(r.x2) > b.right),
        "every line is in the outer margin"
    );
}

/// **The foot is set in the folio style**, not the head's.
#[test]
fn the_foot_line_is_set_in_its_own_style() {
    if !have_backend() {
        return;
    }
    let b = built(PARAGRAPHS, ONE, "[folio]\nfont_size = \"11pt\"\n");
    let foot = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .min_by(|a, c| a.y.partial_cmp(&c.y).expect("numbers"))
        .expect("a foot line");
    assert!(foot.sizes().contains(&11.0), "{:?}", foot.sizes());
}

/// **The dropped initial takes the `initial` style's border.**
#[test]
fn the_dropped_initial_can_be_boxed() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\ndrop_cap_of = \"first_letter\"\n",
        "[initial]\nborder = true\nborder_width = \"0.8pt\"\ncolor = \"#9e2a1f\"\n",
    );
    let boxed: Vec<_> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| (r.thickness - 0.8).abs() < 0.05)
        .collect();
    assert_eq!(
        boxed.len(),
        4,
        "four sides round the initial: {:?}",
        b.rules
    );
    let _ = &b.raw;
}
