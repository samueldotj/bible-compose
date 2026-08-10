//! Golden-file tests for emitted backend input.
//!
//! SILE-005 and DET-001: the generated input is byte-identical across runs,
//! machines and operating systems for identical input. This is the half of
//! determinism that is achievable, and it is asserted from the day the first
//! element is emitted rather than at M5, because retrofitting it means
//! auditing every map in the codebase.
//!
//! `UPDATE_GOLDEN=1 cargo test -p biblecompose-sile` rewrites the files.

use biblecompose_scripture::fixtures;
use biblecompose_sile::emit;
use biblecompose_testkit::golden;

#[test]
fn every_fixture_matches_its_golden_file() {
    let dir = biblecompose_testkit::golden_dir();
    for (name, doc) in fixtures::all() {
        let emitted = emit(&doc, &[]);
        golden::assert_matches(&dir.join(format!("{name}.xml")), &emitted.xml);
    }
}

/// The property the golden files exist to protect. A `HashMap` on the emission
/// path would make this fail on roughly one machine in three, which is why the
/// ordered-map lint in `deny.rs` backs it up rather than relying on this alone.
#[test]
fn emission_does_not_vary_between_runs_in_one_process() {
    for (name, doc) in fixtures::all() {
        let first = emit(&doc, &[]).xml;
        for _ in 0..64 {
            assert_eq!(first, emit(&doc, &[]).xml, "{name} varied between runs");
        }
    }
}

/// DET-001 says "across machines and operating systems". Line endings are the
/// way that breaks in practice, on Windows, silently.
#[test]
fn golden_files_are_stored_with_lf_endings() {
    let dir = biblecompose_testkit::golden_dir();
    for name in fixtures::names() {
        let path = dir.join(format!("{name}.xml"));
        let Ok(bytes) = std::fs::read(path.as_std_path()) else {
            continue; // the golden test above reports a missing file properly
        };
        assert!(
            !bytes.contains(&b'\r'),
            "{path} contains a CR — golden files are LF-only so they compare \
             identically on every platform"
        );
    }
}

/// The adversarial fixture is the one that matters most, so its content is
/// asserted directly as well as by byte comparison — a golden file accepted by
/// mistake would otherwise lock in a regression.
#[test]
fn the_adversarial_golden_still_contains_inert_sil_syntax() {
    let xml = emit(&fixtures::adversarial(), &[]).xml;
    assert!(xml.contains(r"\bd"), "a backslash must survive as text");
    assert!(xml.contains(r"\par"));
    assert!(xml.contains("{like this}"));
    assert!(xml.contains("100%"));
    assert!(xml.contains("&amp;"));
    assert!(
        !xml.contains("<angle>"),
        "a bracket in Scripture must not become an element"
    );
}
