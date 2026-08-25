//! The SILE runtime, carried inside the application executable.
//!
//! [ADR-006] option C. Everything interesting is in [`crate::cache`]; this is
//! the part that knows about `rust-embed`, and it is thin on purpose so that
//! the logic stays testable without an actual 78 MB bundle.
//!
//! # Building with a bundle
//!
//! ```text
//! BIBLECOMPOSE_SILE_BUNDLE=/path/to/stage cargo build --features embedded-sile
//! ```
//!
//! The stage directory is what [`spike/s1-windows-cross.sh`] produces: `sile`
//! (or `sile.exe`) at the top, SILE's Lua tree beside it, `lua_modules/` with
//! the rocks, and on Windows the DLLs. Without the variable the feature
//! compiles against a placeholder and [`ensure`] reports that it is not a real
//! bundle, so a misconfigured release build fails with a sentence rather than
//! with a missing file at a customer site.
//!
//! [ADR-006]: ../../../docs/adr/006-single-binary.md
//! [`spike/s1-windows-cross.sh`]: ../../../spike/s1-windows-cross.sh

use biblecompose_diagnostics::{code, Diagnostic};
use camino::Utf8PathBuf;
use rust_embed::Embed;

use crate::cache::{cache_key, default_cache_root, ensure_extracted, Entry};

/// The bundle. `$BIBLECOMPOSE_SILE_BUNDLE` is resolved at compile time; see the
/// module docs and `build.rs`.
#[derive(Embed)]
#[folder = "$BIBLECOMPOSE_SILE_BUNDLE"]
struct Bundle;

/// Present in the placeholder directory and in no real bundle.
const PLACEHOLDER_MARKER: &str = "PLACEHOLDER";

/// What the executable is called inside the bundle.
pub const EXE_NAME: &str = if cfg!(windows) { "sile.exe" } else { "sile" };

/// Unpack the bundled runtime if needed and return the executable's path.
///
/// Cheap on every run after the first: the cache directory is named after the
/// bundle's contents, so "is it already there" is one `exists()`.
pub fn ensure() -> Result<Utf8PathBuf, Diagnostic> {
    if Bundle::get(PLACEHOLDER_MARKER).is_some() {
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            "this build was compiled with the embedded backend but without a backend to embed",
        )
        .help(
            "rebuild with BIBLECOMPOSE_SILE_BUNDLE pointing at a runtime stage, \
             or drop the embedded-sile feature",
        ));
    }

    let mut entries: Vec<Entry> = Bundle::iter()
        .filter_map(|path| {
            Bundle::get(&path).map(|f| Entry {
                path: path.into_owned(),
                hash: f.metadata.sha256_hash(),
            })
        })
        .collect();

    if entries.is_empty() {
        return Err(Diagnostic::error(
            code::NOT_FOUND,
            "the embedded typesetting runtime is empty",
        )
        .help("BIBLECOMPOSE_SILE_BUNDLE pointed at a directory with no files in it"));
    }

    // **A shipped runtime has no networking code in it, and this is where
    // that stops being a promise** (NFR-004, spike F-16). `luasec` and
    // `luasocket` come with SILE's standard rock set; `scripts/stage-backend.mjs`
    // removes them and CI checks the stage. This is the last line of that
    // defence and the only one that survives a release built by hand: the
    // entries are already being walked to compute the cache key, so it costs
    // a string comparison each and refuses rather than warns, because a
    // warning about a property of the *binary* has nobody left to act on it.
    if let Some(found) = entries.iter().find(|e| is_networking(&e.path)) {
        return Err(Diagnostic::error(
            code::BUNDLE_UNPACK_FAILED,
            format!(
                "the embedded typesetting runtime contains networking code ({}), \
                 which this application does not ship",
                found.path
            ),
        )
        .help(
            "the bundle was staged without scripts/stage-backend.mjs, or with an \
             older copy of it — restage and rebuild",
        ));
    }

    let key = cache_key(&mut entries);

    let root = default_cache_root().ok_or_else(|| {
        Diagnostic::error(
            code::BUNDLE_UNPACK_FAILED,
            "could not work out where to unpack the typesetting runtime",
        )
        .help("neither XDG_CACHE_HOME nor HOME (nor LOCALAPPDATA on Windows) is set")
    })?;

    let dir = ensure_extracted(&root, &key, EXE_NAME, || {
        Bundle::iter().filter_map(|path| Bundle::get(&path).map(|f| (path.into_owned(), f.data)))
    })?;

    Ok(dir.join(EXE_NAME))
}

/// Whether this path inside a bundle belongs to `luasec` or `luasocket`.
///
/// The same names `scripts/stage-backend.mjs` removes, written out again here
/// rather than shared — the script runs in Node at packaging time and this
/// runs in Rust at start-up, and a list duplicated in two languages with a
/// test on both sides is better than a third file neither of them owns.
fn is_networking(path: &str) -> bool {
    // Directory separators inside a `rust-embed` path are always `/`.
    path.split('/').any(|part| {
        matches!(
            part,
            "socket" | "ssl" | "mime" | "ltn12.lua" | "socket.lua" | "ssl.lua" | "mime.lua"
        ) || matches!(
            part,
            "socket.so" | "ssl.so" | "mime.so" | "socket.dll" | "ssl.dll" | "mime.dll"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the placeholder: a build that forgot the bundle says
    /// so, in one sentence, instead of failing later with a missing file.
    ///
    /// This runs against the placeholder because CI has no real runtime to
    /// embed. A release build is verified by P5.7's smoke test instead — see
    /// [S1-NOTES](../../../spike/S1-NOTES.md).
    #[test]
    fn a_build_with_nothing_embedded_says_so() {
        let err = ensure().expect_err("the placeholder must not pass for a runtime");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(
            err.help
                .as_deref()
                .unwrap_or_default()
                .contains("BIBLECOMPOSE_SILE_BUNDLE"),
            "the message should name the variable to set: {err:?}"
        );
    }

    /// The paths that must not be in a shipped bundle, and the ones that are
    /// fine — `cliargs/utils/disect.lua` contains "sect" and a rule matching
    /// substrings rather than path segments would take it out.
    #[test]
    fn networking_is_recognised_by_path_segment() {
        for path in [
            "lua_modules/share/lua/5.1/ssl.lua",
            "lua_modules/share/lua/5.1/socket.lua",
            "lua_modules/share/lua/5.1/socket/http.lua",
            "lua_modules/share/lua/5.1/ssl/https.lua",
            "lua_modules/share/lua/5.1/ltn12.lua",
            "lua_modules/lib/lua/5.1/socket/core.dll",
        ] {
            assert!(is_networking(path), "{path} should be refused");
        }
        for path in [
            "classes/biblecompose.lua",
            "packages/insertions/init.lua",
            "lua_modules/share/lua/5.1/cliargs/utils/disect.lua",
            "core/socketry.lua",
            "languages/mimetype-ish.lua",
        ] {
            assert!(!is_networking(path), "{path} should be allowed");
        }
    }

    #[test]
    fn the_executable_name_matches_the_platform() {
        assert_eq!(EXE_NAME.ends_with(".exe"), cfg!(windows));
    }
}
