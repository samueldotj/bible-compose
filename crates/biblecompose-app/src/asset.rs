//! Figures: where the file is, whether it is there, and whether it may be
//! used at all (SCR-006, ARCHITECTURE §7).
//!
//! # Why the backend cannot be trusted with any of this
//!
//! Spike [F-14] measured what SILE does with a figure. Two of its three
//! findings are comfortable and the third is not:
//!
//! * A **missing** file, a **wrong-format** file and a **non-image** each stop
//!   the build. Good — but they stop it from inside Lua, with a stack trace
//!   and no reference to the verse the figure was called at.
//! * **Location is never checked.** An absolute path to a valid image well
//!   outside the project embedded silently and produced a correct-looking PDF.
//!   SILE validates *format*, never *provenance*.
//!
//! And the class wrapped the draw in a `pcall`, so even the loud failures were
//! swallowed: a project naming two figures that did not exist built to
//! `[completed]`, wrote a PDF, and left two holes in it. That is the shape of
//! defect this pre-flight layer exists for.
//!
//! SRS §15 requires relative asset references to resolve inside the project
//! directory. The check is here, after both lexical normalization and
//! canonicalization, so that `..` and a symlink are both covered — and it is
//! the only place in the pipeline that performs it.
//!
//! [F-14]: ../../../spike/NOTES.md

use biblecompose_config::value::MissingAsset;
use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, ScriptureRef};
use biblecompose_scripture::{Block, FigureRef, Inline, ScriptureDocument};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

/// The image formats S0.7 placed successfully, by their first bytes.
///
/// Sniffed rather than taken from the extension. A PNG named `.dat` is still a
/// PNG and refusing it would be this layer inventing a rule the backend does
/// not have; a `.png` that is actually an SVG is the failure worth catching,
/// and only the bytes know.
const TESTED: [(&str, &[u8]); 3] = [
    ("PNG", b"\x89PNG\r\n\x1a\n"),
    ("JPEG", b"\xff\xd8"),
    ("PDF", b"%PDF-"),
];

/// Image formats the reader recognises but this release has never placed.
///
/// libtexpdf may well draw them. "May well" is not something to find out
/// during a print run, so they get a warning naming what is known to work
/// rather than an error refusing what might.
const UNTESTED: [(&str, &[u8]); 2] = [("GIF", b"GIF8"), ("BMP", b"BM")];

/// What a build should do with each figure it was asked to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preflight {
    /// Figures the backend must not be asked for, because the file is not
    /// there and the project said to carry on without it.
    pub omitted: Vec<Utf8PathBuf>,
}

/// Check every figure in the document (SCR-006).
///
/// `columns` decides only whether `size="span"` can be honoured; see
/// [`span_unsupported`].
pub fn preflight(
    doc: &ScriptureDocument,
    project_root: &Utf8Path,
    policy: MissingAsset,
    columns: u8,
    diagnostics: &mut Diagnostics,
) -> Preflight {
    let root = normalize(project_root);
    let mut omitted = Vec::new();
    let mut spanned = false;
    let mut plated = false;

    for (figure, at) in figures(doc) {
        // `span` is about the page rather than the file, so it is checked
        // whatever happens to the artwork itself.
        if !spanned && figure.size.as_deref() == Some("span") && columns >= 2 {
            spanned = true;
            diagnostics.push(span_unsupported(columns));
        }

        let target = normalize(&root.join(&figure.src));

        // Containment first. A figure outside the project is refused whether
        // or not the file exists, and saying "no such file" about a path this
        // project may not read either way would be answering the wrong
        // question — and would report on a directory that is none of its
        // business.
        if !target.starts_with(&root) {
            diagnostics.push(outside(figure, at.as_ref()));
            continue;
        }

        let Ok(bytes) = first_bytes(&target) else {
            match policy {
                MissingAsset::Stop => diagnostics.push(missing(figure, at.as_ref(), true)),
                MissingAsset::Omit => {
                    diagnostics.push(missing(figure, at.as_ref(), false));
                    omitted.push(figure.src.clone());
                }
            }
            continue;
        };

        // And again once the filesystem has had its say: a symlink inside the
        // project pointing out of it passes the check above and fails here.
        if let Some(real) = canonical(&target) {
            if canonical(&root).is_some_and(|r| !real.starts_with(&r)) {
                diagnostics.push(outside(figure, at.as_ref()).help(format!(
                    "{} is a link to {real}, which is outside the project",
                    figure.src
                )));
                continue;
            }
        }

        if bytes.starts_with(b"%PDF-") {
            if !plated {
                plated = true;
                diagnostics.push(pdf_artwork());
            }
            continue;
        }
        if TESTED.iter().any(|(_, magic)| bytes.starts_with(magic)) {
            continue;
        }
        match UNTESTED.iter().find(|(_, magic)| bytes.starts_with(magic)) {
            Some((name, _)) => diagnostics.push(untested_format(figure, name, at.as_ref())),
            None => diagnostics.push(not_an_image(figure, at.as_ref())),
        }
    }

    Preflight { omitted }
}

/// Every figure, with the reference it sits at.
///
/// Figures are top-level blocks — a table cell holds inlines, not blocks — so
/// this is a walk over each book's block list, keeping the running chapter and
/// verse so a diagnostic can say where in the Scripture to look rather than
/// only which file is wrong.
fn figures(doc: &ScriptureDocument) -> Vec<(&FigureRef, Option<ScriptureRef>)> {
    let mut out = Vec::new();
    for book in &doc.books {
        let name = book
            .names
            .for_running_head()
            .unwrap_or_else(|| book.code.as_str())
            .to_owned();
        let mut chapter: u16 = 0;
        let mut verse: Option<u16> = None;

        for block in &book.blocks {
            if let Block::Figure(figure) = block {
                out.push((
                    figure,
                    (chapter > 0).then(|| ScriptureRef {
                        book: name.clone(),
                        chapter,
                        verse,
                    }),
                ));
                continue;
            }
            block.each_inline(&mut |inline| match inline {
                Inline::Chapter { number, .. } => {
                    chapter = *number;
                    verse = None;
                }
                Inline::Verse { id, .. } => verse = Some(id.start),
                _ => {}
            });
        }
    }
    out
}

/// `.` and `..` resolved without touching the filesystem.
///
/// Lexical, and deliberately so: a `..` that escapes the project has to be
/// caught whether or not the file it names exists, and
/// [`std::fs::canonicalize`] answers nothing about a path that does not.
fn normalize(path: &Utf8Path) -> Utf8PathBuf {
    let mut out = Utf8PathBuf::new();
    for part in path.components() {
        match part {
            Utf8Component::CurDir => {}
            // A `..` at the root is dropped rather than kept, which is what
            // the filesystem does. The escape is still caught, because what is
            // left no longer begins with the project's own path.
            Utf8Component::ParentDir => {
                out.pop();
            }
            Utf8Component::Prefix(p) => out.push(p.as_str()),
            Utf8Component::RootDir => out.push(std::path::MAIN_SEPARATOR_STR),
            Utf8Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// The real path, with symlinks followed. `None` when the platform will not
/// say — which is not evidence of anything, so the caller treats it as no
/// finding rather than as a failure.
fn canonical(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let real = std::fs::canonicalize(path.as_std_path()).ok()?;
    Utf8PathBuf::from_path_buf(real).ok()
}

/// Enough of the file to recognise its format, and proof that it is readable.
fn first_bytes(path: &Utf8Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path.as_std_path())?;
    let mut head = [0u8; 16];
    let read = file.read(&mut head)?;
    Ok(head[..read].to_vec())
}

/// Where to tell a publisher to look.
///
/// **The chapter, not the verse.** USFM's `\fig` is a character marker, so it
/// arrives inside a paragraph, and normalization hoists it to a block after
/// that paragraph — where a float would land anyway, and where SILE decides
/// its final position (P1.5). By the time the figure is reached, the running
/// verse is the paragraph's last rather than the one it was written beside, so
/// naming it would be precise and wrong.
///
/// A figure that carries USFM's own `ref` says it better than either, so that
/// wins where it is given.
fn where_at(figure: &FigureRef, at: Option<&ScriptureRef>) -> String {
    if let Some(reference) = attribute(figure, "ref") {
        return format!(" at {reference}");
    }
    match at {
        Some(r) => format!(" in {} {}", r.book, r.chapter),
        None => String::new(),
    }
}

fn attribute<'a>(figure: &'a FigureRef, key: &str) -> Option<&'a str> {
    figure
        .attributes
        .iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

fn outside(figure: &FigureRef, at: Option<&ScriptureRef>) -> Diagnostic {
    let src = &figure.src;
    Diagnostic::error(
        code::OUTSIDE_PROJECT,
        format!(
            "the figure{} points at {src}, which is outside the project folder",
            where_at(figure, at)
        ),
    )
    .help("copy the file into the project and refer to it by a relative path — a book that only builds on the machine its artwork happens to live on is not a book that can be handed over")
}

fn missing(figure: &FigureRef, at: Option<&ScriptureRef>, blocking: bool) -> Diagnostic {
    let src = &figure.src;
    let message = format!(
        "the figure{} names {src}, which is not there",
        where_at(figure, at)
    );
    if blocking {
        Diagnostic::error(code::MISSING, message).help(
            "add the file, correct the path, or set `assets.missing_figure = \"omit\"` to \
             leave the figure out and carry on",
        )
    } else {
        Diagnostic::warning(code::MISSING, message)
            .help("left out, because `assets.missing_figure` is \"omit\"")
    }
}

fn untested_format(figure: &FigureRef, format: &str, at: Option<&ScriptureRef>) -> Diagnostic {
    let src = &figure.src;
    Diagnostic::warning(
        code::UNSUPPORTED_FORMAT,
        format!(
            "the figure{} is a {format}, which this release has not placed",
            where_at(figure, at)
        ),
    )
    .help(format!(
        "{src}: PNG, JPEG and PDF are the formats known to work"
    ))
}

fn not_an_image(figure: &FigureRef, at: Option<&ScriptureRef>) -> Diagnostic {
    let src = &figure.src;
    Diagnostic::error(
        code::UNSUPPORTED_FORMAT,
        format!(
            "the figure{} names {src}, which is not an image the backend can read",
            where_at(figure, at)
        ),
    )
    .help("PNG, JPEG and PDF are the formats known to work")
}

/// What a PDF brings with it, said once however many plates a book has.
///
/// Measured rather than assumed (P4.3). A six-by-nine plate lifted from
/// another publication placed as a Form XObject with `/BBox [0 0 432 648]` —
/// its *whole page*, margins, running head and folio included, scaled to the
/// column — and its two embedded font subsets joined the output's, taking a
/// two-font PDF to four.
///
/// Neither is a defect and neither can be refused: it is what including a PDF
/// means. It is worth one line in a build log, because a printer's pre-flight
/// will list those fonts, embedding a face is the act a licence governs, and
/// "the plate has a page number on it" is far easier to fix before the run
/// than after.
fn pdf_artwork() -> Diagnostic {
    Diagnostic::info(
        code::PDF_ARTWORK,
        "a figure is a PDF, so its whole page box and its embedded fonts become part of the output",
    )
    .help(
        "crop the plate to its artwork if it carries margins or a folio, and check that its fonts are ones this publication may embed",
    )
}

/// `size="span"` asks for artwork across the full measure, and a two-column
/// page cannot give it one.
///
/// SILE places a figure in the frame the text is flowing through, and in two
/// columns that frame *is* one column — there is no mid-flow way to reach
/// across the gutter. The figure is set at column width instead, which is a
/// reasonable answer and a silent one, so it is said out loud once.
fn span_unsupported(columns: u8) -> Diagnostic {
    Diagnostic::warning(
        code::SIZE_UNSUPPORTED,
        format!(
            "a figure asks for `size=\"span\"`, which needs one column and this \
             publication has {columns}"
        ),
    )
    .help("set at column width instead; `page.columns = 1` is what span needs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> Utf8PathBuf {
        Utf8PathBuf::from(if cfg!(windows) { "C:/proj" } else { "/proj" })
    }

    #[test]
    fn a_relative_path_stays_inside() {
        let r = normalize(&root());
        assert!(normalize(&r.join("art/map.png")).starts_with(&r));
        assert!(normalize(&r.join("./art/./map.png")).starts_with(&r));
        assert!(normalize(&r.join("art/../art/map.png")).starts_with(&r));
    }

    /// F-14, as an assertion: `..` escapes and has to be caught lexically,
    /// because the file it names may not exist and there is nothing to
    /// canonicalize.
    #[test]
    fn a_parent_reference_escapes() {
        let r = normalize(&root());
        assert!(!normalize(&r.join("../elsewhere.png")).starts_with(&r));
        assert!(!normalize(&r.join("art/../../elsewhere.png")).starts_with(&r));
    }

    /// More `..` than there are components does not wrap around into looking
    /// contained again.
    #[test]
    fn climbing_past_the_root_is_still_outside() {
        let r = normalize(&root());
        let far = normalize(&r.join("../../../../../../etc/passwd"));
        assert!(!far.starts_with(&r), "{far}");
    }

    /// `join` with an absolute path replaces rather than appends, which is
    /// what makes an absolute `src` reach the containment check at all.
    #[test]
    fn an_absolute_source_is_checked_where_it_actually_points() {
        let r = normalize(&root());
        let elsewhere = if cfg!(windows) {
            "C:/somewhere/map.png"
        } else {
            "/somewhere/map.png"
        };
        assert!(!normalize(&r.join(elsewhere)).starts_with(&r));
    }

    #[test]
    fn the_tested_formats_are_recognised_by_their_first_bytes() {
        for (name, magic) in TESTED {
            assert!(
                TESTED.iter().any(|(_, m)| magic.starts_with(m)),
                "{name} does not match its own signature"
            );
        }
        assert!(!TESTED.iter().any(|(_, m)| b"<svg xmlns=".starts_with(m)));
        assert!(!UNTESTED.iter().any(|(_, m)| b"<svg xmlns=".starts_with(m)));
    }
}
