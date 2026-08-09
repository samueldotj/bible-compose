//! One parse of a TOML file, read typed and kept editable.
//!
//! ARCHITECTURE §6 says the format-preserving document is the *only* parse and
//! the typed view is derived from it. That is a structural claim, not a
//! stylistic one, and it buys four things at once:
//!
//! * **CFG-003** — a syntax error has a line and a column, because the parser
//!   that found it is the parser whose spans we keep.
//! * **CFG-004** — an unknown key is reported *where it was written*, because
//!   the key is still in the tree with its span attached.
//! * **CFG-006** — write-back preserves comments and ordering, because the
//!   thing being written is the thing that was read.
//! * **[ADR-005]** — every resolved value can say where it came from without a
//!   second pass, because [`Located`] falls out of the read.
//!
//! The alternative — `toml_edit` for writing and `serde` for reading — is one
//! file parsed twice by two code paths that can disagree about it. They
//! disagree rarely, and when they do the symptom is a setting that the
//! inspector attributes to the file and the build ignores. So the `serde`
//! feature of `toml_edit` is switched off in the manifest rather than merely
//! unused, and a test in the test kit holds that line.
//!
//! # Reading is a different type from writing
//!
//! [`ConfigDocument`] wraps `toml_edit::Document`, which retains the source
//! text and therefore the spans. Mutation drops spans, so the editable form is
//! reached by [`ConfigDocument::into_editable`], which consumes the reader.
//! A location taken from a document that has since been edited is therefore
//! not expressible, rather than merely discouraged.
//!
//! [ADR-005]: ../../../docs/adr/005-provenance.md

use std::fmt;
use std::ops::{Deref, Range};

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};
use toml_edit::{Document, DocumentMut, Item, TableLike, Value};

/// Byte offsets of the start of every line, for turning a span into a
/// position.
///
/// Columns are counted in **bytes**, matching `usfm-core`'s `LineCol`. One
/// convention across the whole diagnostics panel is worth more than the
/// marginal accuracy of counting characters, and in a TOML file the text
/// before a reported key or value is very nearly always ASCII, where the two
/// agree.
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut starts = Vec::with_capacity(text.len() / 32 + 1);
        starts.push(0);
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    /// 1-based line and column for a byte offset.
    fn locate(&self, offset: usize) -> (u32, u32) {
        // `starts` is sorted and begins at 0, so this never underflows.
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let col = offset - self.starts[line];
        (line as u32 + 1, col as u32 + 1)
    }
}

/// A parsed configuration file: the document, its source, and its path.
pub struct ConfigDocument {
    path: Utf8PathBuf,
    doc: Document<String>,
    lines: LineIndex,
}

impl fmt::Debug for ConfigDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The whole source in a failed `assert_eq!` is noise; the path is the
        // thing a reader needs to know which file this is.
        f.debug_struct("ConfigDocument")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl ConfigDocument {
    /// Parse, or produce the one diagnostic CFG-003 asks for.
    ///
    /// A syntax error is an error rather than a warning: SRS CFG-003 requires
    /// that malformed TOML cannot silently fall back to defaults, and a
    /// half-read settings file is a build whose output nobody can explain.
    pub fn parse(path: impl Into<Utf8PathBuf>, source: String) -> Result<Self, Diagnostic> {
        let path = path.into();
        let lines = LineIndex::new(&source);

        match Document::parse(source) {
            Ok(doc) => Ok(ConfigDocument { path, doc, lines }),
            Err(err) => {
                // `message` is the bare cause; the `Display` form is the cause
                // plus the offending line with a caret under it. The rendered
                // form goes in `detail` — DIA-005's collapsed-by-default field
                // is exactly right for verbatim tool output, and it is more
                // use here than a one-line quote, because a TOML syntax error
                // is usually about a *position* in the line.
                let mut d = Diagnostic::error(code::INVALID_TOML, err.message().to_owned())
                    .help("the file must be valid TOML before any setting in it can be used")
                    .detail(err.to_string());

                // The location is recomputed from the span rather than scraped
                // out of that rendering, so the panel and the GUI get a
                // structured line and column to jump to.
                d = match err.span() {
                    Some(span) => {
                        let (line, col) = lines.locate(span.start);
                        d.at(SourceLoc::at(path.clone(), line, col))
                    }
                    None => d.at(SourceLoc::file(path.clone())),
                };

                Err(d)
            }
        }
    }

    /// Read a file from disk, reporting an unreadable file as a diagnostic too.
    pub fn read(path: impl Into<Utf8PathBuf>) -> Result<Self, Diagnostic> {
        let path = path.into();
        let source = std::fs::read_to_string(path.as_std_path()).map_err(|e| {
            Diagnostic::error(code::INVALID_TOML, format!("cannot read the file: {e}"))
                .at(SourceLoc::file(path.clone()))
        })?;
        Self::parse(path, source)
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn source(&self) -> &str {
        self.doc.raw()
    }

    /// The whole file, as a node.
    pub fn root(&self) -> Node<'_> {
        Node {
            doc: self,
            path: String::new(),
            slot: Slot::Table(self.doc.as_table()),
            key_span: None,
        }
    }

    /// A node by dotted path — `"page.width"`. `None` if any step is absent.
    ///
    /// Convenience for tests and for one-off reads. The schema walk in P2.3
    /// descends a level at a time instead, because it also has to notice the
    /// keys it did *not* ask for.
    pub fn find(&self, dotted: &str) -> Option<Node<'_>> {
        let mut node = self.root();
        for step in dotted.split('.') {
            node = node.table().ok()?.get(step)?;
        }
        Some(node)
    }

    /// The editable form, for write-back (P2.7).
    ///
    /// Consumes the reader: mutation invalidates every span, so a `SourceLoc`
    /// derived from this document and a document that has since been edited
    /// cannot coexist.
    pub fn into_editable(self) -> DocumentMut {
        self.doc.into_mut()
    }

    fn loc(&self, span: Option<Range<usize>>) -> SourceLoc {
        match span {
            Some(span) => {
                let (line, col) = self.lines.locate(span.start);
                SourceLoc::at(self.path.clone(), line, col)
            }
            // ADR-005: where the position is genuinely unknown, say so rather
            // than fabricate `file:0:0`, which reads as a bug in the file.
            None => SourceLoc::file(self.path.clone()),
        }
    }
}

/// A value paired with where it was written.
///
/// The P2.2 half of [ADR-005]'s `Sourced<T>`: this says *where in this file*,
/// and P2.6's `Origin` adds the cases this cannot express — a built-in default
/// and a value inherited from another style. Every typed read returns one, so
/// threading provenance through resolution is a matter of not throwing it
/// away rather than of remembering to look it up.
///
/// [ADR-005]: ../../../docs/adr/005-provenance.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    pub value: T,
    pub loc: SourceLoc,
}

impl<T> Located<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Located<U> {
        Located {
            value: f(self.value),
            loc: self.loc,
        }
    }
}

/// Reading `page.width` without caring where it came from is the common case.
impl<T> Deref for Located<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

/// What a node is pointing at.
///
/// Two cases rather than one because array elements are bare `Value`s with no
/// enclosing `Item`, and `[page]` and `page = {}` are different types that
/// must behave identically to a reader — the second is what `TableLike`
/// unifies.
#[derive(Clone, Copy)]
enum Slot<'a> {
    Item(&'a Item),
    Value(&'a Value),
    Table(&'a dyn TableLike),
}

/// A place in the document, with the path that reached it.
///
/// The path is owned rather than borrowed: a dotted path is assembled during
/// descent and there is nothing in the document that holds the whole of it.
/// A settings file has hundreds of keys, so the allocation is not worth an
/// arena to avoid.
#[derive(Clone)]
pub struct Node<'a> {
    doc: &'a ConfigDocument,
    path: String,
    slot: Slot<'a>,
    key_span: Option<Range<usize>>,
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({} at {})", self.dotted_path(), self.loc())
    }
}

impl<'a> Node<'a> {
    /// The dotted path that reached this node — `"page.width"`, or `""` for
    /// the file itself.
    pub fn dotted_path(&self) -> &str {
        &self.path
    }

    /// Where this node is, preferring its key over its value.
    ///
    /// The key is what a person searches the file for, and for a table it is
    /// the header line rather than the whole multi-line span. Array elements
    /// and the root have no key, so those fall back to the value.
    pub fn loc(&self) -> SourceLoc {
        self.doc
            .loc(self.key_span.clone().or_else(|| self.value_span()))
    }

    fn value_span(&self) -> Option<Range<usize>> {
        match self.slot {
            Slot::Item(i) => i.span(),
            Slot::Value(v) => v.span(),
            // The root table's span is the whole file, which as a
            // position is worse than none at all.
            Slot::Table(_) => None,
        }
    }

    fn as_value(&self) -> Option<&'a Value> {
        match self.slot {
            Slot::Item(i) => i.as_value(),
            Slot::Value(v) => Some(v),
            Slot::Table(_) => None,
        }
    }

    /// What kind of thing this is, in the words a TOML author would use.
    ///
    /// Exhaustive over both enums rather than falling through to a catch-all,
    /// because a wrong-type diagnostic that says "empty" about a table sends
    /// the reader looking for the wrong problem.
    pub fn type_name(&self) -> &'static str {
        fn of(value: &Value) -> &'static str {
            match value {
                Value::String(_) => "a string",
                Value::Integer(_) => "an integer",
                Value::Float(_) => "a decimal number",
                Value::Boolean(_) => "a boolean",
                Value::Datetime(_) => "a date",
                Value::Array(_) => "an array",
                // `margin = { inner = … }` is a table, and calling it an
                // inline table would be telling an author about a
                // distinction this layer deliberately does not make.
                Value::InlineTable(_) => "a table",
            }
        }

        match self.slot {
            Slot::Table(_) | Slot::Item(Item::Table(_)) => "a table",
            Slot::Value(v) | Slot::Item(Item::Value(v)) => of(v),
            Slot::Item(Item::ArrayOfTables(_)) => "an array of tables",
            Slot::Item(Item::None) => "empty",
        }
    }

    fn wrong_type(&self, expected: &str) -> Diagnostic {
        Diagnostic::error(
            code::WRONG_TYPE,
            format!(
                "{} is {}; expected {expected}",
                self.describe(),
                self.type_name()
            ),
        )
        .at(self.loc())
    }

    fn describe(&self) -> String {
        if self.path.is_empty() {
            "the file".to_owned()
        } else {
            format!("`{}`", self.path)
        }
    }

    pub fn string(&self) -> Result<Located<String>, Diagnostic> {
        self.as_value()
            .and_then(Value::as_str)
            .map(|s| Located {
                value: s.to_owned(),
                loc: self.loc(),
            })
            .ok_or_else(|| self.wrong_type("a string"))
    }

    pub fn integer(&self) -> Result<Located<i64>, Diagnostic> {
        self.as_value()
            .and_then(Value::as_integer)
            .map(|value| Located {
                value,
                loc: self.loc(),
            })
            .ok_or_else(|| self.wrong_type("an integer"))
    }

    /// An integer is accepted where a decimal is expected, because `margin =
    /// 1` is what a person writes and rejecting it would be pedantry with a
    /// diagnostic attached.
    pub fn number(&self) -> Result<Located<f64>, Diagnostic> {
        let v = self.as_value();
        v.and_then(Value::as_float)
            .or_else(|| v.and_then(Value::as_integer).map(|i| i as f64))
            .map(|value| Located {
                value,
                loc: self.loc(),
            })
            .ok_or_else(|| self.wrong_type("a number"))
    }

    pub fn boolean(&self) -> Result<Located<bool>, Diagnostic> {
        self.as_value()
            .and_then(Value::as_bool)
            .map(|value| Located {
                value,
                loc: self.loc(),
            })
            .ok_or_else(|| self.wrong_type("true or false"))
    }

    /// The elements, each a node in its own right so a bad one is reported at
    /// its own position rather than at the array's.
    pub fn array(&self) -> Result<Vec<Node<'a>>, Diagnostic> {
        let array = self
            .as_value()
            .and_then(Value::as_array)
            .ok_or_else(|| self.wrong_type("an array"))?;

        Ok(array
            .iter()
            .enumerate()
            .map(|(i, value)| Node {
                doc: self.doc,
                path: format!("{}[{i}]", self.path),
                slot: Slot::Value(value),
                key_span: None,
            })
            .collect())
    }

    /// Every element as a string, collecting the failures rather than
    /// stopping at the first — DIA-002 wants the whole list of what is wrong.
    pub fn string_array(&self) -> (Vec<Located<String>>, Vec<Diagnostic>) {
        match self.array() {
            Err(d) => (Vec::new(), vec![d]),
            Ok(nodes) => {
                let (mut values, mut errors) = (Vec::new(), Vec::new());
                for node in nodes {
                    match node.string() {
                        Ok(v) => values.push(v),
                        Err(d) => errors.push(d),
                    }
                }
                (values, errors)
            }
        }
    }

    /// This node as a table. `[page]` and `page = { … }` are the same thing
    /// here, which is what stops the schema caring which one an author wrote.
    pub fn table(&self) -> Result<Table<'a>, Diagnostic> {
        let inner = match self.slot {
            Slot::Table(t) => Some(t),
            Slot::Item(i) => i.as_table_like(),
            Slot::Value(v) => v.as_inline_table().map(|t| t as &dyn TableLike),
        }
        .ok_or_else(|| self.wrong_type("a table"))?;

        Ok(Table {
            doc: self.doc,
            path: self.path.clone(),
            inner,
        })
    }
}

/// A table, and the reads a schema walk needs of one.
#[derive(Clone)]
pub struct Table<'a> {
    doc: &'a ConfigDocument,
    path: String,
    inner: &'a dyn TableLike,
}

impl fmt::Debug for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Table({}, {} keys)", self.path, self.inner.len())
    }
}

impl<'a> Table<'a> {
    pub fn get(&self, key: &str) -> Option<Node<'a>> {
        let (k, item) = self.inner.get_key_value(key)?;
        Some(Node {
            doc: self.doc,
            path: join(&self.path, key),
            slot: Slot::Item(item),
            key_span: k.span(),
        })
    }

    /// The keys actually present, in file order.
    ///
    /// File order rather than sorted, because CFG-004's unknown-key warnings
    /// read best in the order a person would scroll past them. `toml_edit`
    /// preserves it, which is one more thing the single parse gives us.
    pub fn names(&self) -> Vec<&'a str> {
        self.inner.iter().map(|(k, _)| k).collect()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}
