//! The book's cover: one PDF page holding the back, the spine and the front.
//!
//! # Why not through the typesetter
//!
//! A cover is a handful of centred lines and some rules on one page. Nothing
//! on it breaks into lines a typesetter would choose better, and running SILE
//! for it would make a cover wait on a backend that has to be found, unpacked
//! and started — and fail with it. So it is drawn here, directly, with the
//! same `lopdf` the front matter is arranged with, and the font embedded
//! whole from the file the project's family resolves to.
//!
//! The price is that the lettering is not shaped: one glyph per character,
//! from the font's character map. That is right for the Latin, Greek and
//! Cyrillic a cover is usually lettered in, and wrong for a script whose
//! letters join or reorder — which is reported, not silently set wrong.
//!
//! # The sheet
//!
//! Left to right as it is printed, outside up: the back, the spine, the front.
//! A paperback is those three at the trim with `cover.bleed` round them. A
//! hardcover is a case wrap — see [`Geometry`]. The page's `TrimBox` and
//! `BleedBox` say which is which, because that is what a printer's pre-flight
//! reads.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use biblecompose_config::value::{Color, CoverBinding, CoverBorder};
use biblecompose_config::Settings;
use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};
use lopdf::{dictionary, Dictionary, Document, Object, Stream, StringFormat};

use crate::project::Opened;

/// What the cover is called, inside the project's `output/` folder.
pub const COVER_FILE: &str = "book_cover.pdf";

const INCH: f64 = 72.0;

/// A hardcover's boards stand this far proud of the page at the head, the
/// foot and the fore-edge — the "squares".
pub const BOARD_SQUARE: f64 = 0.125 * INCH;
/// The groove either side of the spine, where the cover bends.
pub const HINGE: f64 = 0.375 * INCH;
/// What folds round the boards on every edge.
pub const WRAP: f64 = 0.75 * INCH;
/// One board's thickness, which the case's spine has to clear twice.
pub const BOARD_THICKNESS: f64 = 0.1 * INCH;

/// The narrowest spine that is lettered.
pub const MIN_LETTERED_SPINE: f64 = 0.25 * INCH;

/// Where the page count came from, which decides whether the spine can be
/// trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageSource {
    /// `cover.pages`.
    Setting,
    /// Counted in the book's PDF.
    Book(Utf8PathBuf),
    /// Estimated from the Scripture, because the book has not been built.
    Estimate,
    /// `cover.spine_width` was set, so the pages were not needed.
    NotNeeded,
}

/// Every measurement on the sheet, in points from its bottom-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub binding: CoverBinding,
    pub pages: u32,
    /// The whole sheet.
    pub width: f64,
    pub height: f64,
    /// Beyond the trim (paperback) or the boards (hardcover) on every side.
    pub margin: f64,
    /// One face — the trim of a page for a paperback, a board for a
    /// hardcover.
    pub face_width: f64,
    pub face_height: f64,
    /// Zero on a paperback.
    pub hinge: f64,
    pub spine: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    fn inset(self, by: f64) -> Rect {
        Rect {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - 2.0 * by).max(0.0),
            h: (self.h - 2.0 * by).max(0.0),
        }
    }
    fn centre_x(self) -> f64 {
        self.x + self.w / 2.0
    }
    fn centre_y(self) -> f64 {
        self.y + self.h / 2.0
    }
}

impl Geometry {
    /// The sheet for these settings and this many pages.
    pub fn new(settings: &Settings, pages: u32) -> Geometry {
        let c = &settings.cover;
        let trim_w = settings.page.size.width.points();
        let trim_h = settings.page.size.height.points();
        let text_block = if c.spine_width.points() > 0.0 {
            c.spine_width.points()
        } else {
            f64::from(pages) * c.paper.inches_per_page() * INCH
        };
        let binding = *c.binding;
        let (margin, face_width, face_height, hinge, spine) = match binding {
            CoverBinding::Paperback => (c.bleed.points(), trim_w, trim_h, 0.0, text_block),
            CoverBinding::Hardcover => (
                WRAP,
                trim_w + BOARD_SQUARE,
                trim_h + 2.0 * BOARD_SQUARE,
                HINGE,
                text_block + 2.0 * BOARD_THICKNESS,
            ),
        };
        Geometry {
            binding,
            pages,
            width: 2.0 * margin + 2.0 * face_width + 2.0 * hinge + spine,
            height: 2.0 * margin + face_height,
            margin,
            face_width,
            face_height,
            hinge,
            spine,
        }
    }

    pub fn back(&self) -> Rect {
        Rect {
            x: self.margin,
            y: self.margin,
            w: self.face_width,
            h: self.face_height,
        }
    }

    pub fn spine_rect(&self) -> Rect {
        Rect {
            x: self.margin + self.face_width + self.hinge,
            y: self.margin,
            w: self.spine,
            h: self.face_height,
        }
    }

    pub fn front(&self) -> Rect {
        Rect {
            x: self.margin + self.face_width + 2.0 * self.hinge + self.spine,
            y: self.margin,
            w: self.face_width,
            h: self.face_height,
        }
    }

    /// What the printer trims to: the faces and the spine for a paperback,
    /// the whole sheet for a case wrap (whose margin is folded, not cut).
    pub fn trim_box(&self) -> Rect {
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            w: self.width,
            h: self.height,
        };
        match self.binding {
            CoverBinding::Paperback => whole.inset(self.margin),
            CoverBinding::Hardcover => whole,
        }
    }
}

/// What making a cover came to.
#[derive(Debug, Default)]
pub struct CoverReport {
    /// Where it was written, when it was.
    pub output: Option<Utf8PathBuf>,
    pub geometry: Option<Geometry>,
    pub pages_from: Option<PageSource>,
    pub diagnostics: Diagnostics,
}

impl CoverReport {
    pub fn succeeded(&self) -> bool {
        self.output.is_some()
    }
}

/// Where the cover goes unless the caller says otherwise: beside the book.
pub fn default_output(opened: &Opened) -> Utf8PathBuf {
    opened
        .output()
        .parent()
        .map(|p| p.join(COVER_FILE))
        .unwrap_or_else(|| Utf8PathBuf::from(COVER_FILE))
}

/// How many pages the book has, and how that is known.
///
/// The setting, then the book's own PDF, then an estimate. `book` is where
/// the PDF would be; `None` from here is a cover that cannot be sized.
pub fn page_count(opened: &Opened, book: &Utf8Path) -> Option<(u32, PageSource)> {
    let c = &opened.settings.cover;
    if *c.pages > 0 {
        return Some((*c.pages, PageSource::Setting));
    }
    if c.spine_width.points() > 0.0 {
        return Some((0, PageSource::NotNeeded));
    }
    if let Ok(doc) = Document::load(book.as_std_path()) {
        let n = doc.get_pages().len() as u32;
        if n > 0 {
            return Some((n, PageSource::Book(book.to_owned())));
        }
    }
    crate::pages::estimate(&opened.document, &opened.settings).map(|n| (n, PageSource::Estimate))
}

/// Everything a cover is drawn from, worked out once for the PDF and the
/// preview alike.
struct Prepared {
    geometry: Geometry,
    from: PageSource,
    font: CoverFont,
    words: Words,
    lettered_spine: bool,
}

fn prepare(
    opened: &Opened,
    font_dirs: &[Utf8PathBuf],
    diagnostics: &mut Diagnostics,
) -> Option<Prepared> {
    let settings = &opened.settings;
    let c = &settings.cover;

    let Some((pages, from)) = page_count(opened, &opened.output()) else {
        diagnostics.push(
            Diagnostic::error(
                code::COVER_PAGES_UNKNOWN,
                "the cover's spine cannot be sized: nothing says how many pages the book has",
            )
            .help("build the book first, or set `cover.pages` or `cover.spine_width`"),
        );
        return None;
    };
    if from == PageSource::Estimate {
        diagnostics.push(
            Diagnostic::warning(
                code::COVER_PAGES_ESTIMATED,
                format!(
                    "the spine is sized for an estimated {pages} pages, because the book has not been built"
                ),
            )
            .help("build the book, or set `cover.pages` to the printer's count, before sending the cover to print"),
        );
    }

    let geometry = Geometry::new(settings, pages);

    let family = if c.font_family.trim().is_empty() {
        settings.typography.font_family.as_str()
    } else {
        c.font_family.as_str()
    };
    let font = match load_font(family, &opened.root, font_dirs) {
        Ok(f) => f,
        Err(d) => {
            diagnostics.push(d);
            return None;
        }
    };

    let words = Words::from(settings);
    let missing = font.missing(&words.all());
    if !missing.is_empty() {
        let shown: String = missing.iter().take(12).collect();
        diagnostics.push(
            Diagnostic::warning(
                code::MISSING_COVERAGE,
                format!("{family} cannot draw some of the cover's lettering: {shown}"),
            )
            .help("set `cover.font_family` to a font that has those characters"),
        );
    }

    let lettered_spine = geometry.spine >= MIN_LETTERED_SPINE;
    if !lettered_spine && !(words.spine.is_empty() && words.spine_foot.is_empty()) {
        diagnostics.push(
            Diagnostic::warning(
                code::COVER_SPINE_NARROW,
                format!(
                    "the spine is {:.3}in wide, too narrow to letter, so it is left plain",
                    geometry.spine / INCH
                ),
            )
            .help("a spine under 1/4in cannot carry legible type"),
        );
    }

    Some(Prepared {
        geometry,
        from,
        font,
        words,
        lettered_spine,
    })
}

/// The cover as the window previews it: an SVG of the whole sheet, drawn by
/// the same code as the PDF, and what making it had to say.
#[derive(Debug, Default)]
pub struct Preview {
    /// `None` when there is nothing to show — no page count, no font.
    pub svg: Option<String>,
    pub geometry: Option<Geometry>,
    pub pages_from: Option<PageSource>,
    pub diagnostics: Diagnostics,
}

/// Draw the cover without writing anything. `guides` draws the trim, the
/// spine and the hinges whatever `cover.show_guides` says, because a preview
/// is exactly where they are wanted.
pub fn preview(opened: &Opened, font_dirs: &[Utf8PathBuf], guides: bool) -> Preview {
    let mut out = Preview::default();
    let Some(p) = prepare(opened, font_dirs, &mut out.diagnostics) else {
        return out;
    };
    let guides = guides || *opened.settings.cover.show_guides;
    let marks = draw(
        &opened.settings,
        &p.geometry,
        &p.font,
        &p.words,
        p.lettered_spine,
        guides,
    );
    out.svg = Some(to_svg(&marks, &p.geometry, &p.font));
    out.geometry = Some(p.geometry);
    out.pages_from = Some(p.from);
    out
}

/// Make the cover for an opened project, and write it to `output`.
///
/// `font_dirs` are searched after the project's own fonts and before the
/// system's, as the build searches them.
pub fn make(opened: &Opened, output: &Utf8Path, font_dirs: &[Utf8PathBuf]) -> CoverReport {
    let mut report = CoverReport::default();
    let settings = &opened.settings;
    let Some(Prepared {
        geometry,
        from,
        font,
        words,
        lettered_spine,
    }) = prepare(opened, font_dirs, &mut report.diagnostics)
    else {
        return report;
    };

    let marks = draw(
        settings,
        &geometry,
        &font,
        &words,
        lettered_spine,
        *settings.cover.show_guides,
    );
    let bytes = match write_pdf(settings, &geometry, &font, to_pdf(&marks)) {
        Ok(b) => b,
        Err(e) => {
            report.diagnostics.push(
                Diagnostic::error(code::PUBLISH_FAILED, "the cover could not be written").detail(e),
            );
            return report;
        }
    };

    // Written beside the destination and moved over it, so a cover open in a
    // viewer is reported as locked rather than left half-written.
    let staging = Utf8PathBuf::from(format!("{output}.partial"));
    if let Some(parent) = staging.parent() {
        if !parent.as_str().is_empty() {
            let _ = std::fs::create_dir_all(parent.as_std_path());
        }
    }
    if let Err(e) = std::fs::write(staging.as_std_path(), &bytes) {
        report.diagnostics.push(
            Diagnostic::error(
                code::DESTINATION_UNWRITABLE,
                format!("could not write {output}"),
            )
            .at(SourceLoc::file(output.to_owned()))
            .detail(e.to_string()),
        );
        return report;
    }
    if let Err(d) = crate::publish::publish(&staging, output) {
        let _ = std::fs::remove_file(staging.as_std_path());
        report.diagnostics.push(d);
        return report;
    }

    report.output = Some(output.to_owned());
    report.geometry = Some(geometry);
    report.pages_from = Some(from);
    report
}

/// The words on the cover, as they will be set.
struct Words {
    title: Vec<String>,
    subtitle: String,
    spine: String,
    spine_foot: String,
    back: String,
}

impl Words {
    fn from(settings: &Settings) -> Words {
        let c = &settings.cover;
        let case = |s: &str| {
            if *c.uppercase {
                s.to_uppercase()
            } else {
                s.to_owned()
            }
        };
        let title: Vec<String> = c
            .title
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(case)
            .collect();
        let spine = if c.spine_title.trim().is_empty() {
            title.join(" ")
        } else {
            case(c.spine_title.trim())
        };
        Words {
            title,
            subtitle: c.subtitle.trim().to_owned(),
            spine,
            spine_foot: case(c.spine_foot.trim()),
            back: c.back_text.trim().to_owned(),
        }
    }

    fn all(&self) -> String {
        let mut s = self.title.join("");
        for part in [&self.subtitle, &self.spine, &self.spine_foot, &self.back] {
            s.push_str(part);
        }
        s
    }
}

// ---------------------------------------------------------------------------
// The font
// ---------------------------------------------------------------------------

/// A font file, read and ready to embed.
pub struct CoverFont {
    pub family: String,
    data: Vec<u8>,
    index: u32,
    cff: bool,
    units_per_em: f64,
    ascent: f64,
    descent: f64,
    cap_height: f64,
    bbox: [f64; 4],
    italic_angle: f64,
    /// Every glyph used, and the character it was used for.
    used: std::cell::RefCell<BTreeMap<u16, char>>,
}

fn load_font(family: &str, root: &Utf8Path, dirs: &[Utf8PathBuf]) -> Result<CoverFont, Diagnostic> {
    let unresolved = || {
        Diagnostic::error(
            code::UNRESOLVED,
            format!("the cover's font, {family}, is not installed and not in the project"),
        )
        .help("set `cover.font_family` to an installed font, or put its file in assets/fonts")
    };
    let resolved = crate::font::resolve(family, root, dirs).ok_or_else(unresolved)?;
    let data = std::fs::read(resolved.path.as_std_path()).map_err(|e| {
        unresolved()
            .at(SourceLoc::file(resolved.path.clone()))
            .detail(e.to_string())
    })?;
    if data.starts_with(b"ttcf") {
        return Err(Diagnostic::error(
            code::UNRESOLVED,
            format!(
                "{family} comes in a font collection ({}), which a cover cannot embed",
                resolved.path
            ),
        )
        .help("set `cover.font_family` to a family installed as a .ttf or .otf file"));
    }
    let face = ttf_parser::Face::parse(&data, resolved.index).map_err(|e| {
        Diagnostic::error(code::UNRESOLVED, format!("{family} could not be read"))
            .at(SourceLoc::file(resolved.path.clone()))
            .detail(e.to_string())
    })?;
    let upem = f64::from(face.units_per_em());
    let bb = face.global_bounding_box();
    let cff = face.tables().cff.is_some() || face.tables().cff2.is_some();
    let font = CoverFont {
        family: family.to_owned(),
        index: resolved.index,
        cff,
        units_per_em: upem,
        ascent: f64::from(face.ascender()),
        descent: f64::from(face.descender()),
        cap_height: face.capital_height().map(f64::from).unwrap_or(0.7 * upem),
        bbox: [
            f64::from(bb.x_min),
            f64::from(bb.y_min),
            f64::from(bb.x_max),
            f64::from(bb.y_max),
        ],
        italic_angle: f64::from(face.italic_angle()),
        used: Default::default(),
        data: Vec::new(),
    };
    Ok(CoverFont { data, ..font })
}

impl CoverFont {
    fn face(&self) -> ttf_parser::Face<'_> {
        ttf_parser::Face::parse(&self.data, self.index).expect("parsed once already")
    }

    /// The characters in `text` this font has no glyph for, other than space.
    pub fn missing(&self, text: &str) -> BTreeSet<char> {
        let face = self.face();
        text.chars()
            .filter(|c| !c.is_whitespace() && face.glyph_index(*c).is_none())
            .collect()
    }

    /// Cap height as a fraction of the size.
    fn cap(&self) -> f64 {
        self.cap_height / self.units_per_em
    }

    /// Glyphs and their advances (in ems) for `text`, recording what was used.
    fn glyphs(&self, text: &str) -> Vec<(u16, f64)> {
        let face = self.face();
        let mut used = self.used.borrow_mut();
        text.chars()
            .filter_map(|c| {
                let g = face.glyph_index(c)?;
                used.entry(g.0).or_insert(c);
                let adv = f64::from(face.glyph_hor_advance(g).unwrap_or(0)) / self.units_per_em;
                Some((g.0, adv))
            })
            .collect()
    }

    /// How wide `text` is at `size`, with `tracking` (in ems) after every
    /// glyph but the last.
    fn width(&self, text: &str, size: f64, tracking: f64) -> f64 {
        let g = self.glyphs(text);
        let n = g.len();
        let adv: f64 = g.iter().map(|(_, a)| a).sum();
        size * (adv + tracking * n.saturating_sub(1) as f64)
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// A number as a PDF content stream wants it: short, and the same every time.
fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

fn rgb(c: Color) -> String {
    format!(
        "{} {} {}",
        n(f64::from(c.red) / 255.0),
        n(f64::from(c.green) / 255.0),
        n(f64::from(c.blue) / 255.0)
    )
}

/// One thing drawn on the sheet, in points from its bottom-left corner.
///
/// The cover is drawn once, into these, and written out twice: as the PDF's
/// page, and as the SVG the window previews it with. Two drawings would be
/// two covers that could disagree.
#[derive(Debug, Clone)]
enum Mark {
    /// The whole sheet, filled.
    Ground { w: f64, h: f64, color: Color },
    /// A line through `points`, closed back to the first if `closed`.
    Path {
        points: Vec<(f64, f64)>,
        closed: bool,
        width: f64,
        dotted: bool,
        color: Color,
    },
    /// Glyphs and their advances in ems, the baseline's left end at
    /// (`x`, `y`) — or, `turned`, at the origin of a frame turned a quarter
    /// clockwise about (`x`, `y`), which is how a spine is lettered.
    Text {
        glyphs: Vec<(u16, f64)>,
        x: f64,
        y: f64,
        size: f64,
        tracking: f64,
        turned: bool,
        color: Color,
    },
}

/// What guides are drawn in: magenta, which no cover is printed in.
const GUIDE: Color = Color {
    red: 255,
    green: 0,
    blue: 255,
};

struct Pen<'a> {
    marks: Vec<Mark>,
    font: &'a CoverFont,
    color: Color,
    dotted: bool,
}

impl Pen<'_> {
    fn path(&mut self, points: Vec<(f64, f64)>, closed: bool, width: f64) {
        self.marks.push(Mark::Path {
            points,
            closed,
            width,
            dotted: self.dotted,
            color: self.color,
        });
    }

    fn rect(&mut self, r: Rect, width: f64) {
        self.path(
            vec![
                (r.x, r.y),
                (r.x + r.w, r.y),
                (r.x + r.w, r.y + r.h),
                (r.x, r.y + r.h),
            ],
            true,
            width,
        );
    }

    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, width: f64) {
        self.path(vec![(x1, y1), (x2, y2)], false, width);
    }

    fn polygon(&mut self, points: &[(f64, f64)], width: f64) {
        self.path(points.to_vec(), true, width);
    }

    fn glyphs(&mut self, text: &str, x: f64, y: f64, size: f64, tracking: f64, turned: bool) {
        let glyphs = self.font.glyphs(text);
        if glyphs.is_empty() {
            return;
        }
        self.marks.push(Mark::Text {
            glyphs,
            x,
            y,
            size,
            tracking,
            turned,
            color: self.color,
        });
    }

    /// `text` with its baseline's left end at (`x`, `y`).
    fn text(&mut self, text: &str, x: f64, y: f64, size: f64, tracking: f64) {
        self.glyphs(text, x, y, size, tracking, false);
    }

    /// `text` centred on `cx`, its baseline at `y`.
    fn centred(&mut self, text: &str, cx: f64, y: f64, size: f64, tracking: f64) {
        let w = self.font.width(text, size, tracking);
        self.text(text, cx - w / 2.0, y, size, tracking);
    }

    /// A rule, a diamond, a rule — centred on `cx`, its middle at `cy`.
    fn ornament(&mut self, cx: f64, cy: f64, arm: f64, width: f64) {
        let d = 3.5_f64.max(width * 3.0);
        let gap = d + 6.0;
        self.line(cx - gap - arm, cy, cx - gap, cy, width);
        self.line(cx + gap, cy, cx + gap + arm, cy, width);
        self.polygon(
            &[(cx, cy + d), (cx + d, cy), (cx, cy - d), (cx - d, cy)],
            width,
        );
    }
}

/// The largest size, up to `wanted`, at which `text` fits in `room`.
fn fit(font: &CoverFont, text: &str, wanted: f64, tracking: f64, room: f64) -> f64 {
    let at_one = font.width(text, 1.0, tracking);
    if at_one <= 0.0 {
        wanted
    } else {
        wanted.min(room / at_one)
    }
}

/// Greedy line breaking at spaces, by measured width.
fn wrap_lines(font: &CoverFont, text: &str, size: f64, room: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let tried = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && font.width(&tried, size, 0.0) > room {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = tried;
            }
        }
        lines.push(line);
    }
    lines
}

fn border(pen: &mut Pen<'_>, style: CoverBorder, r: Rect, bw: f64) {
    match style {
        CoverBorder::None => {}
        CoverBorder::Single => pen.rect(r, bw),
        CoverBorder::Double => {
            pen.rect(r, bw);
            pen.rect(r.inset(bw * 3.0 + 3.0), bw * 0.5);
        }
        CoverBorder::Triple => {
            pen.rect(r, bw * 0.5);
            pen.rect(r.inset(bw * 2.5 + 2.0), bw * 1.2);
            pen.rect(r.inset(2.0 * (bw * 2.5 + 2.0)), bw * 0.5);
        }
        CoverBorder::ThickThin => {
            pen.rect(r, bw * 2.5);
            pen.rect(r.inset(bw * 3.0 + 3.0), bw * 0.5);
        }
        CoverBorder::Panel => {
            let cut = (r.w.min(r.h) * 0.06).max(8.0);
            let panel = |r: Rect, cut: f64| {
                vec![
                    (r.x + cut, r.y),
                    (r.x + r.w - cut, r.y),
                    (r.x + r.w, r.y + cut),
                    (r.x + r.w, r.y + r.h - cut),
                    (r.x + r.w - cut, r.y + r.h),
                    (r.x + cut, r.y + r.h),
                    (r.x, r.y + r.h - cut),
                    (r.x, r.y + cut),
                ]
            };
            pen.polygon(&panel(r, cut), bw);
            let inner = bw * 3.0 + 3.0;
            pen.polygon(&panel(r.inset(inner), cut), bw * 0.5);
        }
        CoverBorder::Corners => {
            let arm = r.w.min(r.h) * 0.14;
            for (x, y, dx, dy) in [
                (r.x, r.y, 1.0, 1.0),
                (r.x + r.w, r.y, -1.0, 1.0),
                (r.x, r.y + r.h, 1.0, -1.0),
                (r.x + r.w, r.y + r.h, -1.0, -1.0),
            ] {
                pen.path(
                    vec![(x + dx * arm, y), (x, y), (x, y + dy * arm)],
                    false,
                    bw,
                );
                let i = bw * 3.0 + 3.0;
                let a = arm * 0.6;
                pen.path(
                    vec![
                        (x + dx * (i + a), y + dy * i),
                        (x + dx * i, y + dy * i),
                        (x + dx * i, y + dy * (i + a)),
                    ],
                    false,
                    bw * 0.5,
                );
            }
        }
        CoverBorder::Rules => {
            let gap = bw * 3.0 + 3.0;
            for (y, inward) in [(r.y, 1.0), (r.y + r.h, -1.0)] {
                pen.line(r.x, y, r.x + r.w, y, bw);
                pen.line(r.x, y + inward * gap, r.x + r.w, y + inward * gap, bw * 0.5);
            }
        }
        CoverBorder::Dotted => {
            let dot = bw * 1.6;
            pen.dotted = true;
            pen.rect(r, dot);
            pen.dotted = false;
        }
    }
}

fn draw(
    settings: &Settings,
    g: &Geometry,
    font: &CoverFont,
    words: &Words,
    lettered_spine: bool,
    guides: bool,
) -> Vec<Mark> {
    let c = &settings.cover;
    let mut pen = Pen {
        marks: Vec::new(),
        font,
        color: *c.foil,
        dotted: false,
    };

    // The ground, to the edge of the sheet.
    pen.marks.push(Mark::Ground {
        w: g.width,
        h: g.height,
        color: *c.background,
    });

    let bw = c.border_width.points();
    let inset = c.border_inset.points();
    let rule = (bw * 0.5).max(0.5);

    // ---- the front ----
    let front = g.front();
    border(&mut pen, *c.border, front.inset(inset), bw);
    let room = (front.w - 2.0 * inset - 0.5 * INCH).max(INCH);
    let big_wanted = front.w * 0.16;
    let cap = font.cap();
    let lines = &words.title;
    let last = lines.len().saturating_sub(1);
    let mut set: Vec<(String, f64, f64)> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let (wanted, tracking) = if i == last {
                (big_wanted, 0.12)
            } else {
                (big_wanted * 0.36, 0.4)
            };
            (l.clone(), fit(font, l, wanted, tracking, room), tracking)
        })
        .collect();
    let big = set.last().map(|s| s.1).unwrap_or(big_wanted);
    let line_gap = big * 0.2;
    let block_gap = big * 0.4;
    let ornament_h = 8.0;
    let subtitle_size = (big * 0.24).clamp(9.0, 24.0);

    let mut height: f64 = set.iter().map(|(_, s, _)| s * cap).sum::<f64>()
        + line_gap * set.len().saturating_sub(1) as f64;
    if *c.ornaments {
        height += 2.0 * (ornament_h + block_gap);
    }
    if !words.subtitle.is_empty() {
        height += block_gap + subtitle_size * cap;
    }

    let cx = front.centre_x();
    let arm = front.w * 0.11;
    let mut y = front.centre_y() + height / 2.0;
    if *c.ornaments {
        pen.ornament(cx, y - ornament_h / 2.0, arm, rule);
        y -= ornament_h + block_gap;
    }
    for (i, (text, size, tracking)) in set.drain(..).enumerate() {
        if i > 0 {
            y -= line_gap;
        }
        y -= size * cap;
        pen.centred(&text, cx, y, size, tracking);
    }
    if *c.ornaments {
        y -= block_gap;
        pen.ornament(cx, y - ornament_h / 2.0, arm, rule);
        y -= ornament_h;
    }
    if !words.subtitle.is_empty() {
        y -= block_gap + subtitle_size * cap;
        let size = fit(font, &words.subtitle, subtitle_size, 0.04, room);
        pen.centred(&words.subtitle, cx, y, size, 0.04);
    }

    // ---- the back ----
    let back = g.back();
    border(&mut pen, *c.border, back.inset(inset), bw);
    let bcx = back.centre_x();
    if words.back.is_empty() {
        if *c.ornaments {
            pen.ornament(bcx, back.centre_y(), back.w * 0.07, rule);
        }
    } else {
        let size = 11.0;
        let leading = 15.0;
        let room = (back.w - 2.0 * inset - 0.8 * INCH).max(INCH);
        let lines = wrap_lines(font, &words.back, size, room);
        let block = leading * lines.len() as f64;
        let mut y = back.centre_y() + block / 2.0 - size * cap;
        for l in &lines {
            pen.centred(l, bcx, y, size, 0.0);
            y -= leading;
        }
        if *c.ornaments {
            pen.ornament(bcx, y - leading * 0.6, back.w * 0.07, rule);
        }
    }

    // ---- the spine ----
    let spine = g.spine_rect();
    if lettered_spine {
        let scx = spine.centre_x();
        if *c.ornaments {
            let w = spine.w * 0.6;
            for y in [
                spine.y + spine.h - 0.75 * INCH,
                spine.y + spine.h - 0.75 * INCH - 4.0,
                spine.y + 0.75 * INCH,
                spine.y + 0.75 * INCH + 4.0,
            ] {
                pen.line(scx - w / 2.0, y, scx + w / 2.0, y, rule);
            }
        }
        let along = |pen: &mut Pen<'_>, text: &str, size: f64, tracking: f64, mid: f64| {
            // Read top to bottom, the letters' heads toward the front — the
            // way a book lying face up is read along its spine.
            let w = font.width(text, size, tracking);
            let x = scx - size * cap / 2.0;
            pen.glyphs(text, x, mid + w / 2.0, size, tracking, true);
        };
        let ends = if *c.ornaments { 1.0 * INCH } else { 0.6 * INCH };
        let mut length = spine.h - 2.0 * ends;
        if !words.spine_foot.is_empty() {
            let size = fit(
                font,
                &words.spine_foot,
                (spine.w * 0.3).min(12.0),
                0.15,
                spine.h * 0.25,
            );
            let w = font.width(&words.spine_foot, size, 0.15);
            along(
                &mut pen,
                &words.spine_foot,
                size,
                0.15,
                spine.y + ends + w / 2.0,
            );
            length -= w + 0.4 * INCH;
        }
        if !words.spine.is_empty() {
            let size = fit(
                font,
                &words.spine,
                (spine.w * 0.45).min(30.0),
                0.2,
                length.max(INCH),
            );
            let top = spine.y + spine.h - ends;
            along(
                &mut pen,
                &words.spine,
                size,
                0.2,
                top - length.max(INCH) / 2.0,
            );
        }
    }

    // ---- guides, for a proof ----
    if guides {
        pen.color = GUIDE;
        let hair = 0.3;
        let t = g.trim_box();
        pen.rect(t, hair);
        for r in [g.back(), g.front()] {
            pen.rect(r, hair);
        }
        let mut folds = vec![spine.x, spine.x + spine.w];
        if g.hinge > 0.0 {
            folds.extend([spine.x - g.hinge, spine.x + spine.w + g.hinge]);
        }
        for x in folds {
            pen.line(x, 0.0, x, g.height, hair);
        }
    }

    pen.marks
}

/// The marks as a PDF content stream.
fn to_pdf(marks: &[Mark]) -> Vec<u8> {
    let mut out = String::from("0 J 0 j\n");
    for m in marks {
        match m {
            Mark::Ground { w, h, color } => {
                let _ = writeln!(out, "{} rg 0 0 {} {} re f", rgb(*color), n(*w), n(*h));
            }
            Mark::Path {
                points,
                closed,
                width,
                dotted,
                color,
            } => {
                if *dotted {
                    let _ = write!(out, "1 J [0 {}] 0 d ", n(width * 2.4));
                }
                let _ = write!(out, "{} RG {} w ", rgb(*color), n(*width));
                for (i, (x, y)) in points.iter().enumerate() {
                    let _ = write!(
                        out,
                        "{} {} {} ",
                        n(*x),
                        n(*y),
                        if i == 0 { "m" } else { "l" }
                    );
                }
                out.push_str(if *closed { "s" } else { "S" });
                if *dotted {
                    out.push_str(" 0 J [] 0 d");
                }
                out.push('\n');
            }
            Mark::Text {
                glyphs,
                x,
                y,
                size,
                tracking,
                turned,
                color,
            } => {
                let mut hex = String::with_capacity(glyphs.len() * 4);
                for (g, _) in glyphs {
                    let _ = write!(hex, "{g:04X}");
                }
                let (at_x, at_y) = if *turned {
                    let _ = write!(out, "q 0 -1 1 0 {} {} cm ", n(*x), n(*y));
                    (0.0, 0.0)
                } else {
                    (*x, *y)
                };
                let _ = write!(
                    out,
                    "BT {} rg /F1 {} Tf {} Tc {} {} Td <{hex}> Tj ET",
                    rgb(*color),
                    n(*size),
                    n(tracking * size),
                    n(at_x),
                    n(at_y)
                );
                if *turned {
                    out.push_str(" Q");
                }
                out.push('\n');
            }
        }
    }
    out.into_bytes()
}

fn hex(c: Color) -> String {
    c.to_string()
}

/// A glyph's outline as SVG path data, in font units, y up.
struct Outline(String);

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "M{x} {y}");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "L{x} {y}");
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let _ = write!(self.0, "Q{x1} {y1} {x} {y}");
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let _ = write!(self.0, "C{x1} {y1} {x2} {y2} {x} {y}");
    }
    fn close(&mut self) {
        self.0.push('Z');
    }
}

/// The marks as an SVG document, the lettering as the font's own outlines —
/// so the preview is the PDF, glyph for glyph, without the window having to
/// have the font.
fn to_svg(marks: &[Mark], g: &Geometry, font: &CoverFont) -> String {
    let face = font.face();
    let mut used: BTreeSet<u16> = BTreeSet::new();
    for m in marks {
        if let Mark::Text { glyphs, .. } = m {
            used.extend(glyphs.iter().map(|(g, _)| *g));
        }
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">",
        w = n(g.width),
        h = n(g.height)
    );
    out.push_str("<defs>");
    for gid in &used {
        let mut o = Outline(String::new());
        face.outline_glyph(ttf_parser::GlyphId(*gid), &mut o);
        let _ = write!(out, "<path id=\"cover-g{gid}\" d=\"{}\"/>", o.0);
    }
    out.push_str("</defs>");
    // PDF's coordinates: y up from the bottom-left corner.
    let _ = write!(out, "<g transform=\"matrix(1 0 0 -1 0 {})\">", n(g.height));
    let em = font.units_per_em;
    for m in marks {
        match m {
            Mark::Ground { w, h, color } => {
                let _ = write!(
                    out,
                    "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    n(*w),
                    n(*h),
                    hex(*color)
                );
            }
            Mark::Path {
                points,
                closed,
                width,
                dotted,
                color,
            } => {
                let pts: Vec<String> = points
                    .iter()
                    .map(|(x, y)| format!("{},{}", n(*x), n(*y)))
                    .collect();
                let dots = if *dotted {
                    format!(
                        " stroke-linecap=\"round\" stroke-dasharray=\"0 {}\"",
                        n(width * 2.4)
                    )
                } else {
                    String::new()
                };
                let _ = write!(
                    out,
                    "<{tag} points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{dots}/>",
                    pts.join(" "),
                    hex(*color),
                    n(*width),
                    tag = if *closed { "polygon" } else { "polyline" },
                );
            }
            Mark::Text {
                glyphs,
                x,
                y,
                size,
                tracking,
                turned,
                color,
            } => {
                let frame = if *turned {
                    format!("matrix(0 -1 1 0 {} {})", n(*x), n(*y))
                } else {
                    format!("translate({} {})", n(*x), n(*y))
                };
                let _ = write!(out, "<g transform=\"{frame}\" fill=\"{}\">", hex(*color));
                let scale = size / em;
                let mut pen_x = 0.0;
                for (gid, adv) in glyphs {
                    let _ = write!(
                        out,
                        "<use href=\"#cover-g{gid}\" transform=\"translate({} 0) scale({})\"/>",
                        n(pen_x),
                        scale
                    );
                    pen_x += (adv + tracking) * size;
                }
                out.push_str("</g>");
            }
        }
    }
    out.push_str("</g></svg>");
    out
}

// ---------------------------------------------------------------------------
// The file
// ---------------------------------------------------------------------------

/// A PDF text string: literal for ASCII, UTF-16 with a byte-order mark for
/// anything else, as the specification asks.
fn text_string(s: &str) -> Object {
    if s.is_ascii() {
        Object::String(s.as_bytes().to_vec(), StringFormat::Literal)
    } else {
        let mut b = vec![0xFE, 0xFF];
        for u in s.encode_utf16() {
            b.extend_from_slice(&u.to_be_bytes());
        }
        Object::String(b, StringFormat::Hexadecimal)
    }
}

fn boxed(r: Rect) -> Object {
    Object::Array(vec![
        r.x.into(),
        r.y.into(),
        (r.x + r.w).into(),
        (r.y + r.h).into(),
    ])
}

/// The map from glyph back to character, so the lettering can be searched
/// and copied.
fn to_unicode(used: &BTreeMap<u16, char>) -> Vec<u8> {
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = used.iter().collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(s, "{} beginbfchar", chunk.len());
        for (g, c) in chunk {
            let mut units = [0u16; 2];
            let hex: String = c
                .encode_utf16(&mut units)
                .iter()
                .map(|u| format!("{u:04X}"))
                .collect();
            let _ = writeln!(s, "<{g:04X}> <{hex}>");
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s.into_bytes()
}

fn write_pdf(
    settings: &Settings,
    g: &Geometry,
    font: &CoverFont,
    content: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    // ---- the font, embedded whole ----
    let upem = font.units_per_em;
    let scale = |v: f64| (v * 1000.0 / upem).round() as i64;
    let file_dict = if font.cff {
        dictionary! { "Subtype" => Object::Name(b"OpenType".to_vec()) }
    } else {
        dictionary! { "Length1" => font.data.len() as i64 }
    };
    let file_id = doc.add_object(Stream::new(file_dict, font.data.clone()));
    let postscript: String = font
        .family
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let postscript = if postscript.is_empty() {
        "CoverFont".to_owned()
    } else {
        postscript
    };
    let descriptor_id = doc.add_object(dictionary! {
        "Type" => Object::Name(b"FontDescriptor".to_vec()),
        "FontName" => Object::Name(postscript.clone().into_bytes()),
        "Flags" => 4,
        "FontBBox" => Object::Array(font.bbox.iter().map(|v| Object::Integer(scale(*v))).collect()),
        "ItalicAngle" => font.italic_angle,
        "Ascent" => scale(font.ascent),
        "Descent" => scale(font.descent),
        "CapHeight" => scale(font.cap_height),
        "StemV" => 80,
        if font.cff { "FontFile3" } else { "FontFile2" } => Object::Reference(file_id),
    });

    let used = font.used.borrow().clone();
    let face = font.face();
    let mut widths = Vec::new();
    for gid in used.keys() {
        let adv = face
            .glyph_hor_advance(ttf_parser::GlyphId(*gid))
            .unwrap_or(0);
        widths.push(Object::Integer(i64::from(*gid)));
        widths.push(Object::Array(vec![Object::Integer(scale(f64::from(adv)))]));
    }
    let mut cid = dictionary! {
        "Type" => Object::Name(b"Font".to_vec()),
        "Subtype" => Object::Name(if font.cff { b"CIDFontType0".to_vec() } else { b"CIDFontType2".to_vec() }),
        "BaseFont" => Object::Name(postscript.clone().into_bytes()),
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => Object::Reference(descriptor_id),
        "W" => Object::Array(widths),
    };
    if !font.cff {
        cid.set("CIDToGIDMap", Object::Name(b"Identity".to_vec()));
    }
    let cid_id = doc.add_object(cid);
    let cmap_id = doc.add_object(Stream::new(Dictionary::new(), to_unicode(&used)));
    let font_id = doc.add_object(dictionary! {
        "Type" => Object::Name(b"Font".to_vec()),
        "Subtype" => Object::Name(b"Type0".to_vec()),
        "BaseFont" => Object::Name(postscript.into_bytes()),
        "Encoding" => Object::Name(b"Identity-H".to_vec()),
        "DescendantFonts" => Object::Array(vec![Object::Reference(cid_id)]),
        "ToUnicode" => Object::Reference(cmap_id),
    });

    // ---- the page ----
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content));
    let whole = Rect {
        x: 0.0,
        y: 0.0,
        w: g.width,
        h: g.height,
    };
    let page_id = doc.add_object(dictionary! {
        "Type" => Object::Name(b"Page".to_vec()),
        "Parent" => Object::Reference(pages_id),
        "MediaBox" => boxed(whole),
        "BleedBox" => boxed(whole),
        "TrimBox" => boxed(g.trim_box()),
        "Contents" => Object::Reference(content_id),
        "Resources" => dictionary! {
            "Font" => dictionary! { "F1" => Object::Reference(font_id) },
        },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => Object::Name(b"Pages".to_vec()),
            "Kids" => Object::Array(vec![Object::Reference(page_id)]),
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => Object::Name(b"Catalog".to_vec()),
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut info = dictionary! { "Producer" => Object::string_literal("BibleCompose") };
    let title = settings
        .project
        .name
        .as_deref()
        .map(|n| format!("{n} — cover"))
        .unwrap_or_else(|| "Cover".to_owned());
    info.set("Title", text_string(&title));
    let info_id = doc.add_object(info);
    doc.trailer.set("Info", Object::Reference(info_id));

    for obj in doc.objects.values_mut() {
        if let Object::Stream(s) = obj {
            let _ = s.compress();
        }
    }

    let mut out = Vec::new();
    doc.save_to(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short_and_stable() {
        assert_eq!(n(1.0), "1");
        assert_eq!(n(0.12345), "0.123");
        assert_eq!(n(-0.0001), "0");
        assert_eq!(n(432.5), "432.5");
    }
}
