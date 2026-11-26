//! Fonts, checked before a page is set (ARCHITECTURE §7.1).
//!
//! The spike watched the alternative succeed. A Latin font asked to set Tamil
//! produces a valid PDF, correct trim, fonts embedded, a text layer that
//! extracts perfectly — and a page of empty boxes, with exit code zero
//! ([spike F-12]). Measured on a real corpus book: **95.5% of the glyphs
//! drawn were `.notdef`** and nothing in the build said a word.
//!
//! SRS-REVIEW F4 specifies DET-002's PDF assertions as structural — page
//! count, geometry, embedded font list, extracted text. Every one of those
//! passes on the tofu page. So this is not defence in depth; it is the only
//! defence, and it has to run before the backend rather than after.
//!
//! # What this can and cannot promise
//!
//! It reads the face that *this* resolution finds for a family name and checks
//! the codepoints the document actually uses against its character map. What
//! it cannot promise is that SILE resolves the same name to the same file:
//! SILE asks fontconfig, this asks `fontdb`, and on a machine with two faces
//! of one name they can disagree. Both look in the same places — the project's
//! own font directory first, then the system — so a disagreement means two
//! installed faces with one name, which is worth knowing about anyway.
//!
//! A project font is therefore passed to the backend **by path** rather than
//! by name (FONT-003), which removes the ambiguity for the case that matters:
//! the font a publisher shipped with their book.
//!
//! [spike F-12]: ../../../spike/NOTES.md

use std::collections::BTreeMap;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, ScriptureRef, SourceLoc};
use biblecompose_scripture::{Inline, ScriptureDocument};
use camino::{Utf8Path, Utf8PathBuf};

/// Where a project keeps fonts it ships with the book (FONT-003).
pub const PROJECT_FONTS: &str = "assets/fonts";

/// A font family, resolved to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFont {
    pub family: String,
    pub path: Utf8PathBuf,
    /// Which face within the file, for a collection such as `Nirmala.ttc`.
    pub index: u32,
    /// Whether it came from the project rather than the system, which decides
    /// whether the backend is told a path or a name.
    pub from_project: bool,
}

/// The families among `families` that have no italic face, as the class
/// reads them: joined by `|`.
///
/// The class slants the regular face for an italic these cannot give, as a
/// word processor does — Mukta Malar, like many fonts for Indic scripts, is
/// published with no italic at all. A family nobody has is left out: FONT-001
/// has said so, and there is no face to slant.
pub fn without_italic<'a>(
    families: impl IntoIterator<Item = &'a str>,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
) -> String {
    let mut out: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for family in families {
        if resolve(family, project_root, backend_dirs).is_some()
            && !faces_of(family, project_root, backend_dirs).italic
        {
            out.insert(family);
        }
    }
    out.into_iter().collect::<Vec<_>>().join("|")
}

/// The optical-size axis of each font that has one, as the class reads it:
/// `Family=min:max`, joined by `|`.
///
/// Only these fonts are given an optical size. Asking a font without the
/// axis for one is not ignored but refused — SILE's instancer fails, and the
/// build with it — so the class has to know which fonts can take one, and
/// the range each can, before it asks.
pub fn optical_axes<'a>(fonts: impl IntoIterator<Item = &'a ResolvedFont>) -> String {
    let mut out: std::collections::BTreeMap<String, (f32, f32)> = std::collections::BTreeMap::new();
    for font in fonts {
        if out.contains_key(&font.family) {
            continue;
        }
        let Ok(data) = std::fs::read(font.path.as_std_path()) else {
            continue;
        };
        let Ok(face) = ttf_parser::Face::parse(&data, font.index) else {
            continue;
        };
        if let Some(axis) = face
            .variation_axes()
            .into_iter()
            .find(|a| a.tag == ttf_parser::Tag::from_bytes(b"opsz"))
        {
            out.insert(font.family.clone(), (axis.min_value, axis.max_value));
        }
    }
    out.into_iter()
        .map(|(family, (min, max))| format!("{family}={min}:{max}"))
        .collect::<Vec<_>>()
        .join("|")
}

/// One codepoint the font cannot draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub character: char,
    /// How many times it appears. A single stray character and a whole script
    /// are different problems and the count is what distinguishes them.
    pub count: usize,
    /// Where it first appears, so the message points at Scripture rather than
    /// at a codepoint (FONT-002).
    pub reference: Option<ScriptureRef>,
}

/// Resolve a family name to a file: the project's fonts, then the backend's,
/// then the system's.
///
/// Project first because a book that ships its own font must render the same
/// way on a machine where that font is not installed — and because a publisher
/// who put a file in their project meant it. The backend's own directory next,
/// because that is the order its fontconfig file lists, and the built-in
/// default lives there rather than in the operating system.
pub fn resolve(
    family: &str,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
) -> Option<ResolvedFont> {
    let wanted = family.trim();
    if wanted.is_empty() {
        return None;
    }

    let project_dir = project_root.join(PROJECT_FONTS);
    if let Some(found) = lookup(wanted, Some(&project_dir), false) {
        return Some(ResolvedFont {
            from_project: true,
            ..found
        });
    }
    for dir in backend_dirs {
        if let Some(found) = lookup(wanted, Some(dir), false) {
            return Some(found);
        }
    }
    lookup(wanted, None, true)
}

fn lookup(family: &str, dir: Option<&Utf8Path>, system: bool) -> Option<ResolvedFont> {
    lookup_face(family, fontdb::Style::Normal, 400, dir, system)
}

/// The face of `family` nearest to a style and weight, by the same search
/// order as [`resolve`].
///
/// **Nearest, not exact, and that is the point.** fontdb substitutes the way
/// fontconfig does: a family with no italic answers an italic request with its
/// regular face, which is also what SILE then sets. So the face this returns is
/// the face the page gets, and a family whose italic is a different file from
/// its regular returns that file.
fn resolve_face(
    family: &str,
    italic: bool,
    weight: u16,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
) -> Option<ResolvedFont> {
    let wanted = family.trim();
    if wanted.is_empty() {
        return None;
    }
    let style = if italic {
        fontdb::Style::Italic
    } else {
        fontdb::Style::Normal
    };
    let project_dir = project_root.join(PROJECT_FONTS);
    if let Some(found) = lookup_face(wanted, style, weight, Some(&project_dir), false) {
        return Some(ResolvedFont {
            from_project: true,
            ..found
        });
    }
    for dir in backend_dirs {
        if let Some(found) = lookup_face(wanted, style, weight, Some(dir), false) {
            return Some(found);
        }
    }
    lookup_face(wanted, style, weight, None, true)
}

fn lookup_face(
    family: &str,
    style: fontdb::Style,
    weight: u16,
    dir: Option<&Utf8Path>,
    system: bool,
) -> Option<ResolvedFont> {
    let mut db = fontdb::Database::new();
    match dir {
        Some(dir) if dir.exists() => db.load_fonts_dir(dir.as_std_path()),
        Some(_) => return None,
        None => {}
    }
    if system {
        db.load_system_fonts();
    }

    // [`resolve`] asks for the regular face only. Weight and style are the
    // backend's to pick from the family, and asking there for a face this
    // document may never use would reject a family over a bold it does not
    // need. [`resolve_face`] asks for one face because a style is known to use
    // it.
    let id = db.query(&fontdb::Query {
        families: &[fontdb::Family::Name(family)],
        style,
        weight: fontdb::Weight(weight),
        ..Default::default()
    })?;
    let face = db.face(id)?;

    match &face.source {
        fontdb::Source::File(path) => Some(ResolvedFont {
            family: family.to_owned(),
            path: Utf8PathBuf::from_path_buf(path.clone()).ok()?,
            index: face.index,
            from_project: false,
        }),
        // A face `fontdb` holds in memory has no path to give the backend, and
        // nothing this application loads produces one.
        _ => None,
    }
}

/// Where a font came from, which is the first thing a picker has to say about
/// it: a family the project ships travels with the book, and one that is
/// merely installed here does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Project,
    Backend,
    System,
}

impl Source {
    pub const fn as_str(self) -> &'static str {
        match self {
            Source::Project => "project",
            Source::Backend => "backend",
            Source::System => "system",
        }
    }
}

/// One font a person can pick, and whether it can set the book they have open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub family: String,
    pub source: Source,
    /// How many of the Scripture's distinct characters this family cannot
    /// draw. `Some(0)` means it can set the book; `None` means there was no
    /// Scripture to check it against, or the file could not be read.
    pub missing: Option<usize>,
}

/// The distinct characters a document sets.
///
/// Separated from [`codepoints`] because a picker wants to check three hundred
/// families against one book, and re-walking the Scripture for each of them
/// would turn opening a dialog into a build.
pub fn characters(doc: &ScriptureDocument) -> std::collections::BTreeSet<char> {
    codepoints(doc).into_keys().collect()
}

/// Every font family a build could resolve, in the order resolution would find
/// them, each checked against the Scripture (FONT-002).
///
/// The check is the reason this exists rather than an operating-system font
/// dialog. A publisher setting Tamil is choosing from a list of which perhaps
/// four entries can draw the book, and the platform picker will happily hand
/// back one of the other three hundred — which is the failure the pre-flight
/// then has to explain. Better to answer the question in the list.
pub fn choices(
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
    characters: &std::collections::BTreeSet<char>,
) -> Vec<Choice> {
    let mut db = fontdb::Database::new();

    // Loaded in resolution order and recorded as first seen, so a family that
    // exists in two places is attributed to the one a build would use.
    let mut faces: BTreeMap<String, (Source, fontdb::ID)> = BTreeMap::new();
    let mut sweep = |db: &fontdb::Database, source: Source| {
        for face in db.faces() {
            // The first name only. `families` also carries the localized ones,
            // and a list showing a face twice under two spellings of the same
            // family is a list that looks broken. Either name still resolves.
            if let Some((name, _)) = face.families.first() {
                faces
                    .entry(name.clone())
                    .or_insert_with(|| (source, face.id));
            }
        }
    };

    let project_dir = project_root.join(PROJECT_FONTS);
    if project_dir.exists() {
        db.load_fonts_dir(project_dir.as_std_path());
        sweep(&db, Source::Project);
    }
    for dir in backend_dirs {
        if dir.exists() {
            db.load_fonts_dir(dir.as_std_path());
        }
    }
    sweep(&db, Source::Backend);
    db.load_system_fonts();
    sweep(&db, Source::System);

    let mut out: Vec<Choice> = faces
        .into_iter()
        .map(|(family, (source, id))| Choice {
            family,
            source,
            missing: if characters.is_empty() {
                None
            } else {
                missing_from(&db, id, characters)
            },
        })
        .collect();

    // Alphabetical, and not by coverage: a list that reorders itself when a
    // different book is open is a list nobody can learn. The count is beside
    // each name and the frontend can filter on it.
    out.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| a.family.to_lowercase().cmp(&b.family.to_lowercase()))
    });
    out
}

/// How many of `characters` one face cannot draw.
///
/// Through `with_face_data` rather than reading the path, because a family can
/// come from a collection or from memory and this is the only accessor that
/// covers both.
fn missing_from(
    db: &fontdb::Database,
    id: fontdb::ID,
    characters: &std::collections::BTreeSet<char>,
) -> Option<usize> {
    db.with_face_data(id, |data, index| {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        Some(
            characters
                .iter()
                .filter(|c| face.glyph_index(**c).is_none())
                .count(),
        )
    })
    .flatten()
}

/// Every distinct character the document sets, and where each first appears.
///
/// Distinct rather than every occurrence: a Bible is millions of characters
/// and a few hundred codepoints, and the check is about the codepoints. The
/// reference comes from the chapter and verse anchors the model keeps inline
/// (SCR-001), which is what makes a coverage failure nameable as Scripture
/// rather than as a hex value.
pub fn codepoints(doc: &ScriptureDocument) -> BTreeMap<char, (usize, Option<ScriptureRef>)> {
    let mut seen: BTreeMap<char, (usize, Option<ScriptureRef>)> = BTreeMap::new();

    for book in &doc.books {
        let name = book
            .names
            .for_running_head()
            .unwrap_or_else(|| book.code.as_str())
            .to_owned();
        let mut chapter: u16 = 0;
        let mut verse: Option<u16> = None;

        for block in &book.blocks {
            block.each_inline(&mut |inline| match inline {
                Inline::Chapter { number, .. } => {
                    chapter = *number;
                    verse = None;
                }
                Inline::Verse { id, .. } => verse = Some(id.start),
                Inline::Text(text) => {
                    for c in text.chars() {
                        // Whitespace is not set by a glyph, and reporting a
                        // font for lacking a space would be noise in front of
                        // the finding that matters.
                        if c.is_whitespace() || c.is_control() {
                            continue;
                        }
                        let entry = seen.entry(c).or_insert((0, None));
                        entry.0 += 1;

                        // The first occurrence *that has a reference*, rather
                        // than simply the first. A book opens with a title and
                        // often an introduction, so the commonest letter of
                        // the script first appears where there is no chapter
                        // to name — and "John 0" names a chapter that does not
                        // exist. Keep looking until the text proper.
                        if entry.1.is_none() && chapter > 0 {
                            entry.1 = Some(ScriptureRef {
                                book: name.clone(),
                                chapter,
                                verse,
                            });
                        }
                    }
                }
                _ => {}
            });
        }
    }

    seen
}

/// Which of those characters the font cannot draw.
pub fn gaps(font: &ResolvedFont, doc: &ScriptureDocument) -> Result<Vec<Gap>, Diagnostic> {
    let data = std::fs::read(font.path.as_std_path()).map_err(|e| {
        Diagnostic::error(
            code::UNRESOLVED,
            format!("could not read the font file for {}", font.family),
        )
        .at(SourceLoc::file(font.path.clone()))
        .detail(e.to_string())
    })?;

    let face = ttf_parser::Face::parse(&data, font.index).map_err(|e| {
        Diagnostic::error(
            code::UNRESOLVED,
            format!("{} is not a font this release can read", font.family),
        )
        .at(SourceLoc::file(font.path.clone()))
        .detail(e.to_string())
    })?;

    let mut gaps: Vec<Gap> = codepoints(doc)
        .into_iter()
        .filter(|(c, _)| face.glyph_index(*c).is_none())
        .map(|(character, (count, reference))| Gap {
            character,
            count,
            reference,
        })
        .collect();

    // Commonest first: the one that appears ten thousand times is the script
    // the font does not have, and the one that appears once is a stray.
    gaps.sort_by(|a, b| b.count.cmp(&a.count).then(a.character.cmp(&b.character)));
    Ok(gaps)
}

/// Which of the two faces beyond the regular one a family actually has.
///
/// **This exists because SILE answers a request for a face a font has not got
/// with the regular one and no warning** — measured in P4.6, where the runtime
/// ships DejaVu Serif in regular and bold only and thirteen selectors in the
/// built-in style sheet ask for italic. Every one of them was setting in the
/// regular face, and nothing on the page or in the log said so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Faces {
    pub italic: bool,
    pub bold: bool,
}

/// What faces this family has, as the same search order a build resolves by.
///
/// Enumerated rather than queried. `fontdb`'s query *substitutes* — asking it
/// for the italic of a family with no italic returns the regular face rather
/// than nothing, which is exactly the behaviour being checked for and would
/// make this report every font as complete.
pub fn faces_of(family: &str, project_root: &Utf8Path, backend_dirs: &[Utf8PathBuf]) -> Faces {
    let mut db = fontdb::Database::new();
    let project = project_root.join(PROJECT_FONTS);
    if project.exists() {
        db.load_fonts_dir(project.as_std_path());
    }
    for dir in backend_dirs {
        if dir.exists() {
            db.load_fonts_dir(dir.as_std_path());
        }
    }
    db.load_system_fonts();

    let mut faces = Faces::default();
    for face in db.faces() {
        if !face.families.iter().any(|(name, _)| name == family) {
            continue;
        }
        if face.style != fontdb::Style::Normal {
            faces.italic = true;
        }
        if face.weight.0 >= 600 {
            faces.bold = true;
        }
        if !faces.bold && bold_on_an_axis(&db, face.id) {
            faces.bold = true;
        }
    }
    faces
}

/// Whether a face carries its bold on a weight axis rather than in a file.
///
/// **`fontdb` describes a variable font by its default instance**, so a family
/// that holds every weight from Thin to Black on one axis arrives here looking
/// like a family with a single regular face. Noto Serif Devanagari is one file
/// reporting weight 400, and FONT-005 duly warned that the 21 styles asking for
/// bold would each be set in the regular — of the 35 families the font
/// catalogue offers, 23 are variable, so the warning was wrong far more often
/// than it was right. A warning that is wrong by default is how a publisher
/// learns to ignore warnings.
///
/// They are not set in the regular: SILE instantiates the axis. Measured on
/// that same font by setting one line at weight 400 and again at 700 and
/// comparing the PDFs, and by watching it write five instances during one
/// build of the book that found this.
///
/// **The weight axis only, and `ital` and `slnt` deliberately left alone.**
/// SILE picks a face for `style="italic"` through fontconfig rather than by
/// pinning an axis, and nothing here has established that it would reach an
/// italic axis at all. Claiming an italic that never arrives is this same
/// defect pointing the other way, and the quieter direction to be wrong in.
fn bold_on_an_axis(db: &fontdb::Database, id: fontdb::ID) -> bool {
    db.with_face_data(id, |data, index| {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        let axes = face.tables().fvar?.axes;
        Some(axes.into_iter().any(|axis| {
            axis.tag == ttf_parser::Tag::from_bytes(b"wght") && axis.max_value >= 600.0
        }))
    })
    .flatten()
    .unwrap_or(false)
}

/// Report every style whose emphasis the font cannot draw (FONT-005).
///
/// A warning and not an error, and the severity is the point. The page still
/// sets — in the nearest face the family does have — so nothing downstream
/// fails and nothing in the PDF says the emphasis went missing. A publisher
/// finds out when a reader asks why the quotations are not italic.
///
/// One diagnostic per family and missing face, naming the selectors, because
/// a built-in sheet asking for italic in thirteen places should say that once.
pub fn preflight_faces(
    body_family: &str,
    styles: &biblecompose_config::ResolvedStyles,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
    diagnostics: &mut Diagnostics,
) {
    // family -> (selectors wanting italic, selectors wanting bold).
    let mut wanted: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for (selector, resolved) in styles.iter() {
        let style = &resolved.style;
        // A style with no font of its own is set in the body's, which is what
        // makes the built-in sheet's thirteen italics a question about one
        // family rather than thirteen.
        let family = style
            .font_family
            .as_deref()
            .unwrap_or(body_family)
            .to_owned();
        let entry = wanted.entry(family).or_default();
        if style.italic.unwrap_or(false) {
            entry.0.push(selector.key());
        }
        if style.weight.unwrap_or(400) >= 600 {
            entry.1.push(selector.key());
        }
    }

    for (family, (italic, bold)) in wanted {
        if italic.is_empty() && bold.is_empty() {
            continue;
        }
        // **A family nobody has is not a family missing a face.** FONT-001 has
        // already said it cannot be found; adding that it has no italic and no
        // bold is two more diagnostics about the same absence, and they read
        // as separate problems. Measured on a clean machine, where the default
        // font is not installed: one blocking error and two warnings, of which
        // only the error was worth printing.
        let Some(resolved) = resolve(&family, project_root, backend_dirs) else {
            continue;
        };
        let have = faces_of(&family, project_root, backend_dirs);
        for (missing, selectors, word) in
            [(!have.italic, italic, "italic"), (!have.bold, bold, "bold")]
        {
            if !missing || selectors.is_empty() {
                continue;
            }
            // **A missing italic is slanted, and only said.** The class leans
            // the regular for it, as a word processor does, so the emphasis
            // is on the page; a note says it is a slanted regular and not a
            // drawn italic. A project font is a file the backend is pinned
            // to, by path, and is not slanted.
            if word == "italic" && !resolved.from_project {
                diagnostics.push(
                    Diagnostic::info(
                        code::MISSING_FACE,
                        format!(
                            "{family} has no italic face, so {} {} slanted from its regular: {}",
                            if selectors.len() == 1 {
                                "the 1 style that asks for one is"
                            } else {
                                "the styles that ask for one are"
                            },
                            if selectors.len() == 1 { "" } else { "each" },
                            list(&selectors),
                        )
                        .replace("  ", " "),
                    )
                    .help(format!(
                        "install the italic of {family} or put it in {PROJECT_FONTS}/ \
                         for a drawn italic in place of the slanted regular"
                    )),
                );
                continue;
            }
            diagnostics.push(
                Diagnostic::warning(
                    code::MISSING_FACE,
                    format!(
                        "{family} has no {word} face, so the {} {} for one {} be \
                         set in the regular face instead: {}",
                        selectors.len(),
                        if selectors.len() == 1 {
                            "style that asks"
                        } else {
                            "styles that ask"
                        },
                        if selectors.len() == 1 {
                            "will"
                        } else {
                            "will each"
                        },
                        list(&selectors),
                    ),
                )
                .help(format!(
                    "install the {word} of {family}, put it in {PROJECT_FONTS}/, or \
                     choose a family that has one — the page will set either way, \
                     and this is the only warning it gets"
                )),
            );
        }
    }
}

/// The whole check, as diagnostics (FONT-001, FONT-002).
///
/// One error for an unresolvable family, one for a coverage gap. Both block:
/// a build that cannot draw the text has nothing worth publishing at the end
/// of it, and finding out from the paper is finding out too late.
pub fn preflight(
    family: &str,
    doc: &ScriptureDocument,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
    diagnostics: &mut Diagnostics,
) -> Option<ResolvedFont> {
    let Some(font) = resolve(family, project_root, backend_dirs) else {
        diagnostics.push(
            Diagnostic::error(
                code::UNRESOLVED,
                format!("no font called {family:?} is installed, and none is in the project"),
            )
            .help(format!(
                "install it, choose another in the settings, or put the font file in {PROJECT_FONTS}/"
            )),
        );
        return None;
    };

    match gaps(&font, doc) {
        Ok(gaps) if gaps.is_empty() => Some(font),
        Ok(gaps) => {
            diagnostics.push(coverage_error(&font, &gaps, doc));
            Some(font)
        }
        Err(d) => {
            diagnostics.push(d);
            None
        }
    }
}

/// Whether the face the running head is set in can draw what the head prints
/// (FONT-002).
///
/// **The head is the one place the book's own script is set in a face nobody
/// checked.** The body check reads the body's regular face against the
/// Scripture's text. The built-in sheet sets the head in italic, the book name
/// is in the book's script, and a family's italic is often a different file
/// that covers less: Times New Roman's regular draws Arabic and its italic does
/// not. A Romans in Arabic built in Times New Roman passed every check and
/// printed its book name on every page as a row of empty boxes.
///
/// So this asks for the face the head's style resolves to — its family or the
/// body's, its italic, its weight — and checks it against what the head
/// templates print: the book names the templates name, and their literal text.
/// The references are digits and punctuation, which any face that draws the
/// book name draws too, and the check does not pretend to know which verses
/// land on which page.
///
/// An error, as the body's is. A page with a tofu head is no more publishable
/// than one with a tofu body, and it is the same failure: the backend sets
/// `.notdef` and reports success.
pub fn preflight_head(
    settings: &biblecompose_config::Settings,
    body: Option<&ResolvedFont>,
    styles: &biblecompose_config::ResolvedStyles,
    doc: &ScriptureDocument,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
    diagnostics: &mut Diagnostics,
) {
    let style = styles
        .get(biblecompose_config::StyleSelector::RunningHead)
        .style;
    let family = style
        .font_family
        .as_deref()
        .unwrap_or(&settings.typography.font_family);
    // A family nobody has is FONT-001's, said once already.
    let Some(regular) = resolve(family, project_root, backend_dirs) else {
        return;
    };
    let italic = style.italic.unwrap_or(false);
    let weight = style.weight.unwrap_or(400);
    // A project font reaches the backend as a file, and a file is one face:
    // asking SILE for its italic gets that same file. Only a family looked up
    // by name gets the face this asks fontdb for.
    let face = if regular.from_project {
        regular
    } else {
        resolve_face(family, italic, weight, project_root, backend_dirs).unwrap_or(regular)
    };

    // Said once already when the head is set in the body's own face and that
    // face cannot draw the book: the body's error names the script.
    let same_as_body = body.is_some_and(|b| b.path == face.path && b.index == face.index);
    if same_as_body && diagnostics.iter().any(|d| d.code == code::MISSING_COVERAGE) {
        return;
    }

    // The head's three slots on each side. The foot's are the folio's, in a
    // style of their own, and print numbers.
    let headers = &settings.headers;
    let (fields, literal) = template_parts(&[
        headers.left_page.header_left.as_str(),
        headers.left_page.header_center.as_str(),
        headers.left_page.header_right.as_str(),
        headers.right_page.header_left.as_str(),
        headers.right_page.header_center.as_str(),
        headers.right_page.header_right.as_str(),
    ]);
    let mut names: Vec<&str> = Vec::new();
    for book in &doc.books {
        if fields.iter().any(|f| f == "book") {
            names.extend(book.names.for_running_head());
        }
        if fields.iter().any(|f| f == "altbook") {
            names.extend(book.names.for_alternate_head());
        }
    }
    names.sort_unstable();
    names.dedup();

    let characters: std::collections::BTreeSet<char> = names
        .iter()
        .flat_map(|n| n.chars())
        .chain(literal.chars())
        // A ◆ or a ▬ in a template is an ornament, which the class draws
        // itself and not in the head's face.
        .filter(|c| !c.is_whitespace() && !c.is_control() && !matches!(c, '◆' | '▬'))
        .collect();
    if characters.is_empty() {
        return;
    }

    let Ok(data) = std::fs::read(face.path.as_std_path()) else {
        return;
    };
    let Ok(parsed) = ttf_parser::Face::parse(&data, face.index) else {
        return;
    };
    // What the head's face cannot draw, the backend draws in the body's
    // family, at the head's size and weight (the class makes the body family
    // every style's fallback). So a character is missing only when neither
    // has it: a Tamil book name in a head set in Cinzel prints in the body's
    // Tamil face, and the head's own face sets the figures.
    let body_data = body
        .filter(|b| b.path != face.path || b.index != face.index)
        .and_then(|b| {
            std::fs::read(b.path.as_std_path())
                .ok()
                .map(|d| (d, b.index))
        });
    let body_face = body_data
        .as_ref()
        .and_then(|(d, index)| ttf_parser::Face::parse(d, *index).ok());
    let missing: Vec<char> = characters
        .iter()
        .copied()
        .filter(|c| parsed.glyph_index(*c).is_none())
        .filter(|c| {
            body_face
                .as_ref()
                .is_none_or(|b| b.glyph_index(*c).is_none())
        })
        .collect();
    if missing.is_empty() {
        return;
    }

    // The face as a person would name it, from what was asked for: the file's
    // own name for it is a PostScript name nobody types.
    let mut described = family.to_owned();
    if weight >= 600 {
        described.push_str(" bold");
    }
    if italic && !face.from_project {
        described.push_str(" italic");
    }
    let worst = names
        .iter()
        .find(|n| n.chars().any(|c| missing.contains(&c)))
        .map(|n| format!(" — the book name {n:?} would be set as empty boxes"))
        .unwrap_or_default();
    let mut help = String::from("choose a family for `[head]` in styles.toml whose ");
    help.push_str(if italic { "italic" } else { "face" });
    help.push_str(" covers this script");
    if italic {
        help.push_str(
            ", or set `italic = false` under `[head]` if the upright face covers it — \
             many scripts are not set in italic at all",
        );
    }
    let shown: Vec<String> = missing.iter().take(5).map(|c| describe(*c)).collect();

    diagnostics.push(
        Diagnostic::error(
            code::MISSING_COVERAGE,
            format!(
                "the running head is set in {described}, which cannot draw {} character{} \
                 it prints{worst}",
                missing.len(),
                if missing.len() == 1 { "" } else { "s" },
            ),
        )
        .at(SourceLoc::file(face.path.clone()))
        .help(help)
        .detail(format!("missing from {}: {}", face.path, shown.join(", "))),
    );
}

/// The fields a set of head templates name, lower-case and without
/// underscores as the class compares them, and every literal character
/// between them.
fn template_parts(templates: &[&str]) -> (Vec<String>, String) {
    let mut fields = Vec::new();
    let mut literal = String::new();
    for template in templates {
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' | '}' if chars.peek() == Some(&c) => {
                    chars.next();
                    literal.push(c);
                }
                '{' => {
                    let name: String = chars.by_ref().take_while(|c| *c != '}').collect();
                    fields.push(name.replace('_', "").to_lowercase());
                }
                _ => literal.push(c),
            }
        }
    }
    (fields, literal)
}

/// Resolve every font a style names, and report the ones nobody has.
///
/// A style that names a font the machine does not have is the same failure as
/// a body font nobody has (FONT-001), except that it used to arrive as
/// `attempt to index local 'file' (a nil value)` from inside the shaper.
/// Resolved here so it arrives as a sentence naming the style and the font.
///
/// Coverage is deliberately *not* checked per style yet. Doing it honestly
/// means the codepoint set of the text each selector actually applies to, and
/// that is a walk of the model this does not do — so a style font that
/// resolves but cannot draw its own text still reaches the page. That is
/// P5.2's remaining half, and it is now unblocked rather than done.
pub fn preflight_styles(
    styles: &biblecompose_config::ResolvedStyles,
    project_root: &Utf8Path,
    backend_dirs: &[Utf8PathBuf],
    diagnostics: &mut Diagnostics,
) -> BTreeMap<String, ResolvedFont> {
    // Which selectors asked for each family, so one missing font is one
    // diagnostic however many styles named it.
    let mut wanted: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (selector, resolved) in styles.iter() {
        if let Some(family) = resolved.style.font_family.as_deref() {
            wanted.entry(family).or_default().push(selector.key());
        }
    }

    let mut out = BTreeMap::new();
    for (family, selectors) in wanted {
        match resolve(family, project_root, backend_dirs) {
            Some(found) => {
                out.insert(family.to_owned(), found);
            }
            None => diagnostics.push(
                Diagnostic::error(
                    code::UNRESOLVED,
                    format!(
                        "no font called {family:?} is installed, and none is in the project — {} {} for it",
                        list(&selectors),
                        if selectors.len() == 1 { "asks" } else { "ask" },
                    ),
                )
                .help(format!(
                    "install it, choose another in the style editor, or put the font file in {PROJECT_FONTS}/"
                )),
            ),
        }
    }
    out
}

/// The selectors that asked, read aloud.
///
/// Capped at three and then counted. Setting a font on `heading.s1` gives it
/// to every deeper heading by inheritance, so one edit can produce a dozen
/// selectors — and a diagnostic that lists all twelve buries the two facts
/// that matter, which are the font's name and that nobody has it.
fn list(items: &[String]) -> String {
    const SHOWN: usize = 3;
    let named: Vec<String> = items.iter().take(SHOWN).map(|s| format!("`{s}`")).collect();
    let rest = items.len().saturating_sub(SHOWN);

    match (named.as_slice(), rest) {
        ([], _) => String::new(),
        ([one], 0) => one.clone(),
        (some, 0) => match some.split_last() {
            Some((last, first)) => format!("{} and {last}", first.join(", ")),
            None => String::new(),
        },
        (some, more) => format!("{} and {more} more", some.join(", ")),
    }
}

/// A character, named so a person can find it.
///
/// A combining mark has no standalone glyph and debug-formats as an escape,
/// which is unreadable; a letter does have one and 'e' beats U+0065. So: the
/// codepoint always, and the character too when it can be shown on its own.
/// Whether it needed escaping is the test for that.
fn describe(c: char) -> String {
    let quoted = format!("{c:?}");
    let code = format!("U+{:04X}", c as u32);
    if quoted.contains('\\') {
        code
    } else {
        format!("{quoted} ({code})")
    }
}

/// How many places a coverage message lists before it counts the rest.
const PLACES_SHOWN: usize = 5;

/// Where `c` is in the project's files: `file:line:column` for the first
/// [`PLACES_SHOWN`], and how many there are in all.
///
/// A chapter and verse finds the passage, and a stray character such as a
/// U+FFFD left by a bad conversion is found by its line. The model keeps no
/// source positions, so this reads the files its provenance names. That is a
/// second read of each, and it happens only when a build is already failing
/// on a character. A file that cannot be read, such as built-in front matter
/// that is not on disk, is skipped. The column counts characters from 1, as
/// the problems list does.
fn places(doc: &ScriptureDocument, c: char) -> (Vec<SourceLoc>, usize) {
    let mut shown = Vec::new();
    let mut total = 0;
    for source in &doc.provenance {
        let Ok(text) = std::fs::read_to_string(source.path.as_std_path()) else {
            continue;
        };
        for (line, content) in text.lines().enumerate() {
            for (column, found) in content.chars().enumerate() {
                if found != c {
                    continue;
                }
                total += 1;
                if shown.len() < PLACES_SHOWN {
                    shown.push(SourceLoc::at(
                        source.path.clone(),
                        line as u32 + 1,
                        column as u32 + 1,
                    ));
                }
            }
        }
    }
    (shown, total)
}

/// `TIT.usfm:46:99, TIT.usfm:46:100 and 3 more`. The file's name, not its
/// path, which the location already carries in full.
fn list_places(shown: &[SourceLoc], total: usize) -> String {
    let named: Vec<String> = shown
        .iter()
        .map(|at| {
            let file = at.path.file_name().unwrap_or(at.path.as_str());
            match (at.line, at.column) {
                (Some(line), Some(column)) => format!("{file}:{line}:{column}"),
                _ => file.to_owned(),
            }
        })
        .collect();
    match total.saturating_sub(named.len()) {
        0 => named.join(", "),
        more => format!("{} and {more} more", named.join(", ")),
    }
}

/// The message a publisher acts on.
fn coverage_error(font: &ResolvedFont, gaps: &[Gap], doc: &ScriptureDocument) -> Diagnostic {
    let total: usize = gaps.iter().map(|g| g.count).sum();
    // The commonest gap that can name a place. A gap whose only occurrences
    // are in a title is a true finding with nowhere to point, and pointing is
    // most of what makes this message useful.
    let worst = gaps
        .iter()
        .find(|g| g.reference.is_some())
        .unwrap_or(&gaps[0]);

    let mut message = format!(
        "{} cannot draw {} character{} used in this Scripture",
        font.family,
        gaps.len(),
        if gaps.len() == 1 { "" } else { "s" },
    );
    if let Some(reference) = &worst.reference {
        message.push_str(&format!(
            " — {} appears {} time{}, first at {reference}",
            describe(worst.character),
            worst.count,
            if worst.count == 1 { "" } else { "s" },
        ));
    }

    // And where in the files, which is what finds a stray character: the
    // verse says which passage, the line says which byte to fix.
    let (shown, in_files) = places(doc, worst.character);
    if !shown.is_empty() {
        message.push_str(&format!(" — in {}", list_places(&shown, in_files)));
    }

    // A few, not all: a Latin font against Tamil is missing every letter, and
    // a list of two hundred of them is not more informative than a list of
    // five. Each with its first place, when it has one in the files.
    let examples: Vec<String> = gaps
        .iter()
        .take(5)
        .map(|g| {
            let (first, _) = places(doc, g.character);
            match first.first() {
                Some(at) => format!(
                    "{} ×{} (first in {})",
                    describe(g.character),
                    g.count,
                    list_places(std::slice::from_ref(at), 1)
                ),
                None => format!("{} ×{}", describe(g.character), g.count),
            }
        })
        .collect();

    // At the first place the character is, so the problems list names the
    // book and line. The font file it is missing from is in the detail.
    let at = shown
        .into_iter()
        .next()
        .unwrap_or_else(|| SourceLoc::file(font.path.clone()));

    Diagnostic::error(code::MISSING_COVERAGE, message)
        .at(at)
        .help(format!(
            "choose a font that covers this script, or put one in {PROJECT_FONTS}/ — \
             SILE would otherwise set {total} character{} as empty boxes and report success",
            if total == 1 { "" } else { "s" }
        ))
        .detail(format!(
            "missing from {}: {}",
            font.path,
            examples.join(", ")
        ))
}
