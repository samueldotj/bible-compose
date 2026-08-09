//! Unpacking an embedded runtime into a cache directory, once.
//!
//! [ADR-006] option C: the application carries SILE inside it and extracts it
//! the first time it is needed. This module is that mechanism, with no
//! knowledge of `rust-embed` — so it is testable without a real bundle, which
//! is 78 MB on Windows and not something a unit test should need.
//!
//! Three properties the spike showed are worth having:
//!
//! - **Content-addressed.** The directory name is derived from what is in it,
//!   so upgrading the application cannot half-reuse the previous runtime, and
//!   two versions can coexist during an upgrade.
//! - **Atomic.** Files are written to a private temporary directory and moved
//!   into place with one rename, so a crash mid-extraction leaves no directory
//!   that looks complete. The same reasoning as publishing a PDF (BLD-009).
//! - **Concurrency-safe by losing gracefully.** Two processes extracting at
//!   once both succeed; the loser discards its copy and uses the winner's,
//!   because the content is identical by construction.
//!
//! [ADR-006]: ../../../docs/adr/006-single-binary.md

use std::borrow::Cow;

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};

/// One file in the bundle: a bundle-relative path and its content hash.
///
/// The hash comes from the embedder rather than being computed here — every
/// embedding scheme worth using already has one, and rehashing 78 MB on each
/// startup to learn something we were told is a poor trade.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub path: String,
    pub hash: [u8; 32],
}

/// A stable name for a set of files, from their paths and hashes.
///
/// FNV-1a rather than a cryptographic digest, deliberately: this names a cache
/// directory, it does not authenticate anything. The bundle's integrity comes
/// from being inside a signed executable (P6.1), not from this.
pub fn cache_key(entries: &mut [Entry]) -> String {
    entries.sort();

    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for e in entries.iter() {
        eat(e.path.as_bytes());
        eat(&[0]);
        eat(&e.hash);
        eat(&[0]);
    }
    format!("{h:016x}")
}

/// Where extracted runtimes live when the caller has no opinion.
///
/// Resolved by hand rather than through a crate: it is fifteen lines, and a
/// dependency that reads three environment variables is a dependency to keep
/// current for no reason.
pub fn default_cache_root() -> Option<Utf8PathBuf> {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());

    let base = if cfg!(windows) {
        var("LOCALAPPDATA").map(Utf8PathBuf::from)
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| Utf8PathBuf::from(h).join("Library/Caches"))
    } else {
        var("XDG_CACHE_HOME")
            .map(Utf8PathBuf::from)
            .or_else(|| var("HOME").map(|h| Utf8PathBuf::from(h).join(".cache")))
    }?;

    Some(base.join("biblecompose").join("sile"))
}

/// Make sure the runtime named by `key` is present under `root`, and return the
/// directory holding it.
///
/// `files` is consulted only when extraction is actually needed, so the warm
/// path does not touch the bundle at all.
///
/// `exe_rel` is the bundle-relative path of the executable. It is what gets the
/// execute bit on Unix, and its presence is what "already extracted" means —
/// checking the directory exists would accept a directory some other tool made.
pub fn ensure_extracted<I, F>(
    root: &Utf8Path,
    key: &str,
    exe_rel: &str,
    files: F,
) -> Result<Utf8PathBuf, Diagnostic>
where
    F: FnOnce() -> I,
    I: IntoIterator<Item = (String, Cow<'static, [u8]>)>,
{
    let target = root.join(key);
    if target.join(exe_rel).exists() {
        return Ok(target);
    }

    // Private to this process, so two concurrent extractions cannot interleave
    // their writes into one directory.
    let tmp = root.join(format!(".{key}.tmp.{}", std::process::id()));
    let _ = std::fs::remove_dir_all(tmp.as_std_path());

    let unpack = || -> std::io::Result<()> {
        std::fs::create_dir_all(tmp.as_std_path())?;
        for (rel, bytes) in files() {
            let dest = tmp.join(&rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent.as_std_path())?;
            }
            std::fs::write(dest.as_std_path(), &bytes)?;
        }
        make_executable(&tmp.join(exe_rel))
    };

    if let Err(e) = unpack() {
        let _ = std::fs::remove_dir_all(tmp.as_std_path());
        return Err(Diagnostic::error(
            code::BUNDLE_UNPACK_FAILED,
            "could not unpack the bundled typesetting runtime",
        )
        .at(SourceLoc::file(tmp))
        .help("check for free space, and that the cache directory is writable")
        .detail(e.to_string()));
    }

    match std::fs::rename(tmp.as_std_path(), target.as_std_path()) {
        Ok(()) => Ok(target),
        // Lost the race. The winner's copy is identical — the key says so.
        Err(_) if target.join(exe_rel).exists() => {
            let _ = std::fs::remove_dir_all(tmp.as_std_path());
            Ok(target)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(tmp.as_std_path());
            Err(Diagnostic::error(
                code::BUNDLE_UNPACK_FAILED,
                "could not publish the unpacked typesetting runtime",
            )
            .at(SourceLoc::file(target))
            .detail(e.to_string()))
        }
    }
}

#[cfg(unix)]
fn make_executable(p: &Utf8Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p.as_std_path(), std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_p: &Utf8Path) -> std::io::Result<()> {
    Ok(())
}

/// Give a bundled fontconfig somewhere to look, on platforms that have no
/// system fontconfig for it to fall back to.
///
/// Windows only, and the asymmetry is the point. A fontconfig cross-built for
/// Windows carries a compiled-in config path from the machine that built it —
/// something under `/usr/x86_64-w64-mingw32/` — which does not exist on the
/// user's machine, so it loads nothing and every font lookup fails. Linux and
/// macOS have a real system configuration, and overriding it would replace
/// working font discovery with our guess at it.
///
/// Written rather than shipped because the paths must be absolute and the
/// runtime's location is content-addressed, so it is not known until now. The
/// content is a pure function of `root`, which makes rewriting it harmless.
fn ensure_fontconfig(root: &Utf8Path) -> Option<Utf8PathBuf> {
    if !cfg!(windows) {
        return None;
    }

    let conf = root.join("fontconfig.conf");
    let cache = root.join("fccache");
    let bundled = root.join("fonts");

    // `<dir>` entries are read in order and the bundle's own fonts come first,
    // so a document that asks for a font we ship gets ours rather than an
    // older copy someone installed system-wide.
    let body = format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<!-- Generated by BibleCompose for this unpacked runtime. Edits are lost. -->
<fontconfig>
  <dir>{bundled}</dir>
  <dir>WINDOWSFONTDIR</dir>
  <dir prefix="xdg">fonts</dir>
  <cachedir>{cache}</cachedir>
</fontconfig>
"#
    );

    let _ = std::fs::create_dir_all(cache.as_std_path());

    let existing = std::fs::read_to_string(conf.as_std_path()).ok();
    if existing.as_deref() != Some(body.as_str())
        && std::fs::write(conf.as_std_path(), &body).is_err()
    {
        // A read-only cache is survivable: fontconfig will complain and the
        // font pre-flight (FONT-001) will report it in terms a user can act on.
        return None;
    }

    Some(conf)
}

/// The `LUA_PATH` / `LUA_CPATH` / `SILE_PATH` an extracted runtime needs.
///
/// Not optional, and not obvious: [S1-NOTES P-7](../../../spike/S1-NOTES.md)
/// found that SILE's embedded Lua is not enough on its own. Without `LUA_PATH`
/// a real document dies inside SILE's own module cache with `attempt to
/// concatenate a nil value`, several frames from anything that names a path.
///
/// The separator is `;` on every platform — that is Lua's, not the OS's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnv {
    pub sile_path: Utf8PathBuf,
    pub lua_path: String,
    pub lua_cpath: String,
    /// `FONTCONFIG_FILE`, on the platforms where we have to supply one.
    pub fontconfig: Option<Utf8PathBuf>,
}

impl RuntimeEnv {
    pub fn for_root(root: &Utf8Path) -> Self {
        // Forward slashes, joined by hand rather than by `Utf8Path::join`.
        // These are read by Lua, not by the OS path layer, and Lua's own
        // searcher substitutes into them literally. Windows accepts `/`
        // everywhere, so one form works on both and stays readable in a log.
        let share = format!("{root}/lua_modules/share/lua/5.1");
        let lib = format!("{root}/lua_modules/lib/lua/5.1");

        // Both extensions on both platforms: the cross build produces Windows
        // DLLs that luarocks still names `.so`, and SILE searches both anyway
        // (S1-NOTES P-9).
        let lua_cpath = format!("{lib}/?.so;{lib}/?.dll;{lib}/?/core.so;{lib}/?/core.dll;;");

        RuntimeEnv {
            fontconfig: ensure_fontconfig(root),
            sile_path: root.to_owned(),
            lua_path: format!("{share}/?.lua;{share}/?/init.lua;;"),
            lua_cpath,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, seed: u8) -> Entry {
        Entry {
            path: path.to_owned(),
            hash: [seed; 32],
        }
    }

    fn files(pairs: &[(&str, &[u8])]) -> Vec<(String, Cow<'static, [u8]>)> {
        pairs
            .iter()
            .map(|(p, b)| ((*p).to_owned(), Cow::Owned(b.to_vec())))
            .collect()
    }

    #[test]
    fn the_key_does_not_depend_on_iteration_order() {
        let mut a = vec![entry("a", 1), entry("b", 2), entry("c", 3)];
        let mut b = vec![entry("c", 3), entry("a", 1), entry("b", 2)];
        assert_eq!(cache_key(&mut a), cache_key(&mut b));
    }

    #[test]
    fn the_key_changes_when_any_content_changes() {
        let mut base = vec![entry("a", 1), entry("b", 2)];
        let key = cache_key(&mut base);

        let mut one_byte_different = vec![entry("a", 1), entry("b", 3)];
        assert_ne!(key, cache_key(&mut one_byte_different));

        // A renamed file is a different runtime even with identical content,
        // because SILE resolves modules by path.
        let mut renamed = vec![entry("a", 1), entry("B", 2)];
        assert_ne!(key, cache_key(&mut renamed));
    }

    #[test]
    fn extraction_writes_the_tree_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");

        let out = ensure_extracted(&root, "abc123", "sile", || {
            files(&[
                ("sile", b"#!/bin/false\n"),
                ("core/sile.lua", b"-- core"),
                ("lua_modules/share/lua/5.1/pl/init.lua", b"-- penlight"),
            ])
        })
        .expect("extract");

        assert_eq!(out, root.join("abc123"));
        assert!(out.join("sile").exists());
        assert!(out.join("core/sile.lua").exists());
        assert!(out.join("lua_modules/share/lua/5.1/pl/init.lua").exists());

        let leftovers: Vec<_> = std::fs::read_dir(root.as_std_path())
            .expect("read root")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "temporary left behind: {leftovers:?}");
    }

    #[test]
    fn a_second_call_does_not_unpack_again() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");

        ensure_extracted(&root, "k", "sile", || files(&[("sile", b"first")])).expect("first");

        // If the bundle were consulted a second time this would panic, which is
        // the point: the warm path must not touch it.
        let out = ensure_extracted(
            &root,
            "k",
            "sile",
            || -> Vec<(String, Cow<'static, [u8]>)> {
                panic!("the bundle was read on the warm path")
            },
        )
        .expect("second");

        assert_eq!(
            std::fs::read(out.join("sile").as_std_path()).expect("read"),
            b"first"
        );
    }

    #[test]
    fn a_different_key_is_a_different_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");

        let old = ensure_extracted(&root, "v1", "sile", || files(&[("sile", b"old")])).expect("v1");
        let new = ensure_extracted(&root, "v2", "sile", || files(&[("sile", b"new")])).expect("v2");

        assert_ne!(old, new);
        assert!(
            old.join("sile").exists(),
            "upgrading removed the old runtime"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_executable_is_executable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("utf8");

        let out = ensure_extracted(&root, "k", "sile", || files(&[("sile", b"x")])).expect("x");
        let mode = std::fs::metadata(out.join("sile").as_std_path())
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "mode was {mode:o}");
    }

    #[test]
    fn the_runtime_env_points_at_the_extracted_tree() {
        let env = RuntimeEnv::for_root(Utf8Path::new("/cache/abc"));
        assert_eq!(env.sile_path, "/cache/abc");
        assert!(env
            .lua_path
            .contains("/cache/abc/lua_modules/share/lua/5.1/?.lua"));
        assert!(env.lua_path.ends_with(";;"), "{}", env.lua_path);
        assert!(env.lua_cpath.contains("?.so"));
        assert!(env.lua_cpath.contains("?.dll"));
    }
}
