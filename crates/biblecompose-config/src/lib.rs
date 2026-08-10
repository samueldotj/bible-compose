//! Settings and styles: defaults, cascade, provenance, validation.
//!
//! [`document`] is the layer everything else in this crate stands on — one
//! `toml_edit` parse per file, read typed and kept editable, so the values a
//! build uses and the file a GUI writes back cannot come apart
//! (ARCHITECTURE §6). The schema, the merge with embedded defaults, and unit
//! parsing arrive at P2.3 onward and all read through it.

pub mod cascade;
pub mod document;
pub mod edit;
pub mod form;
pub mod provenance;
pub mod selector;
pub mod settings;
pub mod style;
pub mod value;

pub use cascade::{ResolvedStyle, ResolvedStyles};
pub use document::{ConfigDocument, Located, Node, Table};
pub use edit::{set_validated, SettingValue, TomlFile};
pub use form::{Field, Kind};
pub use provenance::{Origin, Provenance, Sourced};
pub use selector::StyleSelector;
pub use settings::{known_keys, resolve, Settings, SCHEMA_VERSION};
pub use style::{Align, Style, StyleSheet};
pub use value::{Length, PageSize, Unit};
