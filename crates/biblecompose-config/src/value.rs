//! The vocabulary a settings file is written in: lengths, page sizes,
//! choices, ranges.
//!
//! ARCHITECTURE §6: "Units are parsed, not passed through. A string that
//! reaches the emitter is a bug, because it means an invalid unit will be
//! diagnosed by SILE, in SILE's words, at the wrong layer." So every value
//! here turns into a typed thing at the configuration boundary or into a
//! diagnostic with a line and a column — there is no third outcome.
//!
//! These read [`Node`]s rather than strings, because a diagnostic that cannot
//! say *where* the bad unit is has moved the problem rather than solved it.

use std::fmt;

use biblecompose_diagnostics::{code, Diagnostic};

use crate::document::{Located, Node};

/// A unit a length may be written in.
///
/// Absolute units only. SILE also understands `%pw`, `%ph` and `em`, and the
/// class's own fallbacks are written that way so they work at any page size —
/// but a relative length cannot be range-checked or compared against the page
/// without resolving it first, and nothing in SRS §7.3 asks a publisher to
/// write one. A percentage is therefore diagnosed rather than accepted, with a
/// message that says what to write instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Pt,
    Pc,
    In,
    Mm,
    Cm,
}

impl Unit {
    const ALL: [(&'static str, Unit); 5] = [
        ("pt", Unit::Pt),
        ("pc", Unit::Pc),
        ("in", Unit::In),
        ("mm", Unit::Mm),
        ("cm", Unit::Cm),
    ];

    /// How many points one of these is. The typographic point, 1/72 inch —
    /// which is what SILE means by `pt` too.
    const fn in_points(self) -> f64 {
        match self {
            Unit::Pt => 1.0,
            Unit::Pc => 12.0,
            Unit::In => 72.0,
            Unit::Mm => 72.0 / 25.4,
            Unit::Cm => 72.0 / 2.54,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Unit::Pt => "pt",
            Unit::Pc => "pc",
            Unit::In => "in",
            Unit::Mm => "mm",
            Unit::Cm => "cm",
        }
    }
}

/// A length, normalised to points and remembering how it was written.
///
/// Points because that is the one unit every comparison, sum and range check
/// can be done in without a conversion at each site; the original unit because
/// a settings form that shows a publisher `39.6pt` when they typed `0.55in`
/// has lost an argument it did not need to have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    points: f64,
    unit: Unit,
}

impl Length {
    pub const fn points(self) -> f64 {
        self.points
    }

    pub const fn unit(self) -> Unit {
        self.unit
    }

    pub fn from_points(points: f64) -> Length {
        Length {
            points,
            unit: Unit::Pt,
        }
    }

    /// In the unit it was written in — what a form field should show.
    pub fn in_written_unit(self) -> f64 {
        self.points / self.unit.in_points()
    }

    /// What goes to the backend.
    ///
    /// Always points, always the same formatting. A length that reached SILE
    /// as `0.55in` on one machine and `0.550in` on another would be a
    /// difference in the class options and therefore, eventually, a
    /// difference in a golden file (DET-001).
    pub fn to_sile(self) -> String {
        format!("{}pt", trim(self.points))
    }
}

/// Fixed precision, then trailing zeros removed — so the text is a function of
/// the value and not of the platform's float formatter.
fn trim(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", trim(self.in_written_unit()), self.unit.as_str())
    }
}

/// Split `"0.55in"` into its number and its unit.
fn split_unit(text: &str) -> Option<(f64, Unit)> {
    let text = text.trim();
    let digits = text
        .rfind(|c: char| c.is_ascii_digit() || c == '.')
        .map(|i| i + 1)?;
    let (number, suffix) = text.split_at(digits);
    let unit = Unit::ALL
        .iter()
        .find(|(name, _)| suffix.trim().eq_ignore_ascii_case(name))
        .map(|(_, u)| *u)?;
    let value: f64 = number.trim().parse().ok()?;
    value.is_finite().then_some((value, unit))
}

fn bad_unit(node: &Node, text: &str, what: &str) -> Diagnostic {
    let mut d = Diagnostic::error(
        code::INVALID_UNIT,
        format!("`{}` is not {what}: {text:?}", node.dotted_path()),
    )
    .at(node.loc());

    // The two mistakes worth naming, because both are things a person would
    // reasonably write and neither is a typo they will spot by rereading.
    d = if text.contains('%') {
        d.help(
            "percentages of the page are not supported in settings; \
             write an absolute length such as \"0.55in\"",
        )
    } else if text.trim().parse::<f64>().is_ok() {
        d.help("a length needs a unit — write \"12pt\" rather than \"12\"")
    } else {
        d.help("write a number and a unit, such as \"0.55in\", \"14mm\" or \"11.5pt\"")
    };

    d
}

/// A length such as `"0.55in"`.
pub fn length(node: &Node) -> Result<Located<Length>, Diagnostic> {
    let text = node.string()?;
    let (value, unit) =
        split_unit(&text.value).ok_or_else(|| bad_unit(node, &text.value, "a length"))?;

    // `<=` rather than `!(> 0.0)` is safe because `split_unit` has already
    // rejected anything that is not finite.
    if value <= 0.0 {
        return Err(Diagnostic::error(
            code::INVALID_VALUE,
            format!(
                "`{}` must be greater than zero, but it is {text}",
                node.dotted_path(),
                text = text.value
            ),
        )
        .at(node.loc()));
    }

    Ok(Located {
        value: Length {
            points: value * unit.in_points(),
            unit,
        },
        loc: text.loc,
    })
}

/// A length that is allowed to be zero — a margin, a gap, a space above.
pub fn length_or_zero(node: &Node) -> Result<Located<Length>, Diagnostic> {
    let text = node.string()?;
    let (value, unit) =
        split_unit(&text.value).ok_or_else(|| bad_unit(node, &text.value, "a length"))?;

    if value < 0.0 {
        return Err(Diagnostic::error(
            code::INVALID_VALUE,
            format!(
                "`{}` cannot be negative, but it is {}",
                node.dotted_path(),
                text.value
            ),
        )
        .at(node.loc()));
    }

    Ok(Located {
        value: Length {
            points: value * unit.in_points(),
            unit,
        },
        loc: text.loc,
    })
}

/// A trim size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width: Length,
    pub height: Length,
}

impl PageSize {
    /// SILE's `papersize`, in points for the same reason lengths are.
    pub fn to_sile(&self) -> String {
        format!("{} x {}", self.width.to_sile(), self.height.to_sile())
    }
}

impl fmt::Display for PageSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `6x9in` if both were written the same way, which is how they are
        // written; `6in x 297mm` only if someone mixed them.
        if self.width.unit == self.height.unit {
            write!(
                f,
                "{}x{}{}",
                trim(self.width.in_written_unit()),
                trim(self.height.in_written_unit()),
                self.width.unit.as_str()
            )
        } else {
            write!(f, "{} x {}", self.width, self.height)
        }
    }
}

/// The named sizes, in millimetres. Kept short on purpose: a name that is not
/// here is a `WxH` away, and a long table is a long list of things to be
/// subtly wrong about.
const NAMED: [(&str, f64, f64); 8] = [
    ("a4", 210.0, 297.0),
    ("a5", 148.0, 210.0),
    ("a6", 105.0, 148.0),
    ("b5", 176.0, 250.0),
    ("letter", 215.9, 279.4),
    ("legal", 215.9, 355.6),
    ("trade", 152.4, 228.6),   // 6x9in, the common Bible trim
    ("compact", 108.0, 152.0), // a hand-sized New Testament
];

/// The smallest and largest page BibleCompose will lay out, in points.
///
/// Not a matter of taste: SILE will accept a 3pt page and then fail deep in
/// frame solving with a message about glue, which is the wrong layer
/// diagnosing the wrong thing.
const MIN_PAGE: f64 = 72.0; // 1in
const MAX_PAGE: f64 = 72.0 * 48.0; // 48in

/// `"6x9in"`, `"210x297mm"`, `"6in x 9in"`, or a name such as `"a5"`.
pub fn page_size(node: &Node) -> Result<Located<PageSize>, Diagnostic> {
    let text = node.string()?;
    let raw = text.value.trim();
    let lower = raw.to_ascii_lowercase();

    let size = NAMED
        .iter()
        .find(|(name, _, _)| *name == lower)
        .map(|&(_, w, h)| PageSize {
            width: Length {
                points: w * Unit::Mm.in_points(),
                unit: Unit::Mm,
            },
            height: Length {
                points: h * Unit::Mm.in_points(),
                unit: Unit::Mm,
            },
        })
        .or_else(|| parse_dimensions(&lower))
        .ok_or_else(|| {
            bad_unit(node, raw, "a page size").help(format!(
                "write two dimensions such as \"6x9in\", or one of: {}",
                NAMED
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

    for (side, len) in [("width", size.width), ("height", size.height)] {
        if !(MIN_PAGE..=MAX_PAGE).contains(&len.points) {
            return Err(Diagnostic::error(
                code::INVALID_VALUE,
                format!(
                    "`{}` has a {side} of {len}, which is outside the 1in to 48in \
                     BibleCompose can lay out",
                    node.dotted_path()
                ),
            )
            .at(node.loc())
            .help("a page this size fails inside SILE's frame solver, where the message would not mention the page size at all"));
        }
    }

    Ok(Located {
        value: size,
        loc: text.loc,
    })
}

/// `6x9in` — one unit for both — or `6in x 9in`, where each carries its own.
fn parse_dimensions(text: &str) -> Option<PageSize> {
    let (left, right) = text.split_once('x')?;
    let (left, right) = (left.trim(), right.trim());

    // The common spelling puts the unit only on the second number.
    if let Some((w, unit)) = split_unit(&format!("{left}{}", suffix_of(right)?)) {
        if let Some((h, _)) = split_unit(right) {
            return Some(PageSize {
                width: Length {
                    points: w * unit.in_points(),
                    unit,
                },
                height: Length {
                    points: h * unit.in_points(),
                    unit,
                },
            });
        }
    }

    let (w, wu) = split_unit(left)?;
    let (h, hu) = split_unit(right)?;
    Some(PageSize {
        width: Length {
            points: w * wu.in_points(),
            unit: wu,
        },
        height: Length {
            points: h * hu.in_points(),
            unit: hu,
        },
    })
}

fn suffix_of(text: &str) -> Option<&'static str> {
    Unit::ALL
        .iter()
        .find(|(name, _)| text.to_ascii_lowercase().ends_with(name))
        .map(|(name, _)| *name)
}

/// A closed set of spellings, its table, and the two impls every one of them
/// needs.
///
/// Written once because the head slots were the first of five such types and
/// the fifth would have been the fifth hand-kept `NAMES` table, `as_str` and
/// `Display` — three places per type where a variant can be added to one and
/// forgotten in the others. (The head slots have since become templates, and
/// are no longer one of these.) The table is the single statement of the vocabulary: `as_str` reads
/// it, `Display` defers to `as_str`, and [`choice`] both parses from it and
/// lists it in the diagnostic when a file says something else.
macro_rules! spelled {
    (
        $(#[$outer:meta])*
        $name:ident { $( $(#[$inner:meta])* $variant:ident => $text:literal ),* $(,)? }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $( $(#[$inner])* $variant, )*
        }

        impl $name {
            pub const NAMES: &'static [(&'static str, $name)] =
                &[ $( ($text, $name::$variant) ),* ];

            /// The same vocabulary without the values, for a form to offer.
            pub const SPELLINGS: &'static [&'static str] = &[ $( $text ),* ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $text, )*
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

spelled! {
    /// The marks that key a note to the place it belongs to (SCR-003).
    ///
    /// USFM's `\f` carries a caller of its own — `+` asks for the next mark in
    /// the sequence, `-` asks for none, and anything else is the mark the
    /// editor chose and is printed as written. This setting is what `+` means,
    /// and it is a setting because the answer differs by house and by how many
    /// notes a page carries: a page with three notes reads well with symbols
    /// and a page with thirty does not.
    CallerStyle {
        /// `1`, `2`, `3`.
        Numbers => "numbers",
        /// `a`, `b`, `c`, and `aa` after `z`.
        Letters => "letters",
        /// `*`, `†`, `‡`, `§`, `‖`, `¶`, then doubled.
        Symbols => "symbols",
        /// No mark at all. The note is keyed by the reference it carries,
        /// which is what an edition setting notes in a column of their own
        /// does.
        None => "none",
    }
}

spelled! {
    /// Where a caller sequence starts again.
    ///
    /// Not per page, and the omission is deliberate. A caller is typeset into
    /// the paragraph that calls it, and SILE breaks a page only after that
    /// whole paragraph is set — so at the moment the mark is drawn, which page
    /// it will land on is not yet decided. A per-page policy would therefore be
    /// right in the middle of a page and wrong at both its edges, which is
    /// worse than not offering it. Per chapter is the common Bible convention
    /// and is exactly determined at the point the mark is made.
    RestartNumbering {
        PerChapter => "per_chapter",
        PerBook => "per_book",
        /// One sequence for the whole publication.
        Never => "never",
    }
}

spelled! {
    /// Where a cross-reference is set (SCR-005).
    ///
    /// The MVP's two answers, and no third: centre-column references are
    /// explicitly post-MVP, and a gutter is a page-geometry decision rather
    /// than a note one.
    ReferencePlacement {
        /// At the foot, beside the footnotes and styled apart from them.
        NoteArea => "note_area",
        /// In the text, where the reference was called, in brackets.
        Inline => "inline",
        /// Gathered and set as a line under the paragraph that called them.
        EndOfParagraph => "end_of_paragraph",
    }
}

spelled! {
    /// How much of the text a PDF can be pointed at (SCR-008).
    ///
    /// **The default is chapters, and that is a measurement rather than a
    /// preference.** One destination per verse cost 15% of the build time and
    /// 14% of the file size on a 4,950-verse document — and nothing in this
    /// release points at one, because no cross-reference is a link yet. What a
    /// reader uses today is the outline, which chapters give for almost
    /// nothing. Verse anchors are there for whoever wants a document that can
    /// be linked into from outside, and for the release that turns references
    /// into links; until then they are a cost with no reader on the other end.
    Anchors {
        /// A destination for each book and chapter, and an outline of both.
        Chapter => "chapter",
        /// And one for every verse: `JHN.3.16`.
        Verse => "verse",
        /// None at all, for a publication that is only ever going to be
        /// printed.
        None => "none",
    }
}

spelled! {
    /// What drops into the text when a chapter opens with a drop cap.
    DropCap {
        /// The chapter's first letter — its first syllable, in a script
        /// that writes one as several characters. The chapter number then
        /// takes a line of its own above it.
        FirstLetter => "first_letter",
        /// The chapter number itself, set large and dropped into the
        /// opening lines, with the text's first letter left as it is.
        ChapterNumber => "chapter_number",
    }
}

spelled! {
    /// What a build does about a figure whose file is not there.
    ///
    /// Only this one question is a policy. A figure pointing *outside* the
    /// project is refused whatever this says, because that is a rule about
    /// what a project is rather than a preference about drafts (SRS §15); and
    /// a format the backend cannot read is refused too, because it is a
    /// mistake in the project rather than an asset that has not arrived yet.
    MissingAsset {
        /// Refuse to build. The default: a printed book with a hole where an
        /// illustration should be is worse than a build that says why.
        Stop => "stop",
        /// Leave the figure out, warn, and carry on — which is what a proof
        /// wants while the artwork is still being drawn.
        Omit => "omit",
    }
}

/// An ink colour, as `#rrggbb`.
///
/// # Why hex and not names
///
/// A red-letter edition is the reason this type exists, and "red" is exactly
/// the wrong way to ask for one: the red a Bible is printed in is a decision a
/// publisher makes with a press, and there are as many of them as there are
/// houses. A hex triple is the same colour every time, states it in the file,
/// and does not quietly change when a backend revises its palette.
///
/// Stored as three bytes rather than as the text that was written, so
/// `#FFF`, `#ffffff` and `#FFFFFF` are one value and produce one build
/// (DET-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

impl Color {
    /// `#rgb` or `#rrggbb`, in either case.
    ///
    /// The short form is expanded the way CSS expands it — each digit doubled,
    /// so `#f00` is `#ff0000` and not `#f00000`. That is what everyone who has
    /// ever written a short hex colour expects, and getting it wrong would
    /// darken every colour written that way by a hair nobody would trace.
    pub fn parse(text: &str) -> Option<Color> {
        let digits = text.trim().strip_prefix('#')?;
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |s: &str| u8::from_str_radix(s, 16).ok();
        match digits.len() {
            3 => {
                let mut c = digits.chars().map(|d| byte(&format!("{d}{d}")));
                Some(Color {
                    red: c.next()??,
                    green: c.next()??,
                    blue: c.next()??,
                })
            }
            6 => Some(Color {
                red: byte(&digits[0..2])?,
                green: byte(&digits[2..4])?,
                blue: byte(&digits[4..6])?,
            }),
            _ => None,
        }
    }
}

/// A colour, or a diagnostic saying what one looks like.
pub fn color(node: &Node) -> Result<Located<Color>, Diagnostic> {
    let text = node.string()?;
    match Color::parse(&text.value) {
        Some(value) => Ok(Located {
            value,
            loc: text.loc,
        }),
        None => Err(Diagnostic::error(
            code::INVALID_VALUE,
            format!(
                "`{}` is {:?}, which is not a colour",
                node.dotted_path(),
                text.value.trim()
            ),
        )
        .at(text.loc)
        .help("write a colour as `#rrggbb` — `#c81414` is a typical red-letter red")),
    }
}

/// One of a fixed set of spellings.
///
/// The allowed values are listed in the diagnostic, and the nearest one is
/// suggested when it is near enough to be a typo rather than a guess.
pub fn choice<T: Copy>(node: &Node, options: &[(&str, T)]) -> Result<Located<T>, Diagnostic> {
    let text = node.string()?;
    let given = text.value.trim();

    if let Some((_, value)) = options
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(given))
    {
        return Ok(Located {
            value: *value,
            loc: text.loc,
        });
    }

    let allowed = options
        .iter()
        .map(|(n, _)| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut d = Diagnostic::error(
        code::INVALID_VALUE,
        format!(
            "`{}` is {given:?}, which is not one of {allowed}",
            node.dotted_path()
        ),
    )
    .at(node.loc());

    if let Some(near) = nearest(given, options.iter().map(|(n, _)| *n)) {
        d = d.help(format!("did you mean \"{near}\"?"));
    }

    Err(d)
}

/// A number within bounds — columns, line spacing, a scale factor.
pub fn number_in(node: &Node, lo: f64, hi: f64) -> Result<Located<f64>, Diagnostic> {
    let n = node.number()?;
    if (lo..=hi).contains(&n.value) {
        return Ok(n);
    }
    Err(Diagnostic::error(
        code::INVALID_VALUE,
        format!(
            "`{}` is {}, but it must be between {} and {}",
            node.dotted_path(),
            trim(n.value),
            trim(lo),
            trim(hi)
        ),
    )
    .at(node.loc()))
}

/// A whole number within bounds.
pub fn integer_in(node: &Node, lo: i64, hi: i64) -> Result<Located<i64>, Diagnostic> {
    let n = node.integer()?;
    if (lo..=hi).contains(&n.value) {
        return Ok(n);
    }
    Err(Diagnostic::error(
        code::INVALID_VALUE,
        format!(
            "`{}` is {}, but it must be between {lo} and {hi}",
            node.dotted_path(),
            n.value
        ),
    )
    .at(node.loc()))
}

/// The closest candidate, if it is close enough to be a slip rather than a
/// different word. Two edits, which catches a transposition and a doubled or
/// dropped letter without proposing `"inline"` for `"footnote-area"`.
fn nearest<'a>(given: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let given = given.to_ascii_lowercase();
    candidates
        .map(|c| (distance(&given, &c.to_ascii_lowercase()), c))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Levenshtein distance, two rows at a time. Shared with the unknown-key
/// walk in `settings`, which suggests the nearest key for the same reason.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];

    for (i, ac) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, bc) in b.iter().enumerate() {
            let cost = usize::from(ac != bc);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod color_tests {
    use super::Color;

    #[test]
    fn the_short_form_doubles_each_digit() {
        // CSS's rule, and the one anyone writing `#f00` means. Padding with a
        // zero instead would darken every short colour by a hair.
        assert_eq!(Color::parse("#f00"), Color::parse("#ff0000"));
        assert_eq!(Color::parse("#abc"), Color::parse("#aabbcc"));
    }

    #[test]
    fn case_and_spacing_do_not_make_a_different_colour() {
        let one = Color::parse("  #C81414 ").expect("a colour");
        assert_eq!(one, Color::parse("#c81414").expect("a colour"));
        assert_eq!(one.to_string(), "#c81414", "one spelling reaches the build");
    }

    #[test]
    fn what_is_not_a_colour_is_refused() {
        for text in ["red", "#12345", "#gg0000", "c81414", "", "#"] {
            assert_eq!(Color::parse(text), None, "{text:?}");
        }
    }
}

// ------------------------------------------------------------------------
// Head and foot templates

/// A field a head or foot template can name.
///
/// The one statement of the vocabulary: the resolver checks a template
/// against it, the window lists it as documentation, and the example page
/// renders a template with the `example` of each. The class has its own copy
/// of the *names* — it is Lua and cannot read this — and the test in
/// `heads.rs` that sets every field is what keeps the two in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadField {
    /// The name inside the braces, in its canonical spelling. Matched without
    /// regard to case or underscores, so `{first_chapter}` and `{firstchapter}`
    /// are `{FirstChapter}`.
    pub name: &'static str,
    /// A short label, for a control that offers the field on its own.
    pub label: &'static str,
    /// What it puts on the page, in a sentence.
    pub description: &'static str,
    /// What it might read on a page of 1 John, for an example.
    pub example: &'static str,
}

/// Every field, in the order they are documented.
pub const HEAD_FIELDS: &[HeadField] = &[
    HeadField {
        name: "Book",
        label: "Book name",
        description: "The book's running-head name, from USFM's \\h.",
        example: "1 John",
    },
    HeadField {
        name: "AltBook",
        label: "Alternate book name",
        description: "The fuller form of the name: USFM's \\toc1, or the title.",
        example: "The First Epistle of John",
    },
    HeadField {
        name: "Page",
        label: "Page number",
        description: "The page number, in whatever numbering the page uses.",
        example: "413",
    },
    HeadField {
        name: "Range",
        label: "Reference range",
        description: "The span of Scripture on the page, first reference to last. A page inside one verse gives that verse alone.",
        example: "1:1–2:6",
    },
    HeadField {
        name: "FirstReference",
        label: "First reference",
        description: "Chapter and verse where the page starts.",
        example: "1:1",
    },
    HeadField {
        name: "LastReference",
        label: "Last reference",
        description: "Chapter and verse where the page ends.",
        example: "2:6",
    },
    HeadField {
        name: "FirstChapter",
        label: "First chapter",
        description: "The chapter the page starts in.",
        example: "1",
    },
    HeadField {
        name: "FirstVerse",
        label: "First verse",
        description: "The verse the page starts in.",
        example: "1",
    },
    HeadField {
        name: "LastChapter",
        label: "Last chapter",
        description: "The chapter the page ends in.",
        example: "2",
    },
    HeadField {
        name: "LastVerse",
        label: "Last verse",
        description: "The verse the page ends in.",
        example: "6",
    },
];

/// A name as it is compared: lower-case, underscores dropped.
fn fold(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

impl HeadField {
    /// The field this name means, however it is cased or underscored.
    pub fn named(name: &str) -> Option<&'static HeadField> {
        let want = fold(name.trim());
        HEAD_FIELDS.iter().find(|f| fold(f.name) == want)
    }

    /// Every field's name in braces, for a message that lists them.
    fn listed() -> String {
        HEAD_FIELDS
            .iter()
            .map(|f| format!("{{{}}}", f.name))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// What one head or foot slot holds: text, with fields in braces.
///
/// `"{Book} {Range}"` reads "1 John 1:1–2:6"; `""` is an empty slot. A field
/// is replaced by what it names on each page; text stays as written; `{{` and
/// `}}` are a brace of the publisher's own. Checked here, so that a
/// misspelled field is a diagnostic at its line and not a head reading
/// `{Bok}` on every page of the book.
///
/// The seven names the slots took before they were templates — `page_number`,
/// `book_name` and the rest — still read, as the template each one meant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadTemplate {
    text: String,
}

/// The old names and what each one is now. Read for old files; written by
/// nobody new.
const HEAD_ALIASES: &[(&str, &str)] = &[
    ("empty", ""),
    ("page_number", "{Page}"),
    ("reference_range", "{Range}"),
    ("first_reference", "{FirstReference}"),
    ("last_reference", "{LastReference}"),
    ("book_name", "{Book}"),
    ("alt_book_name", "{AltBook}"),
];

impl HeadTemplate {
    /// Read a template, refusing one that names a field that does not exist
    /// or leaves a brace open. The error is a sentence for the diagnostic.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        if let Some((_, meant)) = HEAD_ALIASES
            .iter()
            .find(|(old, _)| old.eq_ignore_ascii_case(trimmed))
        {
            return Ok(HeadTemplate {
                text: (*meant).to_owned(),
            });
        }

        let mut rest = text;
        while let Some(at) = rest.find(['{', '}']) {
            let brace = rest.as_bytes()[at];
            let after = &rest[at + 1..];
            // A doubled brace is a literal one.
            if after.as_bytes().first() == Some(&brace) {
                rest = &after[1..];
                continue;
            }
            if brace == b'}' {
                return Err(
                    "a `}` with no `{` before it; write `}}` for a brace of your own".into(),
                );
            }
            let Some(end) = after.find('}') else {
                return Err("a `{` with no `}` after it".into());
            };
            let name = &after[..end];
            if name.contains('{') {
                return Err(format!("a `{{` inside `{{{name}`; fields do not nest"));
            }
            if HeadField::named(name).is_none() {
                return Err(format!(
                    "`{{{name}}}` is not a field; the fields are {}",
                    HeadField::listed()
                ));
            }
            rest = &after[end + 1..];
        }
        Ok(HeadTemplate {
            text: text.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// An empty slot: nothing is set there.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl fmt::Display for HeadTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// A head or foot slot's template, checked.
pub fn head_template(node: &Node) -> Result<Located<HeadTemplate>, Diagnostic> {
    let text = node.string()?;
    match HeadTemplate::parse(&text.value) {
        Ok(value) => Ok(Located {
            value,
            loc: text.loc,
        }),
        Err(why) => Err(Diagnostic::error(
            code::INVALID_VALUE,
            format!(
                "`{}` is not a head or foot template: {why}",
                node.dotted_path()
            ),
        )
        .at(node.loc())
        .help(format!(
            "write text with fields in braces, such as \"{{Book}} {{Range}}\"; \
             the fields are {}",
            HeadField::listed()
        ))),
    }
}

#[cfg(test)]
mod head_template_tests {
    use super::*;

    #[test]
    fn fields_are_matched_without_regard_to_case_or_underscores() {
        for spelling in [
            "{FirstChapter}",
            "{firstchapter}",
            "{first_chapter}",
            "{FIRST_CHAPTER}",
        ] {
            assert!(HeadTemplate::parse(spelling).is_ok(), "{spelling}");
        }
        assert_eq!(
            HeadField::named("first_chapter").map(|f| f.name),
            Some("FirstChapter")
        );
    }

    #[test]
    fn text_around_the_fields_is_kept_as_written() {
        let t = HeadTemplate::parse("{Book}:{FirstChapter}-{FirstVerse}").expect("a template");
        assert_eq!(t.as_str(), "{Book}:{FirstChapter}-{FirstVerse}");
        assert!(HeadTemplate::parse("Page {Page} of the book").is_ok());
        assert!(HeadTemplate::parse("plain words").is_ok());
    }

    #[test]
    fn a_doubled_brace_is_a_brace() {
        assert!(HeadTemplate::parse("{{{Book}}}").is_ok());
        assert!(HeadTemplate::parse("{{not a field}}").is_ok());
    }

    #[test]
    fn a_field_that_does_not_exist_is_refused_with_the_list() {
        let why = HeadTemplate::parse("{Bok}").expect_err("no such field");
        assert!(why.contains("`{Bok}` is not a field"), "{why}");
        assert!(
            why.contains("{Book}") && why.contains("{LastVerse}"),
            "{why}"
        );
    }

    #[test]
    fn an_open_brace_is_refused() {
        assert!(HeadTemplate::parse("{Book").is_err());
        assert!(HeadTemplate::parse("Book}").is_err());
        assert!(HeadTemplate::parse("{Bo{ok}").is_err());
    }

    /// The old names still read, as the template each one meant.
    #[test]
    fn the_old_names_are_the_templates_they_meant() {
        assert_eq!(
            HeadTemplate::parse("page_number").unwrap().as_str(),
            "{Page}"
        );
        assert_eq!(HeadTemplate::parse("Book_Name").unwrap().as_str(), "{Book}");
        assert_eq!(HeadTemplate::parse("empty").unwrap().as_str(), "");
        assert!(HeadTemplate::parse("empty").unwrap().is_empty());
        assert!(HeadTemplate::parse("").unwrap().is_empty());
    }
}
