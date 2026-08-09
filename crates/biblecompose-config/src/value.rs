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

/// Levenshtein distance, two rows at a time.
fn distance(a: &str, b: &str) -> usize {
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
