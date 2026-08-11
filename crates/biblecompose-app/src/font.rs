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
    let mut db = fontdb::Database::new();
    match dir {
        Some(dir) if dir.exists() => db.load_fonts_dir(dir.as_std_path()),
        Some(_) => return None,
        None => {}
    }
    if system {
        db.load_system_fonts();
    }

    // By family name only. Weight and style are the backend's to pick from the
    // family, and asking here for a face this document may never use would
    // reject a family over a bold it does not need.
    let id = db.query(&fontdb::Query {
        families: &[fontdb::Family::Name(family)],
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
            diagnostics.push(coverage_error(&font, &gaps));
            Some(font)
        }
        Err(d) => {
            diagnostics.push(d);
            None
        }
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

/// The message a publisher acts on.
fn coverage_error(font: &ResolvedFont, gaps: &[Gap]) -> Diagnostic {
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

    // A few, not all: a Latin font against Tamil is missing every letter, and
    // a list of two hundred of them is not more informative than a list of
    // five.
    let examples: Vec<String> = gaps
        .iter()
        .take(5)
        .map(|g| format!("{} ×{}", describe(g.character), g.count))
        .collect();

    Diagnostic::error(code::MISSING_COVERAGE, message)
        .at(SourceLoc::file(font.path.clone()))
        .help(format!(
            "choose a font that covers this script, or put one in {PROJECT_FONTS}/ — \
             SILE would otherwise set {total} character{} as empty boxes and report success",
            if total == 1 { "" } else { "s" }
        ))
        .detail(format!("missing: {}", examples.join(", ")))
}
