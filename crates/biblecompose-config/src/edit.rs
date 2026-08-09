//! Changing a settings file without disturbing the rest of it
//! (CFG-005 – CFG-007).
//!
//! The whole reason `toml_edit` is the only parse is here. What gets written
//! back is the document that was read, with one value replaced, so a comment a
//! publisher wrote about why their margins are what they are survives a GUI
//! edit to the font — and their key order, their alignment, and their blank
//! lines survive with it.
//!
//! # Reset is a removal, not a write
//!
//! CFG-007 asks that a setting can go back to inherited behaviour. That is
//! [`SettingsFile::reset`], which *deletes* the key: writing the built-in value
//! into the file would look identical today and diverge silently the first time
//! a release changes a default. A file that does not mention `page.size` gets
//! whatever this release thinks a page is; a file that says `"6x9in"` is
//! pinned to 6×9in for ever, and the publisher who clicked "reset" did not ask
//! for that.

use biblecompose_diagnostics::{code, Diagnostic, Diagnostics, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::document::ConfigDocument;
use crate::value::Length;

/// A value a settings key can be set to.
///
/// A small enum rather than `toml_edit::Value`, so `toml_edit` stays an
/// implementation detail of this crate — a GUI that had to build a
/// `toml_edit::Value` would be a GUI that depends on the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
}

impl From<&str> for SettingValue {
    fn from(s: &str) -> Self {
        SettingValue::Str(s.to_owned())
    }
}

impl From<String> for SettingValue {
    fn from(s: String) -> Self {
        SettingValue::Str(s)
    }
}

impl From<bool> for SettingValue {
    fn from(b: bool) -> Self {
        SettingValue::Bool(b)
    }
}

impl From<i64> for SettingValue {
    fn from(i: i64) -> Self {
        SettingValue::Int(i)
    }
}

/// A length is written back in the unit it is carried in, not in points.
/// `0.55in` is what a publisher recognises as their margin; `39.6pt` is the
/// same measurement and a worse thing to find in your own file.
impl From<Length> for SettingValue {
    fn from(l: Length) -> Self {
        SettingValue::Str(l.to_string())
    }
}

impl SettingValue {
    fn into_toml(self) -> Value {
        match self {
            SettingValue::Str(s) => Value::from(s),
            SettingValue::Int(i) => Value::from(i),
            SettingValue::Float(f) => Value::from(f),
            SettingValue::Bool(b) => Value::from(b),
            SettingValue::List(items) => {
                let mut array = toml_edit::Array::new();
                for item in items {
                    array.push(item);
                }
                Value::Array(array)
            }
        }
    }
}

/// A settings file open for editing.
///
/// Made from a [`ConfigDocument`], which it consumes: reading gives spans and
/// editing destroys them, and the two are different types so a location taken
/// before an edit cannot be used after one.
#[derive(Clone)]
pub struct SettingsFile {
    path: Utf8PathBuf,
    doc: DocumentMut,
}

impl SettingsFile {
    pub fn new(doc: ConfigDocument) -> Self {
        let path = doc.path().to_owned();
        SettingsFile {
            path,
            doc: doc.into_editable(),
        }
    }

    /// An empty file, for a project that has no settings yet.
    ///
    /// The header is the only thing written that nobody asked for, and it is
    /// there because a bare `[page]` appearing in a folder is a mystery
    /// otherwise.
    pub fn create(path: impl Into<Utf8PathBuf>, schema_version: i64) -> Self {
        let source = format!(
            "# BibleCompose settings. Anything not set here uses the built-in\n\
             # default, so an empty file and no file mean the same thing.\n\
             schema_version = {schema_version}\n"
        );
        let path = path.into();
        let doc = ConfigDocument::parse(path.clone(), source)
            .expect("the generated header is valid TOML");
        SettingsFile::new(doc)
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    /// Set one key, creating the tables above it if they are missing.
    ///
    /// A table this creates is a real `[page]` header rather than an inline
    /// one, because that is what the rest of the file looks like.
    pub fn set(&mut self, key: &str, value: impl Into<SettingValue>) {
        let value = value.into().into_toml();
        let (path, leaf) = split(key);

        let mut table = self.doc.as_table_mut();
        for step in path {
            let entry = table
                .entry(step)
                .or_insert_with(|| Item::Table(Table::new()));
            // A key that exists but is not a table — `page = 3` — is replaced
            // rather than descended into. It cannot hold the value being set,
            // and refusing to write would leave a GUI unable to fix a file by
            // editing it, which is the one thing it is for.
            if !entry.is_table_like() {
                *entry = Item::Table(Table::new());
            }
            table = entry
                .as_table_mut()
                .expect("just ensured this is a standard table");
        }

        table[leaf] = Item::Value(value);
    }

    /// Remove one key, so the built-in value applies again (CFG-007).
    ///
    /// Returns whether there was anything to remove, so a GUI can leave the
    /// reset control disabled for a value that is already the default.
    ///
    /// An emptied table is left behind. `[page]` with nothing under it is
    /// inert, and a publisher who wrote that header did not ask for it to be
    /// deleted because the last key inside it was reset.
    pub fn reset(&mut self, key: &str) -> bool {
        let (path, leaf) = split(key);

        let mut table = self.doc.as_table_mut();
        for step in path {
            match table.get_mut(step).and_then(Item::as_table_mut) {
                Some(next) => table = next,
                None => return false,
            }
        }
        table.remove(leaf).is_some()
    }

    /// The file as it would be written.
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    /// Write it back, replacing the old file in one step.
    ///
    /// Through a temporary file and a rename, for the same reason the PDF is
    /// published that way: a crash halfway through must not leave a settings
    /// file that is half of two versions and valid as neither.
    pub fn save(&self) -> Result<(), Diagnostic> {
        let text = self.to_toml();
        let tmp = self.path.with_extension("toml.saving");

        std::fs::write(tmp.as_std_path(), text.as_bytes()).map_err(|e| self.failed(&tmp, e))?;
        std::fs::rename(tmp.as_std_path(), self.path.as_std_path()).map_err(|e| {
            // The temporary is ours; leaving it behind after a failure is
            // litter in the publisher's project folder.
            let _ = std::fs::remove_file(tmp.as_std_path());
            self.failed(&self.path, e)
        })
    }

    fn failed(&self, path: &Utf8Path, e: std::io::Error) -> Diagnostic {
        Diagnostic::error(
            code::DESTINATION_UNWRITABLE,
            format!("could not save {}", self.path),
        )
        .at(SourceLoc::file(path.to_owned()))
        .detail(e.to_string())
    }
}

/// Set a key only if the file still resolves cleanly afterwards.
///
/// A form field is text, and text can say `"quarto"`. Validating it here would
/// mean a second opinion about what a page size is; instead the edit is made
/// on a copy, the copy is resolved by the same reader that resolves a
/// hand-written file, and the edit is kept only if it introduced no complaint
/// that was not there before.
///
/// "Not there before" rather than "no complaints at all", because a file may
/// already have a problem elsewhere and a publisher must still be able to fix
/// *this* field. On refusal the file is untouched and the new diagnostics are
/// returned — they are what the field shows.
pub fn set_validated(
    file: &mut SettingsFile,
    key: &str,
    value: SettingValue,
) -> Result<(), Diagnostics> {
    let before = complaints(&file.to_toml(), file.path());

    let mut trial = file.clone();
    trial.set(key, value);
    let after = complaints(&trial.to_toml(), file.path());

    let mut fresh = Diagnostics::new();
    for d in after {
        if !before
            .iter()
            .any(|b| b.code == d.code && b.message == d.message)
        {
            fresh.push(d);
        }
    }
    if !fresh.is_empty() {
        return Err(fresh);
    }

    *file = trial;
    Ok(())
}

/// Every diagnostic a settings text produces, including the parse error if it
/// does not parse at all.
fn complaints(toml: &str, path: &Utf8Path) -> Vec<Diagnostic> {
    match ConfigDocument::parse(path.to_owned(), toml.to_owned()) {
        Ok(doc) => crate::settings::resolve(Some(&doc)).1.into_iter().collect(),
        Err(d) => vec![d],
    }
}

/// `"page.margin.inner"` → (`["page", "margin"]`, `"inner"`).
fn split(key: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = key.split('.').collect();
    let leaf = parts.pop().expect("a key has at least one segment");
    (parts, leaf)
}
