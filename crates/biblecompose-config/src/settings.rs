//! The settings schema, the embedded defaults, and the merge between them.
//!
//! CFG-001 and CFG-002: a USFM-only folder builds because every value has a
//! built-in answer, and a project file that sets one key changes one key.
//!
//! # How the merge cannot go wrong
//!
//! Both sides are read by the same function through the same [`Node`] API. A
//! default and an override therefore cannot be validated differently, and a
//! default that would be rejected from a project file fails our own test suite
//! rather than shipping. There is one deliberate asymmetry: a project value
//! that fails validation is reported and the built-in one is kept, because
//! `page.size = "quarto"` should cost the publisher a page size and not the
//! whole file.
//!
//! Every field is a [`Sourced`], so what a build used and where it came from
//! are the same read ([ADR-005]).
//!
//! [ADR-005]: ../../../docs/adr/005-provenance.md

use std::collections::BTreeSet;

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, Severity, SourceLoc};

use crate::document::{ConfigDocument, Located, Node};
use crate::provenance::{Provenance, Sourced};
use crate::value::{
    self, CallerStyle, HeadSlot, Length, MissingAsset, PageSize, ReferencePlacement,
    RestartNumbering,
};

/// The settings vocabulary this release speaks.
pub const SCHEMA_VERSION: i64 = 1;

/// The built-in defaults, as the TOML a project file overrides.
///
/// Compiled in, so it cannot be missing, edited or shadowed by a file on the
/// machine — the answer to "what is the default" is the same everywhere.
pub const DEFAULTS_TOML: &str = include_str!("../defaults.toml");

/// The embedded defaults, parsed.
///
/// Panicking is right here and nowhere else: this file ships inside the
/// executable, and [`the_embedded_defaults_are_valid`] proves it parses and
/// resolves without a single diagnostic before any of it is released.
///
/// [the_embedded_defaults_are_valid]: ../../tests/settings.rs
pub fn defaults() -> ConfigDocument {
    ConfigDocument::parse("<built-in defaults>", DEFAULTS_TOML.to_owned())
        .expect("the embedded defaults are valid TOML")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Where every value above came from, by key ([ADR-005]).
    ///
    /// The typed fields carry their own origin; this is the string-keyed index
    /// over them, for the two callers that cannot name a field at compile
    /// time — the inspector and reset-to-default.
    ///
    /// [ADR-005]: ../../../docs/adr/005-provenance.md
    pub provenance: Provenance,
    /// CFG-004: whether a key this release does not recognise stops the build
    /// rather than warning. Off by default, because a settings file written
    /// for a later release should degrade rather than fail; on for a publisher
    /// who would rather find out.
    pub strict: Sourced<bool>,
    pub project: Project,
    pub books: Books,
    pub page: Page,
    pub typography: Typography,
    pub numbering: Numbering,
    pub contents: Contents,
    pub notes: Notes,
    pub headers: Headers,
    pub assets: Assets,
    pub output: Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// Absent means "use the folder's name". There is no default publication
    /// name that is right for anybody.
    pub name: Option<Sourced<String>>,
    /// A BCP-47 tag, for hyphenation and language-aware breaking.
    pub language: Sourced<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Books {
    pub order: Sourced<Vec<String>>,
    /// `None` is "everything discovered"; `Some(empty)` is a project that has
    /// selected nothing, which the caller reports rather than treats as the
    /// same thing.
    pub include: Option<Sourced<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub size: Sourced<PageSize>,
    pub columns: Sourced<u8>,
    pub margin_top: Sourced<Length>,
    pub margin_bottom: Sourced<Length>,
    pub margin_inner: Sourced<Length>,
    pub margin_outer: Sourced<Length>,
    pub column_gap: Sourced<Length>,
    pub header_gap: Sourced<Length>,
    pub footer_gap: Sourced<Length>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    pub font_family: Sourced<String>,
    pub font_size: Sourced<Length>,
    pub leading: Sourced<Length>,
    pub hyphenation: Sourced<bool>,
    /// Justified, or ragged at the outer edge. Justified is what a Bible is
    /// normally set in and what the backend does by default.
    pub justify: Sourced<bool>,
    /// Whether a poetry line keeps the indent its level asks for. Off sets
    /// every line flush, which some editions want and which is otherwise a
    /// style override per level.
    pub keep_poetry_indentation: Sourced<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Numbering {
    pub show_chapter_numbers: Sourced<bool>,
    pub show_verse_numbers: Sourced<bool>,
    /// Whether verse 1 goes unnumbered where a chapter number already marks
    /// the place. A common setting in Bible typography and an odd one
    /// everywhere else, which is why it is a setting and not a style.
    pub hide_first_verse_number: Sourced<bool>,
    /// Whether USFM's `\cl` is printed — the words an edition gives a chapter,
    /// such as `\cl அத்தியாயம் 1` beside `\c 1`.
    ///
    /// Here rather than under `contents` because what a publisher is deciding
    /// is how a chapter is announced, and the number beside it is the other
    /// half of that decision. A translation that carries labels and an edition
    /// that wants only figures are both ordinary, and the file says which.
    pub show_chapter_labels: Sourced<bool>,
}

/// Which parts of a book are printed at all.
///
/// Everything here is in the document either way: turning an introduction off
/// hides it, it does not drop it (ADR-002). A project that prints a Gospel
/// without its introduction for a school edition and with it for the study
/// edition is one setting, not two files.
#[derive(Debug, Clone, PartialEq)]
pub struct Contents {
    pub show_book_introductions: Sourced<bool>,
    pub show_introductory_outlines: Sourced<bool>,
    pub show_section_headings: Sourced<bool>,
}

/// Footnotes and cross-references: whether, how marked, and — for references —
/// where (SCR-003 – SCR-005).
///
/// The two kinds keep separate caller settings on purpose. A page carrying both
/// needs to say which mark belongs to which apparatus, and the way editions do
/// that is by giving them different sequences — numbers against letters — not
/// by interleaving one sequence between them. That is the whole of what P4.2's
/// "styled independently of footnotes" asks for at the level of the mark.
#[derive(Debug, Clone, PartialEq)]
pub struct Notes {
    pub show_footnotes: Sourced<bool>,
    pub show_cross_references: Sourced<bool>,
    pub footnote_callers: Sourced<CallerStyle>,
    pub cross_reference_callers: Sourced<CallerStyle>,
    /// Applies to both sequences, because a page whose footnotes restart at a
    /// chapter and whose references do not is a page with two different
    /// answers to the same question.
    pub restart_numbering: Sourced<RestartNumbering>,
    pub cross_reference_placement: Sourced<ReferencePlacement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Headers {
    pub header_left: Sourced<HeadSlot>,
    pub header_center: Sourced<HeadSlot>,
    pub header_right: Sourced<HeadSlot>,
    pub footer_left: Sourced<HeadSlot>,
    pub footer_center: Sourced<HeadSlot>,
    pub footer_right: Sourced<HeadSlot>,
}

/// The files a project points at that are not Scripture (SCR-006).
#[derive(Debug, Clone, PartialEq)]
pub struct Assets {
    pub missing_figure: Sourced<MissingAsset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub keep_intermediates: Sourced<bool>,
}

/// One of the seven things a head or a footer can hold.
fn slot(n: &Node) -> Result<Located<HeadSlot>, Diagnostic> {
    value::choice(n, HeadSlot::NAMES)
}

fn caller_style(n: &Node) -> Result<Located<CallerStyle>, Diagnostic> {
    value::choice(n, CallerStyle::NAMES)
}

fn restart(n: &Node) -> Result<Located<RestartNumbering>, Diagnostic> {
    value::choice(n, RestartNumbering::NAMES)
}

fn placement(n: &Node) -> Result<Located<ReferencePlacement>, Diagnostic> {
    value::choice(n, ReferencePlacement::NAMES)
}

fn missing_asset(n: &Node) -> Result<Located<MissingAsset>, Diagnostic> {
    value::choice(n, MissingAsset::NAMES)
}

/// The maximum a page can be divided into before a column is too narrow to
/// break a line of Scripture in.
const MAX_COLUMNS: i64 = 4;

impl Settings {
    /// Everything built in. What a folder with no `biblecompose.toml` gets.
    pub fn builtin() -> Settings {
        let (settings, diagnostics) = resolve(None);
        debug_assert!(
            diagnostics.is_empty(),
            "the embedded defaults produced diagnostics: {:?}",
            diagnostics.iter().collect::<Vec<_>>()
        );
        settings
    }
}

/// Every settings key this release understands, in key order.
///
/// Derived by running resolution with no project file and keeping the list of
/// keys it asked for, so it is the same list CFG-004 checks against and the
/// same list the GUI can enumerate. There is no separately written schema to
/// fall out of step with.
pub fn known_keys() -> BTreeSet<String> {
    let defaults = defaults();
    let mut r = Resolver {
        defaults: &defaults,
        project: None,
        diagnostics: Diagnostics::new(),
        provenance: Provenance::default(),
        asked: BTreeSet::new(),
    };
    r.asked.insert("schema_version".to_owned());
    let _ = resolve_fields(&mut r);
    r.asked
}

/// Merge the embedded defaults with a project file, field by field.
///
/// Never fails. A settings file cannot be so wrong that there is no answer,
/// because there is always a built-in one; what it can do is produce a list of
/// diagnostics, which the caller decides whether to block on (DIA-002).
pub fn resolve(project: Option<&ConfigDocument>) -> (Settings, Diagnostics) {
    let defaults = defaults();
    let mut r = Resolver {
        defaults: &defaults,
        project: None,
        diagnostics: Diagnostics::new(),
        provenance: Provenance::default(),
        asked: BTreeSet::new(),
    };

    // The version gate runs before anything else is read, and closes the
    // project file rather than merely warning about it. ARCHITECTURE §6: "an
    // unknown version is one clear diagnostic instead of a cascade of unknown
    // field errors" — which is only true if the cascade never happens.
    r.asked.insert("schema_version".to_owned());
    if let Some(doc) = project {
        if check_schema_version(doc, &mut r.diagnostics) {
            r.project = Some(doc);
        }
    }

    let mut settings = resolve_fields(&mut r);

    settings.provenance = std::mem::take(&mut r.provenance);

    // CFG-004, last: everything the file said that nothing above asked for.
    // After resolution rather than during, because `asked` is only complete
    // once every field has been read.
    if let Some(doc) = r.project {
        let severity = if *settings.strict {
            Severity::Error
        } else {
            Severity::Warning
        };
        report_unknown_keys(doc, &r.asked, severity, &mut r.diagnostics);
    }

    (settings, r.diagnostics)
}

/// Report every key in the project file that resolution never looked for.
///
/// The set of legal keys is *what the resolver asked for*, not a list written
/// out separately. A second list is a second thing to update when a setting is
/// added, and when it is forgotten the result is a warning about a key that
/// works — which is how a publisher learns to ignore warnings.
fn report_unknown_keys(
    doc: &ConfigDocument,
    asked: &BTreeSet<String>,
    severity: Severity,
    diagnostics: &mut Diagnostics,
) {
    fn walk(
        table: &crate::document::Table<'_>,
        asked: &BTreeSet<String>,
        severity: Severity,
        diagnostics: &mut Diagnostics,
    ) {
        for name in table.names() {
            let Some(node) = table.get(name) else {
                continue;
            };
            let path = node.dotted_path().to_owned();

            if asked.contains(&path) {
                continue;
            }

            // A table we know part of — `[page]` with a typo in it — is
            // descended into, so the complaint lands on the stray key. A table
            // we know nothing about is reported once, at its header: eight
            // warnings about the inside of `[gribble]` say less than one about
            // `[gribble]`.
            let prefix = format!("{path}.");
            let known_within = asked.iter().any(|k| k.starts_with(&prefix));
            if known_within {
                if let Ok(inner) = node.table() {
                    walk(&inner, asked, severity, diagnostics);
                    continue;
                }
            }

            diagnostics.push(
                Diagnostic::new(
                    severity,
                    code::UNKNOWN_KEY,
                    format!("`{path}` is not a setting this release recognises"),
                )
                .at(node.loc())
                .help(match REMOVED.iter().find(|(key, _)| *key == path) {
                    Some((_, instead)) => (*instead).to_owned(),
                    None => match nearest_setting(&path, asked) {
                        Some(near) => format!("did you mean `{near}`?"),
                        None => "remove it, or check the spelling against the settings \
                                 documentation"
                            .to_owned(),
                    },
                }),
            );
        }
    }

    walk(
        &doc.root().table().expect("a document root is a table"),
        asked,
        severity,
        diagnostics,
    );
}

/// Keys this release used to have, and what to do instead.
///
/// A removed setting is not a misspelling, and "did you mean `books.include`?"
/// is a poor answer to somebody who wrote exactly what the documentation said
/// last release. It is still reported as an unknown key — because it is one,
/// and because a setting silently ignored is a publication quietly losing a
/// book — but the help says what actually happened.
const REMOVED: [(&str, &str); 6] = [
    (
        "books.exclude",
        concat!(
            "`books.exclude` was removed: name the books you want in ",
            "`books.include` instead, which says the same thing without two ",
            "settings that can disagree",
        ),
    ),
    (
        "headers.enabled",
        concat!(
            "`headers.enabled` was removed: a head is now what its three slots ",
            "hold, so an empty head is `headers.header_left`, `header_center` ",
            "and `header_right` all set to \"empty\"",
        ),
    ),
    (
        "headers.show_book_name",
        concat!(
            "`headers.show_book_name` was removed: put \"book_name\" in whichever ",
            "of `headers.header_left`, `header_center` or `header_right` it ",
            "should occupy — which is the part the old setting could not say",
        ),
    ),
    (
        "headers.show_reference_range",
        concat!(
            "`headers.show_reference_range` was removed: put \"reference_range\" ",
            "in one of `headers.header_left`, `header_center` or `header_right`",
        ),
    ),
    (
        "headers.show_page_number",
        concat!(
            "`headers.show_page_number` was removed: put \"page_number\" in one ",
            "of the header or footer slots — `headers.footer_center` is where ",
            "it used to be",
        ),
    ),
    (
        "output.file",
        concat!(
            "`output.file` was removed: the PDF is always written to ",
            "`output/bible.pdf` inside the project folder, so that it stays ",
            "with the book it was made from",
        ),
    ),
];

/// The closest known key, if it is close enough to be a slip. Compared on the
/// whole dotted path, so `page.wdith` finds `page.width` and does not offer
/// `books.order`.
fn nearest_setting(given: &str, asked: &BTreeSet<String>) -> Option<String> {
    asked
        .iter()
        .map(|k| (value::distance(given, k), k))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k.clone())
}

/// The fields, read in order. Split out of [`resolve`] so that
/// [`known_keys`] can run the same reads without a project file and keep only
/// the list of keys they asked for.
fn resolve_fields(r: &mut Resolver<'_>) -> Settings {
    Settings {
        // Filled in below: resolution has to finish before the index over it
        // is complete.
        provenance: Provenance::default(),
        strict: r.value("strict", |n| n.boolean()),
        project: Project {
            name: r.optional("project.name", |n| n.string()),
            language: r.value("project.language", |n| n.string()),
        },
        books: Books {
            order: r.list("books.order"),
            include: r.optional_list("books.include"),
        },
        page: Page {
            size: r.value("page.size", value::page_size),
            columns: r.value("page.columns", |n| {
                value::integer_in(n, 1, MAX_COLUMNS).map(|l| l.map(|v| v as u8))
            }),
            margin_top: r.value("page.margin_top", value::length_or_zero),
            margin_bottom: r.value("page.margin_bottom", value::length_or_zero),
            margin_inner: r.value("page.margin_inner", value::length_or_zero),
            margin_outer: r.value("page.margin_outer", value::length_or_zero),
            column_gap: r.value("page.column_gap", value::length_or_zero),
            header_gap: r.value("page.header_gap", value::length_or_zero),
            footer_gap: r.value("page.footer_gap", value::length_or_zero),
        },
        typography: Typography {
            font_family: r.value("typography.font_family", |n| n.string()),
            font_size: r.value("typography.font_size", value::length),
            leading: r.value("typography.leading", value::length),
            hyphenation: r.value("typography.hyphenation", |n| n.boolean()),
            justify: r.value("typography.justify", |n| n.boolean()),
            keep_poetry_indentation: r.value("typography.keep_poetry_indentation", |n| n.boolean()),
        },
        numbering: Numbering {
            show_chapter_numbers: r.value("numbering.show_chapter_numbers", |n| n.boolean()),
            show_verse_numbers: r.value("numbering.show_verse_numbers", |n| n.boolean()),
            hide_first_verse_number: r.value("numbering.hide_first_verse_number", |n| n.boolean()),
            show_chapter_labels: r.value("numbering.show_chapter_labels", |n| n.boolean()),
        },
        contents: Contents {
            show_book_introductions: r.value("contents.show_book_introductions", |n| n.boolean()),
            show_introductory_outlines: r
                .value("contents.show_introductory_outlines", |n| n.boolean()),
            show_section_headings: r.value("contents.show_section_headings", |n| n.boolean()),
        },
        notes: Notes {
            show_footnotes: r.value("notes.show_footnotes", |n| n.boolean()),
            show_cross_references: r.value("notes.show_cross_references", |n| n.boolean()),
            footnote_callers: r.value("notes.footnote_callers", caller_style),
            cross_reference_callers: r.value("notes.cross_reference_callers", caller_style),
            restart_numbering: r.value("notes.restart_numbering", restart),
            cross_reference_placement: r.value("notes.cross_reference_placement", placement),
        },
        headers: Headers {
            header_left: r.value("headers.header_left", slot),
            header_center: r.value("headers.header_center", slot),
            header_right: r.value("headers.header_right", slot),
            footer_left: r.value("headers.footer_left", slot),
            footer_center: r.value("headers.footer_center", slot),
            footer_right: r.value("headers.footer_right", slot),
        },
        assets: Assets {
            missing_figure: r.value("assets.missing_figure", missing_asset),
        },
        output: Output {
            keep_intermediates: r.value("output.keep_intermediates", |n| n.boolean()),
        },
    }
}

/// True if the project file may be read.
fn check_schema_version(doc: &ConfigDocument, diagnostics: &mut Diagnostics) -> bool {
    let Some(node) = doc.find("schema_version") else {
        // A warning rather than an error, deliberately. There is exactly one
        // version, so assuming it is safe; and refusing every settings file
        // written before the key existed would punish early publishers for a
        // problem versioning exists to prevent later. The help says the line
        // to add, which is what gets it into files.
        diagnostics.push(
            Diagnostic::warning(
                code::UNKNOWN_SCHEMA_VERSION,
                "this settings file does not say which settings version it is written for",
            )
            .at(SourceLoc::file(doc.path().to_owned()))
            .help(format!(
                "add `schema_version = {SCHEMA_VERSION}` as the first line; \
                 it is assumed for now"
            )),
        );
        return true;
    };

    let version = match node.integer() {
        Ok(v) => v,
        Err(d) => {
            diagnostics.push(d);
            return false;
        }
    };

    if version.value == SCHEMA_VERSION {
        return true;
    }

    let (message, help) = if version.value > SCHEMA_VERSION {
        (
            format!(
                "this settings file is written for settings version {}, and this \
                 release understands version {SCHEMA_VERSION}",
                version.value
            ),
            "a newer BibleCompose wrote it — update the application rather than the file",
        )
    } else {
        (
            format!(
                "settings version {} is no longer understood; this release speaks \
                 version {SCHEMA_VERSION}",
                version.value
            ),
            "nothing in the file is being used — the built-in defaults are in force",
        )
    };

    diagnostics.push(
        Diagnostic::error(code::UNKNOWN_SCHEMA_VERSION, message)
            .at(version.loc)
            .help(help),
    );
    false
}

/// The merge itself, written once and applied to every field.
struct Resolver<'a> {
    defaults: &'a ConfigDocument,
    project: Option<&'a ConfigDocument>,
    diagnostics: Diagnostics,
    provenance: Provenance,
    /// Every key this resolution looked for.
    ///
    /// CFG-004's "unknown key" is defined against *this* rather than against a
    /// list of legal keys written out somewhere else. A second list is a
    /// second thing to update when a setting is added, and the failure when it
    /// is forgotten is a warning about a key that works perfectly well —
    /// which teaches a publisher to ignore the warnings.
    asked: BTreeSet<String>,
}

impl Resolver<'_> {
    /// The project's value if it has a usable one, otherwise the built-in.
    fn value<T>(
        &mut self,
        key: &str,
        read: impl Fn(&Node<'_>) -> Result<Located<T>, Diagnostic>,
    ) -> Sourced<T> {
        self.asked.insert(key.to_owned());
        let sourced = match self.read_project(key, &read) {
            Some(located) => Sourced::from_file(located),
            None => Sourced::builtin(self.read_default(key, &read)),
        };
        self.record(key, &sourced);
        sourced
    }

    /// A field with no built-in answer — absent unless the project sets it.
    fn optional<T>(
        &mut self,
        key: &str,
        read: impl Fn(&Node<'_>) -> Result<Located<T>, Diagnostic>,
    ) -> Option<Sourced<T>> {
        self.asked.insert(key.to_owned());
        // The defaults file may still carry one, so a future release can give
        // an optional field a value without changing this code.
        let sourced = match self.read_project(key, &read) {
            Some(located) => Sourced::from_file(located),
            None => {
                let node = self.defaults.find(key)?;
                Sourced::builtin(self.must_read(key, &node, &read))
            }
        };
        self.record(key, &sourced);
        Some(sourced)
    }

    /// An unset optional field has no origin, because it has no value —
    /// nothing chose it, and `Builtin` would be a claim that something did.
    fn record<T>(&mut self, key: &str, sourced: &Sourced<T>) {
        self.provenance.record(key, sourced.origin().clone());
    }

    /// A list is read separately from a single value, because one bad element
    /// is not one bad setting: `order = ["MAT", 3, "JHN"]` still says
    /// something about Matthew and John, and DIA-002 wants every bad element
    /// named rather than the first.
    fn list(&mut self, key: &str) -> Sourced<Vec<String>> {
        self.asked.insert(key.to_owned());
        let sourced = match self.project_list(key) {
            Some(sourced) => sourced,
            None => Sourced::builtin(self.read_default(key, &read_strings)),
        };
        self.record(key, &sourced);
        sourced
    }

    fn optional_list(&mut self, key: &str) -> Option<Sourced<Vec<String>>> {
        self.asked.insert(key.to_owned());
        let sourced = match self.project_list(key) {
            Some(sourced) => sourced,
            None => {
                let node = self.defaults.find(key)?;
                Sourced::builtin(self.must_read(key, &node, &read_strings))
            }
        };
        self.record(key, &sourced);
        Some(sourced)
    }

    fn project_list(&mut self, key: &str) -> Option<Sourced<Vec<String>>> {
        let node = self.project?.find(key)?;

        // Not an array at all — `order = "MAT"` — is one problem with the
        // setting, so the built-in list stands.
        let elements = match node.array() {
            Ok(elements) => elements,
            Err(d) => {
                self.diagnostics.push(d);
                return None;
            }
        };

        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            match element.string() {
                Ok(s) => values.push(s.value),
                Err(d) => self.diagnostics.push(d),
            }
        }

        Some(Sourced::from_file(Located {
            value: values,
            loc: node.loc(),
        }))
    }

    fn read_project<T>(
        &mut self,
        key: &str,
        read: &impl Fn(&Node<'_>) -> Result<Located<T>, Diagnostic>,
    ) -> Option<Located<T>> {
        let node = self.project?.find(key)?;
        match read(&node) {
            Ok(located) => Some(located),
            Err(d) => {
                // Reported, and then the built-in value is used. One bad key
                // costs its own setting and nothing else (CFG-002).
                self.diagnostics.push(d);
                None
            }
        }
    }

    fn read_default<T>(
        &mut self,
        key: &str,
        read: &impl Fn(&Node<'_>) -> Result<Located<T>, Diagnostic>,
    ) -> T {
        let node = self
            .defaults
            .find(key)
            .unwrap_or_else(|| panic!("the embedded defaults have no `{key}`"));
        self.must_read(key, &node, read)
    }

    fn must_read<T>(
        &self,
        key: &str,
        node: &Node<'_>,
        read: &impl Fn(&Node<'_>) -> Result<Located<T>, Diagnostic>,
    ) -> T {
        read(node)
            .unwrap_or_else(|d| panic!("the embedded default for `{key}` is invalid: {d}"))
            .value
    }
}

/// A list of strings, all or nothing.
///
/// Used for the embedded defaults only — [`Resolver::project_list`] is the
/// forgiving path. A default list with a bad element is our bug, and the right
/// response to it is the panic in [`Resolver::must_read`] during our own tests.
fn read_strings(node: &Node<'_>) -> Result<Located<Vec<String>>, Diagnostic> {
    let elements = node.array()?;
    let loc = node.loc();
    let mut values = Vec::with_capacity(elements.len());
    for element in elements {
        values.push(element.string()?.value);
    }
    Ok(Located { value: values, loc })
}
