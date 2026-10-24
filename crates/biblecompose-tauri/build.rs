//! Give `rust-embed` somewhere to point when no editor was supplied.
//!
//! The same arrangement, and the same two reasons, as the backend bundle next
//! door in `biblecompose-sile/build.rs`: `#[folder = "$..."]` is resolved when
//! the derive runs and a missing directory is a compile error, so the feature
//! would not type-check in CI without an editor in the repository. And the
//! staged directory is watched file by file rather than by name, because a
//! stage is edited far more often than it is renamed and cargo tracks a
//! directory's own mtime, which does not change when a file inside it does.
//! That defect shipped the wrong typesetter once; it is not worth meeting
//! twice.

use std::path::Path;

fn main() {
    // First, and not optional: this is what generates the Tauri context the
    // shell is built around.
    tauri_build::build();

    println!("cargo:rerun-if-env-changed=BIBLECOMPOSE_EDITOR_BUNDLE");

    match std::env::var_os("BIBLECOMPOSE_EDITOR_BUNDLE") {
        Some(bundle) => watch(Path::new(&bundle)),
        None => {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets this");
            println!("cargo:rustc-env=BIBLECOMPOSE_EDITOR_BUNDLE={manifest}/editor-placeholder");
        }
    }
}

fn watch(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
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
