//! Fixtures, golden helpers and PDF assertions.
//!
//! Deliberately free of any dependency on `biblecompose-sile`: it is consumed
//! by that crate's own tests, and depending back would break ADR-004's rule
//! and create a cycle.

pub mod golden;
pub mod pdf;

use camino::Utf8PathBuf;

/// The repository root, found by walking up from this crate.
///
/// Golden files live in the repository rather than in a temporary directory,
/// because the point of them is to be reviewed in a diff.
pub fn repo_root() -> Utf8PathBuf {
    let here = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .and_then(camino::Utf8Path::parent)
        .expect("crates/<name> is always two levels below the root")
        .to_owned()
}

pub fn golden_dir() -> Utf8PathBuf {
    repo_root().join("tests").join("golden")
}
