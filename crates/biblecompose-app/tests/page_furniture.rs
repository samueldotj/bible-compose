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
    // The book the fixture says it is, alone, so page 1 is the body page
    // being measured.
    let code = usfm
        .strip_prefix("\\id ")
        .and_then(|rest| rest.get(..3))
        .expect("a fixture opens with its \\id");
    std::fs::write(root.join(format!("{code}.usfm")).as_std_path(), usfm).expect("the book");
    std::fs::write(
        root.join("biblecompose.toml").as_std_path(),
        format!("schema_version = 1\n[books]\ninclude = [\"{code}\"]\n{settings}\n"),
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

/// **A paragraph too short for the dropped figure hands the rest of it on**:
/// the Berean Study Bible sets Genesis 1:1 as a paragraph of one line, and
/// the paragraph after it ran into the figure's second line. It now stands
/// off the figure as the first did, and the line after that is back at the
/// margin.
#[test]
fn the_paragraph_after_a_one_line_opening_runs_round_the_figure() {
    if !have_backend() {
        return;
    }
    const SHORT: &str = concat!(
        "\\id GEN\n\\h Genesis\n\\c 1\n\\s1 The Creation\n\\p\n",
        "\\v 1 In the beginning God created the heavens.\n\\p\n",
        "\\v 2 Now the earth was formless and void, and darkness was over the ",
        "surface of the deep. And the Spirit of God was hovering over the ",
        "surface of the waters, and God said, Let there be light.\n",
    );
    let b = built(
        SHORT,
        "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\n\
         drop_cap_of = \"chapter_number\"\ndrop_cap_lines = 2\n",
        "[chapter]\ngap_after = \"4pt\"\n",
    );
    let number = b
        .lines
        .iter()
        .flat_map(|l| &l.marks)
        .find(|m| m.text == "1" && m.size > BODY * 1.5)
        .cloned()
        .expect("the dropped figure");
    let starts: Vec<f64> = b
        .lines
        .iter()
        .filter(|l| l.page == 1 && l.sizes().contains(&BODY) && l.y > number.y - 30.0)
        .take(3)
        .map(|l| {
            l.marks
                .iter()
                .filter(|m| (m.size - BODY).abs() < 0.05)
                .map(|m| m.x)
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    assert_eq!(starts.len(), 3, "three body lines: {starts:?}");
    assert!(
        starts[0] > number.x + 1.0 && starts[1] > number.x + 1.0,
        "verse 1's line and verse 2's first stand right of the figure: {starts:?} vs {}",
        number.x
    );
    assert!(
        starts[1] >= starts[0] - 0.5 && starts[1] - starts[0] < 14.0,
        "and at the same place, but for verse 2's number before its text          (verse 1's is hidden), with no paragraph indent on top: {starts:?}"
    );
    assert!(
        starts[2] < starts[1] - 5.0,
        "the line after the figure is back at the margin: {starts:?}"
    );
}

/// A passage with two study notes (`\ef`) and a word study (`\esb`).
const STUDIED: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\s1 The Creation\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.",
    "\\ef - \\fr 1:1 \\fk In the beginning.\\fk* \\ft Alpha note, which runs on ",
    "long enough to take more than one line of its column.\\ef*\n",
    "\\v 2 Now the earth was formless and void.",
    "\\ef - \\fr 1:2 \\fk formless.\\fk* \\ft Beta note, which also runs on long ",
    "enough to take more than one line of its column.\\ef*\n",
    "\\esb \\cat Word study\\cat*\n\\s1 Created\n\\r Hebrew bara\n",
    "\\p Gamma, the explanation, set beside the word in the box.\n\\esbe\n",
    "\\s1 The First Day\n\\p\n\\v 3 And God said, Let there be light.\n",
);

/// **A passage's study notes are set after it, under a rule and a label,
/// and its word study is a box before them.** In the order the design has:
/// the text, the box, the rule, the label, the notes, then the next
/// heading; the box with its word at the left and the explanation at the
/// right on its own ground; the notes in two columns, one note each.
#[test]
fn study_notes_and_a_word_study_follow_their_passage() {
    if !have_backend() {
        return;
    }
    let b = built(
        STUDIED,
        "[page]\ncolumns = 1\n[notes]\nstudy_notes_columns = 2\n\
         study_notes_rule = \"1pt\"\nstudy_notes_label = \"Study notes\"\n",
        "[sidebar]\nbackground = \"#ff0000\"\n",
    );
    let find = |needle: &str| {
        b.lines
            .iter()
            .find(|l| {
                let squash = |t: &str| t.replace(' ', "").to_lowercase();
                l.page == 1 && squash(&l.text()).contains(&squash(needle))
            })
            .unwrap_or_else(|| panic!("no line with {needle:?}"))
    };
    let verse2 = find("formless and void");
    let word = b
        .lines
        .iter()
        .find(|l| l.page == 1 && l.text().trim() == "Created")
        .expect("the word study's word, on a line of its own");
    let gamma = find("Gamma");
    let label = find("study notes");
    let alpha = find("Alpha");
    let beta = find("Beta");
    let next = find("The First Day");

    // PDF space runs up the page: each is lower than the one before.
    assert!(verse2.y > word.y, "the box is under the text");
    assert!(word.y > label.y, "the notes' label is under the box");
    assert!(label.y > alpha.y, "the notes are under their label");
    assert!(alpha.y > next.y, "and all of it before the next heading");

    // The box: the word at the left, the explanation beside it, on red.
    let x = |l: &&Line| l.marks.first().expect("a mark").x;
    assert!(x(&word) < x(&gamma) - 40.0, "{} vs {}", x(&word), x(&gamma));
    let middle = (b.left + b.right) / 2.0;
    let rules = rules_on(&b, 1);
    let ground: Vec<_> = rules
        .iter()
        .filter(|r| r.color == (1.0, 0.0, 0.0))
        .collect();
    assert_eq!(ground.len(), 1, "{rules:?}");
    assert!(
        ground[0].length() > (b.right - b.left) * 0.95,
        "{:?}",
        ground[0]
    );

    // The rule over the label, the measure wide, a point thick.
    assert!(
        rules.iter().any(|r| r.is_horizontal()
            && (r.thickness - 1.0).abs() < 0.05
            && r.length() > (b.right - b.left) * 0.95
            && r.y1 > label.y
            && r.y1 < word.y),
        "{rules:?}"
    );

    // Two columns, a note in each.
    // A row holds a line of each column, so the word, not the row's start.
    let at = |l: &Line, word: &str| {
        l.marks
            .iter()
            .find(|m| m.text.contains(word))
            .unwrap_or_else(|| panic!("no {word:?} in {:?}", l.text()))
            .x
    };
    assert!(
        at(alpha, "Alpha") < middle,
        "the first note is in the left column"
    );
    assert!(at(beta, "Beta") > middle, "the second is in the right");
    assert!((alpha.y - beta.y).abs() < 0.5, "both start on the same row");
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

/// A psalm as BSB sets one: the chapter opens on a heading, the
/// superscription carries verse 1, and the psalm is poetry.
const PSALM: &str = concat!(
    "\\id PSA\n\\h Psalms\n\\c 3\n\\s1 Deliver Me\n\\d \\v 1 A Psalm of David.\n",
    "\\q1 O LORD, how my foes have increased!\n",
    "\\q2 How many rise up against me!\n",
    "\\q1 \\v 2 Many say of me,\n",
    "\\q2 God will not deliver him.\n",
    "\\q1 \\v 3 But You, O LORD, are a shield around me,\n",
);

/// **A chapter's opening, set as a collection of poems sets one**: a
/// caption in capitals over the number in Roman numerals, both before the
/// chapter's first heading rather than inside it, an ornament under them,
/// and the first letter dropped across three lines of poetry — the lines
/// after the first making room for it, and the fourth not.
#[test]
fn a_psalm_opens_with_its_caption_number_ornament_and_initial() {
    if !have_backend() {
        return;
    }
    const OPENING: &str = "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\n\
        drop_cap_of = \"first_letter\"\ndrop_cap_lines = 3\nchapter_ornament = \"rule\"\n\
        [numbering]\nchapter_caption = \"Chapter\"\npsalm_caption = \"Psalm\"\n\
        chapter_numerals = \"roman\"\n";
    const LOOK: &str = "[chapter_caption]\nown_line = true\nuppercase = true\n";
    let b = built(PSALM, OPENING, LOOK);
    let find = |text: &str| {
        b.lines
            .iter()
            .find(|l| l.page == 1 && l.text().replace(' ', "") == text)
            .unwrap_or_else(|| {
                panic!(
                    "no line {text:?} in {:?}",
                    b.lines.iter().map(|l| l.text()).collect::<Vec<_>>()
                )
            })
    };
    let caption = find("PSALM");
    let number = find("III");
    let heading = find("DeliverMe");
    assert!(caption.y > number.y, "the caption is above the number");
    assert!(number.y > heading.y, "and the number before the heading");
    let ornament: Vec<&Rule> = rules_on(&b, 1);
    assert_eq!(ornament.len(), 1, "one short rule: {ornament:?}");
    assert!(ornament[0].y1 < number.y && ornament[0].y1 > heading.y);

    // The poetry lines beside the initial stand right of it; the fourth
    // line does not. Measured at each line's body-size text.
    let left_of = |l: &Line| {
        l.marks
            .iter()
            .filter(|m| (m.size - BODY).abs() < 0.05)
            .map(|m| m.x)
            .fold(f64::INFINITY, f64::min)
    };
    let second = left_of(find("Howmanyriseupagainstme!"));
    let fourth = left_of(find("Godwillnotdeliverhim."));
    assert!(
        second > fourth + 10.0,
        "the second line makes room for the initial: {second} vs {fourth}"
    );
}

/// **A verse number in the margin takes its space with it**: the line a
/// verse opens starts where the lines under it do.
#[test]
fn a_verse_numbered_in_the_margin_starts_flush() {
    if !have_backend() {
        return;
    }
    let b = built(
        PSALM,
        "[page]\ncolumns = 1\n[numbering]\nverse_number_placement = \"left_margin\"\n",
        "[poetry.q1]\nindent = \"0pt\"\n[poetry.q2]\nindent = \"0pt\"\n",
    );
    let left_of = |text: &str| {
        let line = b
            .lines
            .iter()
            .find(|l| l.text().replace(' ', "").ends_with(text))
            .unwrap_or_else(|| panic!("no line ending {text:?}"));
        line.marks
            .iter()
            .filter(|m| (m.size - BODY).abs() < 0.05)
            .map(|m| m.x)
            .fold(f64::INFINITY, f64::min)
    };
    let opens = left_of("Manysayofme,");
    let under = left_of("Godwillnotdeliverhim.");
    assert!(
        (opens - under).abs() < 0.5,
        "the verse's line starts at {opens}, the line under it at {under}"
    );
}

/// **Side-heads**: with a side column, a section heading leaves the text and
/// is set in the column at its left, level with the first line of the
/// paragraph it opens; the text stands clear of the column and its gap, and
/// the note area does not.
#[test]
fn a_side_head_is_set_beside_its_paragraph() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\nside_column = \"120pt\"\nside_gap = \"12pt\"\n",
        "",
    );
    let heading = b
        .lines
        .iter()
        .find(|l| l.page == 1 && l.text().replace(' ', "").contains("TheFirmament"))
        .unwrap_or_else(|| {
            panic!(
                "no heading in {:?}",
                b.lines.iter().map(|l| (l.y, l.text())).collect::<Vec<_>>()
            )
        });
    // Its paragraph opens with verse 3, at the side column's far edge.
    let three = b
        .lines
        .iter()
        .flat_map(|l| &l.marks)
        .find(|m| m.text == "3" && (m.size - 6.4).abs() < 0.05)
        .expect("verse 3");
    assert!(
        (three.x - (b.left + 132.0)).abs() < 0.6,
        "the text stands clear of the column: verse 3 at {}, the frame at {}",
        three.x,
        b.left
    );
    let head_right = heading.marks.iter().map(|m| m.x).fold(f64::MIN, f64::max);
    assert!(head_right < three.x, "the heading is in the column");
    assert!(heading.left() >= b.left - 0.5);
    // Level with the paragraph's first line, give or take the difference in
    // size between them.
    let first = b
        .lines
        .iter()
        .find(|l| l.marks.iter().any(|m| std::ptr::eq(m, three)))
        .expect("verse 3's line");
    assert!(
        (heading.y - first.y).abs() < 3.0,
        "the heading at {}, its paragraph's first line at {}",
        heading.y,
        first.y
    );
}

/// **An initial on a block of colour**: the `initial` style's `background`
/// is drawn behind the dropped letter, in its own ink, about the letter's
/// size; without it there is no block.
#[test]
fn the_dropped_initial_can_sit_on_a_block_of_colour() {
    if !have_backend() {
        return;
    }
    const DROP: &str = "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\n\
                        drop_cap_of = \"first_letter\"\ndrop_cap_lines = 3\n";
    let b = built(PARAGRAPHS, DROP, "[initial]\nbackground = \"#ff0000\"\n");
    let blocks: Vec<&Rule> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| r.color == (1.0, 0.0, 0.0))
        .collect();
    assert_eq!(blocks.len(), 1, "{:?}", rules_on(&b, 1));
    // Three lines of 11.2pt, near enough, and wider than the letter, an I.
    assert!(
        blocks[0].thickness > 25.0 && blocks[0].length() > 10.0,
        "{:?}",
        blocks[0]
    );

    let plain = built(PARAGRAPHS, DROP, "");
    assert!(rules_on(&plain, 1).is_empty(), "{:?}", rules_on(&plain, 1));
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

/// **A centre column**: with `page.note_column` the notes leave the foot and
/// are set between the two text columns, in a column of that width with the
/// column gap either side, ruled off down both sides; the text columns
/// narrow to make room.
#[test]
fn the_notes_are_set_in_a_centre_column_between_the_text_columns() {
    if !have_backend() {
        return;
    }
    let usfm = PARAGRAPHS.replace(
        "were the first day.",
        "were the first day.\\f + \\fr 1:3 \\ft Zebulun is not here.\\f*",
    );
    let b = built(
        &usfm,
        "[page]\ncolumns = 2\ncolumn_gap = \"10pt\"\nnote_column = \"60pt\"\n\
         column_rule = \"0.5pt\"\n",
        "",
    );
    let column = (b.right - b.left - 60.0 - 20.0) / 2.0;
    let (from, to) = (b.left + column + 10.0, b.right - column - 10.0);
    let centre: String = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .flat_map(|l| &l.marks)
        .filter(|m| m.x > from - 0.5 && m.x < to)
        .map(|m| m.text.as_str())
        .collect();
    assert!(
        centre.replace(' ', "").contains("Zebulunisnot"),
        "the note is in the centre column, between {from} and {to}: {centre:?}"
    );
    // Not at the foot as well: nothing of it outside the centre.
    let outside: String = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .flat_map(|l| &l.marks)
        .filter(|m| m.x < from - 0.5 || m.x >= to)
        .map(|m| m.text.as_str())
        .collect();
    assert!(!outside.contains("Zebulun"), "{outside:?}");
    // Ruled off down both sides, at the column's edges.
    let downs: Vec<f64> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| (r.x1 - r.x2).abs() < 0.1 && r.length() > 100.0)
        .map(|r| r.x1)
        .collect();
    assert_eq!(downs.len(), 2, "{downs:?}");
    assert!(downs.iter().any(|x| (x - from).abs() < 1.0), "{downs:?}");
    assert!(downs.iter().any(|x| (x - to).abs() < 1.0), "{downs:?}");
}

/// **A chapter caption in the translation's own word**: with a caption set
/// **A dropped chapter number under a hidden label** drops into the first
/// paragraph, as it does under a heading, and is not set on a line of its
/// own where the label was.
#[test]
fn a_dropped_number_under_a_hidden_label_drops_into_the_paragraph() {
    if !have_backend() {
        return;
    }
    let usfm = concat!(
        "\\id GEN\n\\h Genesis\n\\c 1\n\\cl Kapitola 1\n\\p\n",
        "\\v 1 In the beginning God created the heaven and the earth. And the ",
        "earth was without form, and void; and darkness was upon the face of ",
        "the deep. And the Spirit of God moved upon the face of the waters.\n",
    );
    let b = built(
        usfm,
        "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\ndrop_cap_of = \"chapter_number\"\n\
         drop_cap_lines = 3\n[numbering]\nshow_chapter_labels = false\n",
        "",
    );
    let page: Vec<_> = b.lines.iter().filter(|l| l.page == 1).collect();
    let number = page
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| m.text == "1")
        .max_by(|a, c| a.size.partial_cmp(&c.size).expect("sizes"))
        .expect("the dropped number");
    // Beside the paragraph: its baseline is level with one of the
    // paragraph's lines, and the text starts to its right.
    let beside = page
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| m.size < number.size && m.x > number.x)
        .any(|m| (m.y - number.y).abs() < 1.5);
    assert!(
        beside,
        "{number:?} {:?}",
        page.iter().map(|l| (l.y, l.text())).collect::<Vec<_>>()
    );
    assert!(
        !page.iter().any(|l| l.text().contains("Kapitola")),
        "the label is hidden"
    );
}

/// and the text labelling its chapters `\cl Kapitola 1`, the caption takes
/// the label's word and the label is not set again as a line of its own; a
/// chapter with no label is captioned in the project language's word.
#[test]
fn the_chapter_caption_is_the_translation_s_own_word() {
    if !have_backend() {
        return;
    }
    let usfm = concat!(
        "\\id GEN\n\\h Genesis\n\\c 1\n\\cl Kapitola 1\n\\p\n",
        "\\v 1 In the beginning God created the heaven and the earth.\n",
    );
    let b = built(
        usfm,
        "[page]\ncolumns = 1\n[numbering]\nchapter_caption = \"Chapter\"\n",
        "[chapter]\nown_line = true\n",
    );
    let text: Vec<String> = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .map(|l| l.text())
        .collect();
    let captions = text.iter().filter(|t| t.contains("Kapitola")).count();
    assert_eq!(captions, 1, "the label's word, once: {text:?}");
    assert!(
        !text.iter().any(|t| t.contains("Chapter")),
        "not the template's English: {text:?}"
    );

    // No label, and the project in Spanish: the caption is Spanish's word.
    let usfm = concat!(
        "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
        "\\v 1 En el principio creó Dios los cielos y la tierra.\n",
    );
    let b = built(
        usfm,
        "[project]\nlanguage = \"es\"\n[page]\ncolumns = 1\n\
         [numbering]\nchapter_caption = \"Chapter\"\n",
        "[chapter]\nown_line = true\n",
    );
    let text: Vec<String> = b
        .lines
        .iter()
        .filter(|l| l.page == 1)
        .map(|l| l.text())
        .collect();
    assert!(
        text.iter().any(|t| t.contains("Capítulo")),
        "Spanish's word: {text:?}"
    );
}

/// **A thick-and-thin frame** is a box at the frame's weight and one inside
/// it at half that.
#[test]
fn a_thick_and_thin_frame_is_heavy_outside_and_light_inside() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\nframe = \"thick_thin\"\nframe_width = \"1.2pt\"\nframe_inset = \"0.25in\"\n",
        "",
    );
    let mut tall: Vec<_> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| r.is_vertical() && r.length() > b.height * 0.8)
        .collect();
    assert_eq!(tall.len(), 4, "{:?}", b.rules);
    tall.sort_by(|a, c| a.x1.partial_cmp(&c.x1).expect("numbers"));
    assert!(
        (tall[0].thickness - 1.2).abs() < 0.05,
        "outer: {:?}",
        tall[0]
    );
    assert!(
        (tall[1].thickness - 0.6).abs() < 0.05,
        "inner: {:?}",
        tall[1]
    );
    assert!(tall[1].x1 - tall[0].x1 > 3.0, "{:?}", &tall[..2]);
}

/// **The initial's border can stand off its block, in its own ink**: four
/// sides in the border colour, outside the block of colour.
#[test]
fn the_initial_s_border_can_stand_off_its_block_in_its_own_ink() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\n[contents]\ndrop_caps = true\ndrop_cap_of = \"first_letter\"\n",
        "[initial]\nbackground = \"#ff0000\"\nborder = true\nborder_width = \"0.6pt\"\n\
         border_color = \"#00ff00\"\nborder_gap = \"2pt\"\n",
    );
    let rules = rules_on(&b, 1);
    let block = rules
        .iter()
        .find(|r| r.color == (1.0, 0.0, 0.0))
        .expect("the block");
    let line: Vec<_> = rules
        .iter()
        .filter(|r| r.color == (0.0, 1.0, 0.0))
        .collect();
    assert_eq!(line.len(), 4, "{rules:?}");
    // The block is drawn as one wide rule; its left edge is its x.
    let block_left = block.x1.min(block.x2);
    let line_left = line.iter().map(|r| r.x1.min(r.x2)).fold(f64::MAX, f64::min);
    assert!(
        block_left - line_left > 2.0,
        "the line stands off the block: {block:?} {line:?}"
    );
}

/// **Folio rules** are two short rules either side of the page number.
#[test]
fn the_page_number_can_have_a_rule_either_side() {
    if !have_backend() {
        return;
    }
    let b = built(
        PARAGRAPHS,
        "[page]\ncolumns = 1\nfolio_rules = true\nfooter_rule_color = \"#00ff00\"\n",
        "",
    );
    let short: Vec<_> = rules_on(&b, 1)
        .into_iter()
        .filter(|r| r.color == (0.0, 1.0, 0.0) && r.is_horizontal())
        .collect();
    assert_eq!(short.len(), 2, "{:?}", b.rules);
    let middle = (b.left + b.right) / 2.0;
    assert!(
        short.iter().any(|r| r.x2.max(r.x1) < middle)
            && short.iter().any(|r| r.x1.min(r.x2) > middle),
        "one each side of the middle: {short:?}"
    );
    let plain = built(PARAGRAPHS, ONE, "");
    assert!(rules_on(&plain, 1).is_empty(), "{:?}", plain.rules);
}

/// **Poetry rules**: one rule over a run of poetry lines and one under it,
/// and none between its lines.
#[test]
fn a_run_of_poetry_is_ruled_above_and_below() {
    if !have_backend() {
        return;
    }
    const SONG: &str = concat!(
        "\\id LUK\n\\h Luke\n\\c 2\n\\p\n",
        "\\v 28 Simeon took Him in his arms and blessed God, saying:\n",
        "\\q1 \\v 29 Sovereign Lord, as You have promised,\n",
        "\\q2 You now dismiss Your servant in peace.\n",
        "\\q1 \\v 30 For my eyes have seen Your salvation,\n",
        "\\p\n",
        "\\v 33 The Child's father and mother were amazed.\n",
    );
    let ruled = built(
        SONG,
        "[page]\ncolumns = 1\n[contents]\npoetry_rules = true\nornament_rule_color = \"#00ff00\"\n",
        "",
    );
    let rules: Vec<_> = rules_on(&ruled, 1)
        .into_iter()
        .filter(|r| r.color == (0.0, 1.0, 0.0))
        .collect();
    assert_eq!(rules.len(), 2, "{:?}", ruled.rules);
    // Inset from the column on both sides.
    for r in &rules {
        assert!(r.x1.min(r.x2) > ruled.left + 1.0, "{r:?}");
        assert!(r.x1.max(r.x2) < ruled.right - 1.0, "{r:?}");
    }
    let plain = built(SONG, ONE, "");
    assert!(rules_on(&plain, 1).is_empty(), "{:?}", plain.rules);
}

/// A book that opens on a lead-in title line and two sections, for the
/// modern opening.
const LEAD_IN: &str = concat!(
    "\\id JHN\n\\h John\n\\mt2 The Gospel According to\n\\mt1 John\n\\c 1\n",
    "\\s1 The Word\n\\p\n",
    "\\v 1 In the beginning was the Word, and the Word was with God. ",
    "\\v 2 He was with God in the beginning.\n",
    "\\s1 The Witness\n\\p\n",
    "\\v 6 There came a man who was sent from God.\n",
);

/// **The modern opening**: the title's lead-in beside it in the side column,
/// a rule across the page under them in place of the head's, each section's
/// verses under its side-head, and the publication's name in the foot.
#[test]
fn a_modern_opening_sets_the_lead_in_the_ranges_and_the_title() {
    if !have_backend() {
        return;
    }
    let b = built(
        LEAD_IN,
        "[project]\nname = \"Test Version\"\n\
         [page]\ncolumns = 1\nside_column = \"90pt\"\n\
         header_rule = \"0.6pt\"\nheader_rule_color = \"#00ff00\"\nrule_color = \"#0000ff\"\n\
         [contents]\nsection_ranges = true\nshow_title_lead = true\ntitle_ornament = \"underline\"\n\
         [headers.right_page]\nfooter_left = \"{Title}\"\n",
        "[book]\nalign = \"start\"\n",
    );
    let page: Vec<_> = b.lines.iter().filter(|l| l.page == 1).collect();
    // The lead-in, in capitals, to the left of the title, on its baseline.
    // Letter-spaced, so a letter to a mark: its line's first mark is it.
    let lead = page
        .iter()
        .find(|l| l.text().contains("ACCORDING"))
        .and_then(|l| l.marks.first())
        .expect("the lead-in");
    // The title, not the head's "John".
    let title = page
        .iter()
        .flat_map(|l| &l.marks)
        .filter(|m| m.text.contains("John"))
        .max_by(|a, c| a.size.partial_cmp(&c.size).expect("sizes"))
        .expect("the title");
    assert!(lead.x < title.x, "{lead:?} {title:?}");
    assert!((lead.y - title.y).abs() < 1.0, "{lead:?} {title:?}");
    // Each section's span, on a line of its own, and the publication's name.
    for text in ["1:1–2", "1:6", "TestVersion"] {
        assert!(
            page.iter()
                .any(|l| l.text() == text || l.text().starts_with(text)),
            "`{text}`: {:?}",
            page.iter().map(|l| l.text()).collect::<Vec<_>>()
        );
    }
    // The title's rule, across the side column too, and no head rule.
    let rules = rules_on(&b, 1);
    assert!(
        rules.iter().all(|r| r.color != (0.0, 1.0, 0.0)),
        "{rules:?}"
    );
    let under = rules
        .iter()
        .find(|r| r.color == (0.0, 0.0, 1.0) && r.is_horizontal())
        .expect("the title's rule");
    assert!(under.x1.min(under.x2) < b.left + 1.0, "{under:?}");
}
