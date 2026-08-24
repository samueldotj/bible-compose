//! Give `rust-embed` somewhere to point when no bundle was supplied, and make
//! sure a bundle that changed is a bundle that gets rebuilt.
//!
//! `#[folder = "$BIBLECOMPOSE_SILE_BUNDLE"]` is resolved when the derive runs,
//! and a missing directory is a compile error — which would mean
//! `cargo build --features embedded-sile` could not even be type-checked in CI
//! without shipping a 78 MB runtime in the repository.
//!
//! So: default the variable to a placeholder directory. The feature compiles
//! anywhere, and `bundle::ensure()` recognises the placeholder and says so.
//!
//! # Why the directory is watched as well as the variable
//!
//! Because a stage is *edited* far more often than it is renamed. Re-staging a
//! runtime with a corrected class into the same path, and rebuilding, produced
//! a binary carrying the previous stage — cargo had no reason to think
//! anything had changed, `rust-embed` was not re-run, and the only symptom was
//! the application reporting that its class and its code were different
//! versions. A release built that way would ship the wrong typesetter and say
//! nothing at all.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=BIBLECOMPOSE_SILE_BUNDLE");

    match std::env::var_os("BIBLECOMPOSE_SILE_BUNDLE") {
        Some(bundle) => watch(Path::new(&bundle)),
        None => {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets this");
            println!("cargo:rustc-env=BIBLECOMPOSE_SILE_BUNDLE={manifest}/bundle-placeholder");
        }
    }
}

/// Every file under the stage, so any edit to any of them forces the embed to
/// run again.
///
/// Cargo re-reads these on every build, so this is a directory walk of the
/// bundle per build — a few thousand `stat` calls against a compile that takes
/// half a minute and produces an 84 MB executable. Naming the directory alone
/// would be cheaper and does not work: cargo tracks a directory's mtime, which
/// on most filesystems does not change when a file two levels down does.
fn watch(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        // A stage that is not there is `bundle::ensure`'s problem to report at
        // runtime, with a sentence. A build script has nowhere good to say it.
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            watch(&path);
        }
    }
}
