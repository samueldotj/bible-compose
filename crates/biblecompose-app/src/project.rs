//! A project folder becoming a `ScriptureDocument`.
//!
//! P1.7's other half. Everything below this was built and tested in isolation
//! — discovery (P1.3), the canon plan (P1.4), normalization (P1.5) — and this
//! is the first thing that runs them in order against a real folder.
//!
//! Orchestration lives here rather than in any of those crates because none of
//! them may know about the others: `biblecompose-project` does not know what a
//! publication is, and `biblecompose-scripture` does not know what a folder
//! is. [ARCHITECTURE §2](../../../docs/ARCHITECTURE.md) puts the arrow from
//! `biblecompose-app` to both of them and nowhere else.

use biblecompose_config::{cascade, ConfigDocument, ResolvedStyles, Settings};
use biblecompose_diagnostics::{Diagnostic, Diagnostics};
use biblecompose_project::discover;
use biblecompose_scripture::normalize::normalize;
use biblecompose_scripture::plan::BookPlan;
use biblecompose_scripture::BookCode;
use biblecompose_scripture::{BookSource, ScriptureDocument};
use camino::{Utf8Path, Utf8PathBuf};

/// What a project folder holds, as a publication.
pub struct Loaded {
    pub document: ScriptureDocument,
    pub diagnostics: Diagnostics,
    /// Books on disk that the settings leave out of the publication.
    ///
    /// Carried because a window has to show them: a book that vanishes from
    /// the list is a book nobody can put back without editing TOML, and
    /// "which books are in" is a question you answer by looking at all of
    /// them. Not parsed — that is the point of leaving them out — so there is
    /// nothing here but the identity and the file.
    pub left_out: Vec<LeftOut>,
}

/// A book the project has but does not publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    pub code: BookCode,
    pub path: Utf8PathBuf,
    /// Where it sat in the full ordered list before it was left out.
    ///
    /// Kept because the window shows one list: a book that is out still has a
    /// place among the ones that are in, and that place is what somebody is
    /// deciding when they tick it back on. Without it the excluded books
    /// could only be shown in a clump at the end, which is not where they are.
    pub position: usize,
}

impl Loaded {
    /// Whether anything found makes a build impossible.
    pub fn blocked(&self) -> bool {
        self.diagnostics.blocking().next().is_some()
    }
}

/// What a project's settings file is called, if it has one.
pub const SETTINGS_FILE: &str = "biblecompose.toml";

/// And its style sheet.
pub const STYLES_FILE: &str = "styles.toml";

/// Read the project's settings.
///
/// CFG-001: a folder with no settings file is the common case and not an
/// error — it gets the built-in defaults. A settings file that will not parse
/// is a different matter: the diagnostic blocks the build, so the defaults
/// returned alongside it are never the ones a PDF gets made from. CFG-003
/// asks for exactly that, because a publisher whose file has a typo in it
/// should not get a book laid out to settings they did not write.
pub fn settings(root: &Utf8Path) -> (Settings, Diagnostics) {
    let path = root.join(SETTINGS_FILE);
    if !path.exists() {
        return (Settings::builtin(), Diagnostics::new());
    }

    match ConfigDocument::read(&path) {
        Ok(doc) => biblecompose_config::resolve(Some(&doc)),
        Err(d) => {
            let mut diagnostics = Diagnostics::new();
            diagnostics.push(d);
            (Settings::builtin(), diagnostics)
        }
    }
}

/// The book plan those settings describe (BOOK-002, BOOK-003).
///
/// Separate from [`settings`] because the canon lives in
/// `biblecompose-scripture` and the settings layer has no business knowing
/// it — resolving `"MAT"` into a book, and saying which codes do not exist, is
/// that crate's job.
pub fn plan(settings: &Settings) -> (BookPlan, Diagnostics) {
    BookPlan::from_settings(
        &settings.books.order,
        settings.books.include.as_ref().map(|i| i.as_slice()),
    )
}

/// A project folder, opened: its settings, its books, and everything either
/// of them had to say about it.
///
/// The composition the CLI and the window both need, in the one crate allowed
/// to orchestrate. Without it each of them would call [`settings`], [`plan`]
/// and [`load`] in the right order and merge three sets of diagnostics — the
/// same four lines, twice, and a chance for the two to disagree about what
/// opening a project means.
pub struct Opened {
    pub root: Utf8PathBuf,
    pub settings: Settings,
    pub styles: ResolvedStyles,
    pub document: ScriptureDocument,
    pub diagnostics: Diagnostics,
    /// Books the folder has and the settings leave out (see [`Loaded`]).
    pub left_out: Vec<LeftOut>,
}

impl Opened {
    /// Whether anything found makes a build impossible.
    pub fn blocked(&self) -> bool {
        self.diagnostics.has_blocking()
    }

    /// Where the PDF goes unless the caller says otherwise (CFG-002).
    pub fn output(&self) -> Utf8PathBuf {
        self.root.join(self.settings.output.file.as_path())
    }
}

/// Read the settings, then the books the settings select.
///
/// Every stage runs even when an earlier one produced errors (DIA-002) —
/// except that a settings file which will not parse closes itself, which
/// [`settings`] handles by returning the built-in values alongside a blocking
/// diagnostic.
pub fn open(root: &Utf8Path) -> Opened {
    let (settings, mut diagnostics) = settings(root);

    let (styles, style_diagnostics) = styles(root, *settings.strict);
    diagnostics.extend(style_diagnostics);

    let (book_plan, plan_diagnostics) = plan(&settings);
    diagnostics.extend(plan_diagnostics);

    let loaded = load(root, &book_plan);
    diagnostics.extend(loaded.diagnostics);

    Opened {
        root: root.to_owned(),
        settings,
        styles,
        document: loaded.document,
        diagnostics,
        left_out: loaded.left_out,
    }
}

/// Read the project's style sheet over the built-in one.
///
/// Same shape as [`settings`], and the same reasoning about a file that will
/// not parse: the diagnostic blocks the build, so the built-in styles returned
/// beside it never lay out a page. `strict` comes from the settings, because a
/// publisher who asked to be stopped by an unrecognised key meant that about
/// the styles too.
pub fn styles(root: &Utf8Path, strict: bool) -> (ResolvedStyles, Diagnostics) {
    let path = root.join(STYLES_FILE);
    if !path.exists() {
        return cascade::resolve(None, strict);
    }

    match ConfigDocument::read(&path) {
        Ok(doc) => cascade::resolve(Some(&doc), strict),
        Err(d) => {
            let (styles, mut diagnostics) = cascade::resolve(None, strict);
            diagnostics.push(d);
            (styles, diagnostics)
        }
    }
}

/// Read a project folder into a publication, applying `plan`.
///
/// **Every stage runs even when an earlier one produced errors** (DIA-002). A
/// publisher fixing a folder wants the whole list, not the first item — so a
/// duplicate book code does not stop the other books from being parsed and
/// reported on.
pub fn load(root: &Utf8Path, plan: &BookPlan) -> Loaded {
    let found = discover(root);
    let mut diagnostics = found.diagnostics;

    // Selection before parsing: a book the project leaves out should not spend
    // time being parsed, and should not contribute diagnostics about a file
    // nobody asked to publish. Ordered first and partitioned second, so the
    // ones left out keep their place among the rest for the window to show.
    let ordered = plan.in_order(found.books, |b| b.book);
    let left_out: Vec<LeftOut> = ordered
        .iter()
        .enumerate()
        .filter(|(_, b)| !plan.includes(b.book))
        .map(|(position, b)| LeftOut {
            code: b.book,
            path: b.path.clone(),
            position,
        })
        .collect();
    let selected: Vec<_> = ordered
        .into_iter()
        .filter(|b| plan.includes(b.book))
        .collect();

    let present = selected.iter().map(|b| b.book).collect();
    for absent in plan.configured_but_absent(&present) {
        diagnostics.push(
            Diagnostic::warning(
                biblecompose_diagnostics::code::UNKNOWN_BOOK_CODE,
                format!("{absent} is configured but no file in the project declares it"),
            )
            .help("add the file, or remove the book from the project's order"),
        );
    }

    let mut books = Vec::new();
    let mut provenance = Vec::new();

    for found in selected {
        let parsed = biblecompose_scripture::usfm::parse(found.path.clone(), found.source);
        diagnostics.extend(parsed.diagnostics);

        let (book, normalized) = normalize(found.book, &found.path, &parsed.document);
        diagnostics.extend(normalized);

        provenance.push(BookSource {
            code: found.book,
            path: found.path,
        });
        books.push(book);
    }

    let mut document = ScriptureDocument::new(books);
    document.provenance = provenance;

    Loaded {
        document,
        diagnostics,
        left_out,
    }
}
