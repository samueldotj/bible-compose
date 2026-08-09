//! Give `rust-embed` somewhere to point when no bundle was supplied.
//!
//! `#[folder = "$BIBLECOMPOSE_SILE_BUNDLE"]` is resolved when the derive runs,
//! and a missing directory is a compile error — which would mean
//! `cargo build --features embedded-sile` could not even be type-checked in CI
//! without shipping a 78 MB runtime in the repository.
//!
//! So: default the variable to a placeholder directory. The feature compiles
//! anywhere, and `bundle::ensure()` recognises the placeholder and says so.

fn main() {
    println!("cargo:rerun-if-env-changed=BIBLECOMPOSE_SILE_BUNDLE");

    if std::env::var_os("BIBLECOMPOSE_SILE_BUNDLE").is_none() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets this");
        println!("cargo:rustc-env=BIBLECOMPOSE_SILE_BUNDLE={manifest}/bundle-placeholder");
    }
}
