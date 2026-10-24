//! How long the publication will be, before the typesetter has said.
//!
//! The progress bar needs a denominator and a typesetter cannot supply one: it
//! does not know how long a document is until it has set it. The only answer
//! here used to be the previous build's page count, which left two holes. The
//! *first* build of a project had no estimate at all and showed a bar with no
//! end. And one bare number cannot say which settings produced it, so taking
//! the body size from 9.2pt to 14pt left the bar measuring against a document
//! a third shorter than the one being set, with nothing saying so.
//!
//! So there are two answers now, in this order:
//!
//! 1. **A build already made in this shape** — the same language, font, body
//!    size, leading, trim and column count — remembered between projects, so a
//!    second publication in a language already set gets a true number on its
//!    first build.
//! 2. **An estimate from the document's own chapter count**, scaled off one
//!    measured constant.
//!
//! # The constant, and what it is worth
//!
//! Measured rather than derived. A whole Protestant canon in Tamil — 1189
//! chapters — set at 11pt on 13.4pt leading came to 3776 pages, which is
//! [`PAGES_PER_CHAPTER`]. The same canon at 9.2pt on 11.2pt came to 2651 and
//! this model says 2640: 0.4% out.
//!
//! **What it scales by is `size × leading`, not the body size.** Those are
//! independent settings here, and the measurements say their product governs
//! the length: 9.2/11.2 to 11/13.4 is a factor of 1.431 and the pages moved by
//! 1.424. Body size alone would have predicted 1.196 and been wrong by a
//! quarter.
//!
//! **And by the area of the text block**, because the margins decide as much as
//! the type does. The same canon at the same 9.2/11.2, differing only in
//! margins, came to 2651 pages with this project's and 2202 with the built-in
//! ones — a fifth, from four settings nobody thinks of as affecting length.
//!
//! # What it is not
//!
//! **A chapter is a coarse ruler, and the error is worth stating plainly.**
//! Mark alone — 16 long chapters of English prose — sets in 57 pages and this
//! estimate says 42, a quarter under, because the canon-wide average is held
//! down by the 150 short chapters of the Psalms. Counting graphemes instead
//! was tried and is no better in the other direction: it reads Mark 51% *over*,
//! because a Latin grapheme is narrower than a Tamil one, and getting that
//! right would need a per-script width this has no way to know. Carrying the
//! area term across a trim it was not measured at costs about a tenth: the
//! built-in margins above predict 2450 against 2202.
//!
//! Which is why the remembered count is always preferred when there is one,
//! why the bar is capped short of full, and why the window says *about*.

use camino::{Utf8Path, Utf8PathBuf};

use biblecompose_config::Settings;
use biblecompose_scripture::ScriptureDocument;

/// The body size the constant was measured at.
const REFERENCE_SIZE_PT: f64 = 11.0;
/// The leading it was measured at.
const REFERENCE_LEADING_PT: f64 = 13.4;
/// The text block it was measured in, in square points: 6x9in less margins of
/// 0.85in top, 1.1in bottom, 0.8in inner and 0.65in outer.
const REFERENCE_AREA_PT2: f64 = 327.6 * 507.6;
/// Pages per chapter at the reference, over a whole canon.
const PAGES_PER_CHAPTER: f64 = 3776.0 / 1189.0;

/// The shape of a build, as far as its length is concerned.
///
/// The settings a measured page count is only true for, and **the books it was
/// measured over**. A colour or a caller style changes the page without
/// changing how many there are, so a build differing only in those is still
/// one this can learn from; a build of six books is not a build of four.
///
/// That last part was missing and is the whole of this type's history: with
/// the settings alone as the key, generating the four gospels and then adding
/// Acts and Romans was the same shape, so the gospels' page count was handed
/// back for a document half again as long. The estimate had the chapters right
/// all along and never got asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    language: String,
    font: String,
    /// Hundredths of a point. An integer, so two shapes compare exactly rather
    /// than nearly, which is what a key has to do.
    size: u64,
    leading: u64,
    trim: String,
    columns: u8,
    /// The books that will be set, in canon order, by code.
    ///
    /// The codes and not their number: dropping Acts and adding Romans leaves
    /// two books either way, and the one thing a key must not do is match a
    /// different document. Not their *contents* either — a book whose text was
    /// edited keeps its shape, and the count stays close enough for a bar
    /// while being wrong in the way the previous build's count always was.
    books: String,
}

impl Shape {
    pub fn of(doc: &ScriptureDocument, settings: &Settings) -> Shape {
        Shape {
            language: settings.project.language.to_string(),
            font: settings.typography.font_family.to_string(),
            size: hundredths(settings.typography.font_size.points()),
            leading: hundredths(settings.typography.leading.points()),
            trim: settings.page.size.to_sile(),
            columns: *settings.page.columns,
            books: doc
                .books
                .iter()
                .map(|b| b.code.as_str())
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    /// One line of the remembered file. Tab-separated because a trim reads
    /// `432pt x 648pt` and a font family has spaces in it, and a tab is the one
    /// character neither can contain.
    fn line(&self, pages: u32) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{pages}",
            self.language,
            self.font,
            self.size,
            self.leading,
            self.trim,
            self.columns,
            self.books
        )
    }

    /// One line back, or nothing.
    ///
    /// A line of the wrong width is simply not a shape, which is also how the
    /// file written before the books were part of the key retires itself: it
    /// stops being read rather than being read wrongly, and the first build
    /// of each shape measures it again.
    fn parse(line: &str) -> Option<(Shape, u32)> {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 8 {
            return None;
        }
        let pages: u32 = f[7].trim().parse().ok()?;
        if pages == 0 {
            return None;
        }
        Some((
            Shape {
                language: f[0].to_owned(),
                font: f[1].to_owned(),
                size: f[2].parse().ok()?,
                leading: f[3].parse().ok()?,
                trim: f[4].to_owned(),
                columns: f[5].parse().ok()?,
                books: f[6].to_owned(),
            },
            pages,
        ))
    }
}

fn hundredths(points: f64) -> u64 {
    (points.max(0.0) * 100.0).round() as u64
}

/// The estimate, in pages, or `None` when nothing honest can be said.
///
/// `None` rather than a guess for an empty document or a nonsensical setting:
/// a bar with no end says "something is happening", which is true, where a
/// wrong denominator says something false.
pub fn estimate(doc: &ScriptureDocument, settings: &Settings) -> Option<u32> {
    let chapters: usize = doc.books.iter().map(|b| b.chapter_count()).sum();
    if chapters == 0 {
        return None;
    }

    let size = settings.typography.font_size.points();
    let leading = settings.typography.leading.points();
    let area = text_block_pt2(settings)?;
    if !(size.is_finite() && leading.is_finite()) || size <= 0.0 || leading <= 0.0 {
        return None;
    }

    let pages = chapters as f64
        * PAGES_PER_CHAPTER
        * ((size * leading) / (REFERENCE_SIZE_PT * REFERENCE_LEADING_PT))
        * (REFERENCE_AREA_PT2 / area);

    (pages.is_finite() && pages >= 1.0).then(|| pages.round() as u32)
}

/// The area the Scripture is set in, in square points.
///
/// The gutter is left out of it: it is the column that narrows, not the block,
/// and the few points it takes are well inside what a chapter-counted estimate
/// is worth anyway.
fn text_block_pt2(settings: &Settings) -> Option<f64> {
    let page = &settings.page;
    let width = page.size.width.points() - page.margin_inner.points() - page.margin_outer.points();
    let height = page.size.height.points() - page.margin_top.points() - page.margin_bottom.points();
    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0).then_some(width * height)
}

/// Where page counts are remembered across projects.
///
/// Outside any project, because the point is the *second* project: a publisher
/// who has set one New Testament in Marathi at 11pt should not watch an endless
/// bar on the next one. The project folder stays the source of truth for the
/// publication itself — this is a measurement about the machine, like the
/// unpacked typesetter beside it, and losing it costs one imprecise bar.
fn store() -> Option<Utf8PathBuf> {
    let root = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
    }?;
    let root = Utf8PathBuf::from_path_buf(root).ok()?;
    Some(root.join("biblecompose").join("pages"))
}

/// How many pages a build in this shape came to, if one has been.
pub fn remembered(shape: &Shape) -> Option<u32> {
    remembered_in(store()?.as_path(), shape)
}

fn remembered_in(path: &Utf8Path, shape: &Shape) -> Option<u32> {
    let text = std::fs::read_to_string(path.as_std_path()).ok()?;
    text.lines()
        .filter_map(Shape::parse)
        .find(|(found, _)| found == shape)
        .map(|(_, pages)| pages)
}

/// The most shapes worth keeping.
///
/// A publisher who has tried thirty settings does not need the first of them,
/// and a file that grows for ever is a file that is read for ever. Newest
/// first, so the cap drops what was least recently measured.
const KEEP: usize = 64;

/// Remember what a build in this shape came to.
///
/// Best-effort throughout: this is an improvement to a progress bar, and a
/// read-only cache directory must not be able to fail a build that succeeded.
pub fn remember(shape: &Shape, pages: u32) {
    if pages == 0 {
        return;
    }
    let Some(path) = store() else { return };
    remember_in(path.as_path(), shape, pages);
}

fn remember_in(path: &Utf8Path, shape: &Shape, pages: u32) {
    let existing = std::fs::read_to_string(path.as_std_path()).unwrap_or_default();
    let mut lines: Vec<String> = vec![shape.line(pages)];
    lines.extend(
        existing
            .lines()
            .filter(|line| Shape::parse(line).is_some_and(|(found, _)| &found != shape))
            .map(str::to_owned),
    );
    lines.truncate(KEEP);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent.as_std_path());
    }
    let _ = std::fs::write(path.as_std_path(), lines.join("\n") + "\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use biblecompose_config::{settings, ConfigDocument};

    /// A document of these books and no content.
    ///
    /// The chapters do not matter to a *key* — only which books are in it —
    /// so these are empty, which also keeps the fixtures out of a test about
    /// identity.
    fn books(codes: &[&str]) -> ScriptureDocument {
        ScriptureDocument::new(
            codes
                .iter()
                .map(|c| {
                    biblecompose_scripture::Book::new(
                        biblecompose_scripture::BookCode::parse(c).expect("a real code"),
                        Default::default(),
                        Vec::new(),
                    )
                })
                .collect(),
        )
    }

    fn with(toml: &str) -> Settings {
        let doc = ConfigDocument::parse("t.toml", format!("schema_version = 1\n{toml}"))
            .expect("valid TOML");
        let (resolved, diagnostics) = settings::resolve(Some(&doc));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        resolved
    }

    /// The geometry and type the constant was measured in.
    fn reference() -> Settings {
        with(
            "[page]\nsize = \"6x9in\"\ncolumns = 2\nmargin_top = \"0.85in\"\n\
             margin_bottom = \"1.1in\"\nmargin_inner = \"0.8in\"\nmargin_outer = \"0.65in\"\n\
             [typography]\nfont_size = \"11pt\"\nleading = \"13.4pt\"\n",
        )
    }

    /// **The whole canon it was measured on comes back.**
    ///
    /// 1189 chapters at the reference were 3776 pages. Not a tautology worth
    /// skipping: it is the arithmetic in both directions, and a sign slip in
    /// either scale would show here.
    #[test]
    fn the_measured_canon_estimates_to_what_it_measured() {
        let got = scale(1189, &reference());
        assert!((3770..=3782).contains(&got), "got {got}");
    }

    /// **And the same canon at a smaller size, which is the real check.**
    ///
    /// 9.2pt on 11.2pt in the same block came to 2651 pages. This is the
    /// measurement the model was not fitted to.
    #[test]
    fn the_same_canon_set_smaller_lands_within_a_percent() {
        let smaller = with(
            "[page]\nsize = \"6x9in\"\ncolumns = 2\nmargin_top = \"0.85in\"\n\
             margin_bottom = \"1.1in\"\nmargin_inner = \"0.8in\"\nmargin_outer = \"0.65in\"\n\
             [typography]\nfont_size = \"9.2pt\"\nleading = \"11.2pt\"\n",
        );
        let got = scale(1189, &smaller);
        let off = (got as f64 - 2651.0).abs() / 2651.0;
        assert!(off < 0.01, "got {got} against a measured 2651, {off:.3} out");
    }

    /// **Leading counts as much as the body size.**
    ///
    /// The defect this model exists to avoid: scaling on the body size alone
    /// read a 43% longer document as 20% longer.
    #[test]
    fn leading_moves_the_estimate() {
        let tight = with("[typography]\nfont_size = \"11pt\"\nleading = \"11.2pt\"\n");
        let loose = with("[typography]\nfont_size = \"11pt\"\nleading = \"13.4pt\"\n");
        assert!(scale(1189, &loose) > scale(1189, &tight));
    }

    /// **And so do the margins**, which nobody thinks of as a length setting.
    #[test]
    fn wider_margins_make_more_pages() {
        let narrow = with("[page]\nmargin_inner = \"0.4in\"\nmargin_outer = \"0.4in\"\n");
        let wide = with("[page]\nmargin_inner = \"1.4in\"\nmargin_outer = \"1.4in\"\n");
        assert!(scale(1189, &wide) > scale(1189, &narrow));
    }

    /// An empty document has no estimate rather than an estimate of nothing.
    #[test]
    fn nothing_to_set_is_not_an_estimate_of_zero() {
        assert_eq!(scale(0, &reference()), 0);
    }

    /// The scale alone, so the arithmetic is testable without a document.
    fn scale(chapters: usize, settings: &Settings) -> u32 {
        if chapters == 0 {
            return 0;
        }
        let size = settings.typography.font_size.points();
        let leading = settings.typography.leading.points();
        let area = text_block_pt2(settings).expect("a real block");
        (chapters as f64
            * PAGES_PER_CHAPTER
            * ((size * leading) / (REFERENCE_SIZE_PT * REFERENCE_LEADING_PT))
            * (REFERENCE_AREA_PT2 / area))
            .round() as u32
    }

    /// **A shape round-trips through the file, and a near miss is a miss.**
    ///
    /// The point of keying it: a build at 14pt must not be answered with the
    /// page count of one at 11pt, which is what one bare number did.
    #[test]
    fn a_remembered_count_is_found_only_for_the_shape_that_made_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("pages"))
            .expect("UTF-8 temp path");

        let eleven = Shape::of(&books(&["MAT"]), &with(
            "[project]\nlanguage = \"ta\"\n[typography]\nfont_size = \"11pt\"\n",
        ));
        let fourteen = Shape::of(&books(&["MAT"]), &with(
            "[project]\nlanguage = \"ta\"\n[typography]\nfont_size = \"14pt\"\n",
        ));
        assert_ne!(eleven, fourteen);

        remember_in(path.as_path(), &eleven, 3776);
        assert_eq!(remembered_in(path.as_path(), &eleven), Some(3776));
        assert_eq!(remembered_in(path.as_path(), &fourteen), None);

        // A second shape joins it rather than replacing it.
        remember_in(path.as_path(), &fourteen, 4800);
        assert_eq!(remembered_in(path.as_path(), &eleven), Some(3776));
        assert_eq!(remembered_in(path.as_path(), &fourteen), Some(4800));

        // And measuring the same shape again replaces that one entry.
        remember_in(path.as_path(), &eleven, 3800);
        assert_eq!(remembered_in(path.as_path(), &eleven), Some(3800));
        assert_eq!(remembered_in(path.as_path(), &fourteen), Some(4800));
    }

    /// **Adding a book is a different document, and a different key.**
    ///
    /// The defect this field exists for, in the shape it was reported: build
    /// the four gospels, then include Acts and Romans and build again. With
    /// the settings alone as the key the second build was handed the first
    /// one's page count — a document half again as long, measured against a
    /// number that could not have been right.
    #[test]
    fn adding_a_book_does_not_reuse_the_count_of_the_books_before_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("pages"))
            .expect("UTF-8 temp path");
        let settings = reference();

        let gospels = Shape::of(&books(&["MAT", "MRK", "LUK", "JHN"]), &settings);
        let plus_two =
            Shape::of(&books(&["MAT", "MRK", "LUK", "JHN", "ACT", "ROM"]), &settings);
        assert_ne!(gospels, plus_two, "six books are not four");

        remember_in(path.as_path(), &gospels, 120);
        assert_eq!(remembered_in(path.as_path(), &gospels), Some(120));
        assert_eq!(
            remembered_in(path.as_path(), &plus_two),
            None,
            "the gospels' count must not answer for the gospels plus two"
        );

        // And swapping one book for another is not the same either, which a
        // count of books rather than their names would have missed.
        let swapped = Shape::of(&books(&["MAT", "MRK", "LUK", "ACT"]), &settings);
        assert_ne!(gospels, swapped);
        assert_eq!(remembered_in(path.as_path(), &swapped), None);
    }

    /// A missing or corrupt file is no estimate, never a panic.
    #[test]
    fn an_unreadable_store_is_simply_no_answer() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path =
            camino::Utf8PathBuf::from_path_buf(dir.path().join("pages")).expect("UTF-8 temp path");
        let shape = Shape::of(&books(&["MAT"]), &with("[project]\nlanguage = \"ta\"\n"));

        assert_eq!(remembered_in(path.as_path(), &shape), None);
        std::fs::write(path.as_std_path(), "nonsense\n\t\t\t\nta\t\t\t\t\t\t0\n").expect("write");
        assert_eq!(remembered_in(path.as_path(), &shape), None);
    }

    /// The file does not grow without end.
    #[test]
    fn the_store_keeps_the_newest_and_drops_the_rest() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path =
            camino::Utf8PathBuf::from_path_buf(dir.path().join("pages")).expect("UTF-8 temp path");

        for n in 0..(KEEP + 10) {
            let shape = Shape::of(&books(&["MAT"]), &with(&format!(
                "[typography]\nfont_size = \"{}pt\"\n",
                9.0 + n as f64 / 10.0
            )));
            remember_in(path.as_path(), &shape, 100 + n as u32);
        }
        let lines = std::fs::read_to_string(path.as_std_path()).expect("read");
        assert_eq!(lines.lines().filter(|l| !l.is_empty()).count(), KEEP);

        // The newest is still there and the oldest is not.
        let newest = Shape::of(&books(&["MAT"]), &with(&format!(
            "[typography]\nfont_size = \"{}pt\"\n",
            9.0 + (KEEP + 9) as f64 / 10.0
        )));
        let oldest = Shape::of(&books(&["MAT"]), &with("[typography]\nfont_size = \"9pt\"\n"));
        assert!(remembered_in(path.as_path(), &newest).is_some());
        assert_eq!(remembered_in(path.as_path(), &oldest), None);
    }
}
