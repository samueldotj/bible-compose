//! When a build can be skipped, and what makes that safe (P5.5).
//!
//! **The measurement first, because it decided the shape of this.** Opening a
//! synthetic 66-book project — 6.4 MB of real Scripture — and reading every
//! book through discovery, parsing and normalization takes **260 ms warm**
//! and 671 ms on a cold file cache. The acceptance bar is 500 ms warm, so it
//! is already met, and a discovery or parse cache would buy a quarter of a
//! second at the price of a whole class of stale-read bug. There is none here,
//! and that is a decision rather than an omission.
//!
//! What *is* slow is the backend. A book of five thousand verses takes SILE
//! thirty-five seconds; a Bible takes minutes. So the cache that is worth
//! having is the one that answers a different question — **has anything that
//! could change the PDF changed?** — and skips the backend when the answer is
//! no. That saves minutes rather than milliseconds.
//!
//! # The five parts
//!
//! [SRS-REVIEW F14d](../../../docs/SRS-REVIEW.md) names them, and names the
//! failure they prevent: a stale-cache bug gets blamed on the emitter. So all
//! five are in the fingerprint, and each has its own field rather than being
//! folded into a single opaque hash, so a test can move one at a time.
//!
//! * **The document** — the emitted XML, which already carries every book, the
//!   whole of the Scripture, and the resolved styles. Hashing the emitter's
//!   output rather than its inputs is the stronger choice: it cannot miss a
//!   thing the emitter reads.
//! * **The settings**, as the argument list the backend is given. Same
//!   reasoning: it is what the backend actually sees.
//! * **The marker table**, which is the model's vocabulary. It has no version
//!   of its own, so the application's stands in — a release that changes the
//!   table is a release.
//! * **The backend version**, because SILE's line breaking is not stable
//!   across releases and neither is its output.
//! * **The application version**, which covers the class, the emitter and
//!   everything else that has no version of its own.
//!
//! # What it deliberately does not cover
//!
//! Anything outside the project that the project points at: a system font, a
//! figure on another disk. Those are read at build time and are not hashed,
//! because hashing them means stat-ing the world on every build to catch a
//! case a `Clean build` already answers. The stamp is a promise about *this
//! project's* inputs and says so.

use camino::{Utf8Path, Utf8PathBuf};

/// The name of the stamp, beside the build directory rather than beside the
/// PDF: the PDF is the publisher's file and goes wherever they said, and
/// leaving a dotfile next to it is not this application's business.
const STAMP: &str = "fingerprint";

/// What a build was made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub document: u64,
    pub settings: u64,
    pub backend: String,
    pub application: &'static str,
}

impl Fingerprint {
    /// The fingerprint of one build.
    ///
    /// `xml` is the emitted document and `options` the backend's argument
    /// list, both taken after resolution — so nothing that reached the backend
    /// can be missing from this.
    pub fn of(xml: &str, options: &[(String, String)], backend: &str) -> Fingerprint {
        let mut settings = Hash::new();
        for (key, value) in options {
            settings.eat(key.as_bytes());
            settings.eat(b"=");
            settings.eat(value.as_bytes());
            settings.eat(b"\n");
        }
        Fingerprint {
            document: Hash::once(xml.as_bytes()),
            settings: settings.finish(),
            backend: backend.to_owned(),
            application: env!("CARGO_PKG_VERSION"),
        }
    }

    /// One line, which is both the file's contents and its comparison.
    ///
    /// Text rather than a binary blob so that a publisher who goes looking can
    /// read it, and a support question about a stale build can be answered by
    /// asking them to paste one line.
    pub fn to_line(&self) -> String {
        format!(
            "1 {:016x} {:016x} {} {}\n",
            self.document, self.settings, self.application, self.backend
        )
    }

    /// Whether the build that wrote `dir`'s stamp was made from this.
    ///
    /// A missing, unreadable or differently shaped stamp is a mismatch. There
    /// is no version negotiation here on purpose: the leading `1` exists so
    /// that a future format is *not* mistaken for this one, and the answer to
    /// an unrecognised stamp is to build.
    pub fn matches(&self, dir: &Utf8Path) -> bool {
        std::fs::read_to_string(stamp_path(dir).as_std_path())
            .map(|found| found == self.to_line())
            .unwrap_or(false)
    }

    /// Record this build. Failure is silent, and has to be: a stamp that
    /// cannot be written costs a rebuild next time, which is exactly what
    /// happens today, and refusing to publish a finished PDF over it would be
    /// a worse answer to a read-only directory.
    pub fn write(&self, dir: &Utf8Path) {
        let _ = std::fs::create_dir_all(dir.as_std_path());
        let _ = std::fs::write(stamp_path(dir).as_std_path(), self.to_line());
    }
}

/// Where a project's stamp lives: beside its build directory, under the folder
/// the application already owns.
pub fn stamp_dir(project_root: &Utf8Path) -> Utf8PathBuf {
    project_root.join(".biblecompose")
}

fn stamp_path(dir: &Utf8Path) -> Utf8PathBuf {
    dir.join(STAMP)
}

/// FNV-1a, 64-bit.
///
/// Not a cryptographic hash and does not need to be: this compares a build
/// against its own previous run on one machine, where the adversary is a
/// forgetful publisher rather than an attacker. Written out rather than taken
/// from `DefaultHasher`, whose output is explicitly not stable between Rust
/// releases — a stamp that changed meaning when the compiler was updated would
/// invalidate every cache in the world once and be very hard to explain.
struct Hash(u64);

impl Hash {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Hash {
        Hash(Hash::OFFSET)
    }

    fn eat(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(Hash::PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }

    fn once(bytes: &[u8]) -> u64 {
        let mut h = Hash::new();
        h.eat(bytes);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<(String, String)> {
        vec![("columns".to_owned(), "2".to_owned())]
    }

    fn base() -> Fingerprint {
        Fingerprint::of("<doc/>", &options(), "SILE 0.15.13")
    }

    /// Each of the parts moves the fingerprint, one at a time. The point of
    /// F14d: a part nothing tests is a part that quietly stops being in the key.
    #[test]
    fn every_part_changes_the_fingerprint() {
        let same = Fingerprint::of("<doc/>", &options(), "SILE 0.15.13");
        assert_eq!(base(), same, "the same inputs give the same fingerprint");

        let document = Fingerprint::of("<doc><p/></doc>", &options(), "SILE 0.15.13");
        assert_ne!(base().document, document.document);

        let settings = Fingerprint::of(
            "<doc/>",
            &[("columns".to_owned(), "1".to_owned())],
            "SILE 0.15.13",
        );
        assert_ne!(base().settings, settings.settings);

        let backend = Fingerprint::of("<doc/>", &options(), "SILE 0.16.0");
        assert_ne!(base().backend, backend.backend);

        // The application's version stands in for its own code, its class and
        // the marker table, none of which is separately versioned.
        assert_eq!(base().application, env!("CARGO_PKG_VERSION"));
    }

    /// **Two argument lists that differ only in where the boundaries are must
    /// not collide.** Concatenating keys and values without separators makes
    /// `ab=c` and `a=bc` the same bytes, and a publisher who moved a character
    /// between two settings would get the previous PDF.
    #[test]
    fn the_argument_list_is_hashed_with_its_boundaries() {
        let one = Fingerprint::of("<d/>", &[("ab".to_owned(), "c".to_owned())], "s");
        let other = Fingerprint::of("<d/>", &[("a".to_owned(), "bc".to_owned())], "s");
        assert_ne!(one.settings, other.settings);

        // And order is part of it: the argument list is ordered on purpose
        // (DET-001), so two orders are two builds.
        let ab = Fingerprint::of(
            "<d/>",
            &[
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "2".to_owned()),
            ],
            "s",
        );
        let ba = Fingerprint::of(
            "<d/>",
            &[
                ("b".to_owned(), "2".to_owned()),
                ("a".to_owned(), "1".to_owned()),
            ],
            "s",
        );
        assert_ne!(ab.settings, ba.settings);
    }

    #[test]
    fn a_stamp_matches_only_the_build_that_wrote_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = Utf8Path::from_path(dir.path()).expect("UTF-8 path");

        // Nothing written yet, so nothing matches. A first build always runs.
        assert!(!base().matches(path));

        base().write(path);
        assert!(base().matches(path));

        let other = Fingerprint::of("<doc><p/></doc>", &options(), "SILE 0.15.13");
        assert!(
            !other.matches(path),
            "a changed document is a changed build"
        );
    }

    /// A stamp from a format this release does not know is a mismatch rather
    /// than a guess.
    #[test]
    fn an_unrecognised_stamp_is_a_rebuild() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = Utf8Path::from_path(dir.path()).expect("UTF-8 path");
        std::fs::write(stamp_path(path).as_std_path(), "2 whatever comes next\n")
            .expect("write a stamp");
        assert!(!base().matches(path));
    }

    /// FNV-1a, against the values in its own specification, so a mistake in the
    /// transcription is caught here rather than as a cache that never hits.
    #[test]
    fn the_hash_is_the_one_it_claims_to_be() {
        assert_eq!(Hash::once(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(Hash::once(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(Hash::once(b"foobar"), 0x85944171f73967e8);
    }
}
