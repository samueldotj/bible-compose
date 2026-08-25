//! Fixtures, golden helpers and PDF assertions.
//!
//! Deliberately free of any dependency on `biblecompose-sile`: it is consumed
//! by that crate's own tests, and depending back would break ADR-004's rule
//! and create a cycle.

pub mod corpus;
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

/// A 1×1 greyscale PNG, valid enough for a decoder rather than only for a
/// signature check.
///
/// Real bytes and not the eight-byte header the asset pre-flight would accept:
/// a fixture that only looks like a PNG to the code reading it is a trap for
/// the first test that hands it to a backend.
pub const PIXEL_PNG: [u8; 67] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3a, 0x7e, 0x9b,
    0x55, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x68, 0x00, 0x00, 0x00,
    0x82, 0x00, 0x81, 0x77, 0xcd, 0x72, 0xb6, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// Put the artwork the built-in fixtures name into a project folder.
///
/// A fixture is a document, and a document has no folder — so the file
/// `kitchen_sink`'s figure refers to exists nowhere until a test makes it.
/// Since P4.3 that matters twice over: the application refuses to build a
/// project whose figure is absent, and the class no longer swallows a draw
/// that fails. Both are the point; neither should make an unrelated test fail
/// for a reason it is not about.
pub fn place_fixture_assets(root: &camino::Utf8Path) {
    let art = root.join("assets/images");
    std::fs::create_dir_all(art.as_std_path()).expect("a temp directory is writable");
    std::fs::write(art.join("map.png").as_std_path(), PIXEL_PNG).expect("write the fixture PNG");
}

pub fn golden_dir() -> Utf8PathBuf {
    repo_root().join("tests").join("golden")
}
