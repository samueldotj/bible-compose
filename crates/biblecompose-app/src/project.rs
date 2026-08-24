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

use biblecompose_config::{cascade, ConfigDocument, ResolvedStyles, Settings, TomlFile};
use biblecompose_diagnostics::{Diagnostic, Diagnostics, SourceLoc};
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

/// Where the finished PDF goes, relative to the project folder.
///
/// A constant and not a setting. It was one, and it bought a publisher the
/// ability to move their own output somewhere the application then had to
/// reason about — an absolute path onto another volume, a path inside the
/// source tree, a path that changes between two machines sharing one project
/// — in exchange for a decision nobody was asking to make. The PDF belongs
/// with the book it was made from.
///
/// A subfolder rather than the root, so the one generated file in a folder of
/// Scripture is not sitting among the Scripture. `--output` on the CLI still
/// overrides it: that is an argument to one command, not a property of the
/// project, and a build script redirecting its own output is reasonable.
pub const OUTPUT_DIR: &str = "output";

/// What the PDF is called when the project has not been named.
pub const UNNAMED_OUTPUT: &str = "bible.pdf";

/// A publication's name, as a filename (BLD-003).
///
/// A name is a person's sentence and a filename is not: it may hold a colon, a
/// slash, a quotation mark, a trailing full stop — all of which are ordinary in
/// `The Holy Bible: New Testament` and none of which every filesystem accepts.
///
/// **The rule is what a filesystem rejects, not what an alphabet contains**,
/// and that distinction is the whole of it. Keeping "letters, digits and
/// spaces" reads as safe and is not: `char::is_alphanumeric` is false for a
/// combining mark, so `திருவிவிலியம்` loses its final virama and becomes a
/// different word. Scripture is not published only in Latin, so everything
/// survives except the characters that are actually reserved.
///
/// Returns `None` for a name with nothing usable left in it, because a file
/// called `.pdf` is worse than one called `bible.pdf`.
pub fn output_name(publication: &str) -> Option<String> {
    // Reserved on Windows, on POSIX, or by convention — plus the control
    // characters, which no filesystem wants and some accept.
    const RESERVED: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    let mut out = String::with_capacity(publication.len());
    let mut spaced = false;
    for c in publication.chars() {
        let reject = RESERVED.contains(&c) || c.is_control();
        if !reject && !(c == ' ' && spaced) {
            out.push(c);
            spaced = c == ' ';
        } else if reject && !spaced && !out.is_empty() {
            // One space where a run of reserved characters was, so
            // `Genesis/Exodus` does not become `GenesisExodus`.
            out.push(' ');
            spaced = true;
        }
    }
    // A trailing space or dot makes a name Windows silently renames.
    let trimmed = out.trim().trim_end_matches('.').trim();
    (!trimmed.is_empty()).then(|| format!("{trimmed}.pdf"))
}

/// Start a project: a folder with a settings file in it and nothing else.
///
/// A project is a folder of USFM, so there is nothing to create but the folder
/// and the one file that says what the publication is. Everything else has a
/// built-in answer (CFG-001), which is why a new project is two keys and not a
/// template — a `styles.toml` full of the defaults would be a file a publisher
/// has to maintain in order to change nothing.
///
/// The folder is created inside `parent` and named after the publication.
/// Refuses rather than merges when something is already there: "new project"
/// over an existing folder is either a mistake or a different verb.
pub fn create(parent: &Utf8Path, name: &str, language: &str) -> Result<Utf8PathBuf, Diagnostic> {
    let name = name.trim();
    let language = language.trim();

    if name.is_empty() {
        return Err(Diagnostic::error(
            biblecompose_diagnostics::code::COULD_NOT_CREATE,
            "a publication needs a name",
        )
        .help("the folder is named after it"));
    }

    // The name doubles as a folder name, so it has to survive being one.
    // Checked rather than sanitised: quietly turning "1 & 2 Kings" into
    // "1 _ 2 Kings" is a folder the publisher did not ask for and will not
    // find.
    const RESERVED: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if let Some(bad) = name.chars().find(|c| RESERVED.contains(c)) {
        return Err(Diagnostic::error(
            biblecompose_diagnostics::code::COULD_NOT_CREATE,
            format!("a publication name cannot contain {bad:?}"),
        )
        .help("the name is also the folder's, and this character cannot be in one"));
    }

    let root = parent.join(name);
    if root.exists() {
        return Err(Diagnostic::error(
            biblecompose_diagnostics::code::COULD_NOT_CREATE,
            format!("{root} already exists"),
        )
        .at(SourceLoc::file(root.clone()))
        .help("choose another name, or open the folder that is already there"));
    }

    std::fs::create_dir_all(root.as_std_path()).map_err(|e| {
        Diagnostic::error(
            biblecompose_diagnostics::code::COULD_NOT_CREATE,
            format!("could not create {root}"),
        )
        .at(SourceLoc::file(root.clone()))
        .detail(e.to_string())
    })?;

    let mut file = TomlFile::create(
        root.join(SETTINGS_FILE),
        &TomlFile::settings_header(biblecompose_config::SCHEMA_VERSION),
    );
    file.set("project.name", name.to_owned());
    if !language.is_empty() {
        file.set("project.language", language.to_owned());
    }
    file.save()?;

    Ok(root)
}

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

    /// Where the PDF goes unless the caller says otherwise (CFG-002, BLD-003).
    ///
    /// The settings file wins; otherwise the publication's name; otherwise the
    /// folder's. The folder's name is allowed here and not in the PDF's
    /// properties, and the difference is not a slip: a *filename* is
    /// somewhere the publisher can already see the folder's name, and a
    /// document *property* travels with the file to people who cannot
    /// (ADR-005).
    pub fn output(&self) -> Utf8PathBuf {
        // A name and not a path. A separator or `..` is rejected rather than
        // sanitised, because a publisher who wrote one meant something this
        // application does not do, and quietly doing something else is worse
        // than the file keeping the name it already had.
        let chosen = self
            .settings
            .output
            .name
            .as_deref()
            .map(|n| n.to_string())
            .filter(|n| !n.contains('/') && !n.contains('\\') && n != ".." && !n.is_empty())
            .map(|n| {
                if n.ends_with(".pdf") {
                    n
                } else {
                    format!("{n}.pdf")
                }
            });
        if let Some(name) = chosen {
            return self.root.join(OUTPUT_DIR).join(name);
        }
        let from_settings = self
            .settings
            .project
            .name
            .as_deref()
            .and_then(|n| output_name(n));
        let from_folder = self.root.file_name().and_then(output_name);
        let name = from_settings
            .or(from_folder)
            .unwrap_or_else(|| UNNAMED_OUTPUT.to_owned());
        self.root.join(OUTPUT_DIR).join(name)
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

#[cfg(test)]
mod output_name_tests {
    use super::output_name;

    /// A publication's name is a person's sentence; a filename is not.
    #[test]
    fn a_name_becomes_something_a_filesystem_will_take() {
        assert_eq!(output_name("My Bible").as_deref(), Some("My Bible.pdf"));
        assert_eq!(
            output_name("The Holy Bible: New Testament").as_deref(),
            Some("The Holy Bible New Testament.pdf"),
            "a colon is reserved on Windows"
        );
        assert_eq!(
            output_name("Genesis/Exodus").as_deref(),
            Some("Genesis Exodus.pdf"),
            "a slash would be a directory"
        );
        assert_eq!(
            output_name("A  Bible").as_deref(),
            Some("A Bible.pdf"),
            "runs collapse rather than leaving a double space"
        );
    }

    /// **Scripture is not published only in Latin**, so the rule is what a
    /// filesystem rejects rather than what an alphabet contains.
    #[test]
    fn a_name_in_another_script_survives() {
        assert_eq!(
            output_name("திருவிவிலியம்").as_deref(),
            Some("திருவிவிலியம்.pdf")
        );
        assert_eq!(
            output_name("الكتاب المقدس").as_deref(),
            Some("الكتاب المقدس.pdf")
        );
    }

    /// A trailing space or dot is a name Windows silently renames, which is
    /// worse than being told.
    #[test]
    fn nothing_ends_in_a_space_or_a_dot() {
        assert_eq!(output_name("My Bible.").as_deref(), Some("My Bible.pdf"));
        assert_eq!(output_name("My Bible   ").as_deref(), Some("My Bible.pdf"));
        assert_eq!(output_name("  Bible  ").as_deref(), Some("Bible.pdf"));
    }

    /// And a name with nothing usable in it is no name at all — a file called
    /// `.pdf` is worse than one called `bible.pdf`.
    #[test]
    fn a_name_of_nothing_but_punctuation_is_refused() {
        assert_eq!(output_name("///").as_deref(), None);
        assert_eq!(output_name("").as_deref(), None);
        assert_eq!(output_name("   ").as_deref(), None);
        assert_eq!(output_name("...").as_deref(), None);
    }
}
