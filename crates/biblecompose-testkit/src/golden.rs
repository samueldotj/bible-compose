//! Golden-file comparison for emitted backend input.
//!
//! The emitted XML is **byte-compared** (SILE-005, DET-001). That is the half
//! of determinism which is achievable, and asserting it from the day the first
//! element is emitted is what stops a `HashMap` reaching the emission path
//! unnoticed — Rust randomises iteration order per process, so the failure
//! would otherwise be intermittent and unreproducible, which is the worst
//! available failure mode.
//!
//! Set `UPDATE_GOLDEN=1` to rewrite the files after a deliberate change.

use camino::Utf8Path;

pub const UPDATE_ENV: &str = "UPDATE_GOLDEN";

pub fn updating() -> bool {
    std::env::var(UPDATE_ENV).is_ok_and(|v| v == "1" || v == "true")
}

/// Compare `actual` against the golden file at `path`, or write it.
///
/// Panics with a readable diff rather than returning an error, because it is
/// only ever called from a test and a `Result` there just becomes `.unwrap()`.
pub fn assert_matches(path: &Utf8Path, actual: &str) {
    if updating() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent.as_std_path()).expect("create golden directory");
        }
        std::fs::write(path.as_std_path(), actual.as_bytes()).expect("write golden file");
        return;
    }

    let expected = match std::fs::read_to_string(path.as_std_path()) {
        Ok(s) => s,
        Err(e) => panic!(
            "missing golden file {path}: {e}\n\
             run the suite once with {UPDATE_ENV}=1 to create it, then review the diff"
        ),
    };

    // Compare the bytes, not a normalised form: line endings are part of what
    // determinism means here, and a golden test that normalises them cannot
    // catch a CRLF creeping in on Windows.
    if expected.as_bytes() != actual.as_bytes() {
        panic!("{}", diff(path, &expected, actual));
    }
}

fn diff(path: &Utf8Path, expected: &str, actual: &str) -> String {
    let mut out = format!("golden mismatch: {path}\n");
    let e: Vec<&str> = expected.lines().collect();
    let a: Vec<&str> = actual.lines().collect();

    if e.len() != a.len() {
        out.push_str(&format!("  line count {} → {}\n", e.len(), a.len()));
    }

    let mut shown = 0;
    for i in 0..e.len().max(a.len()) {
        let (le, la) = (e.get(i).copied(), a.get(i).copied());
        if le != la {
            out.push_str(&format!("  line {}:\n", i + 1));
            out.push_str(&format!("    expected: {}\n", le.unwrap_or("<missing>")));
            out.push_str(&format!("    actual:   {}\n", la.unwrap_or("<missing>")));
            shown += 1;
            if shown == 10 {
                out.push_str("  … further differences suppressed\n");
                break;
            }
        }
    }
    out.push_str(&format!(
        "\nrun with {UPDATE_ENV}=1 to accept the new output\n"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    #[test]
    fn identical_content_passes() {
        let d = tempfile::tempdir().unwrap();
        let p = Utf8PathBuf::from_path_buf(d.path().join("g.xml")).unwrap();
        std::fs::write(p.as_std_path(), b"<a/>\n").unwrap();
        assert_matches(&p, "<a/>\n");
    }

    #[test]
    #[should_panic(expected = "golden mismatch")]
    fn differing_content_fails_with_a_diff() {
        let d = tempfile::tempdir().unwrap();
        let p = Utf8PathBuf::from_path_buf(d.path().join("g.xml")).unwrap();
        std::fs::write(p.as_std_path(), b"<a/>\n").unwrap();
        assert_matches(&p, "<b/>\n");
    }

    /// A CRLF slipping in must fail, not be normalised away.
    #[test]
    #[should_panic(expected = "golden mismatch")]
    fn line_ending_changes_are_caught() {
        let d = tempfile::tempdir().unwrap();
        let p = Utf8PathBuf::from_path_buf(d.path().join("g.xml")).unwrap();
        std::fs::write(p.as_std_path(), b"<a/>\n").unwrap();
        assert_matches(&p, "<a/>\r\n");
    }
}
