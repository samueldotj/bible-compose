//! Noticing that the project changed under us (FUN-007).
//!
//! A fingerprint taken when the project was opened, compared on demand. Not a
//! filesystem watcher, deliberately:
//!
//! * A watcher on a synced folder — Dropbox, OneDrive, a network share, all of
//!   which is where translation projects actually live — produces storms of
//!   events for one logical change, and the debouncing that fixes it is a
//!   guess about how long a storm lasts.
//! * A watcher can miss changes while the application is not running; a
//!   comparison cannot, because it reads what is there now.
//! * Statting a few dozen files is bounded work with no background thread, no
//!   platform-specific API, and nothing to leak when a project is closed.
//!
//! The cost is latency: a change is noticed when someone asks rather than the
//! instant it happens. For an indication that offers a reload, that is the
//! right trade — the answer only has to be true by the time it is acted on.

use std::collections::BTreeMap;
use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};

/// What one file looked like.
///
/// Length as well as modification time, because a same-second edit that
/// changes the size is common — a translation tool rewriting a file it just
/// wrote — and filesystem timestamp resolution is coarse enough to miss it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl Stamp {
    fn of(path: &Utf8Path) -> Option<Stamp> {
        let meta = std::fs::metadata(path.as_std_path()).ok()?;
        Some(Stamp {
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// What the project looked like on disk when it was read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fingerprint {
    files: BTreeMap<Utf8PathBuf, Stamp>,
}

/// What has happened since.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    pub modified: Vec<Utf8PathBuf>,
    pub added: Vec<Utf8PathBuf>,
    pub removed: Vec<Utf8PathBuf>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.modified.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }

    pub fn len(&self) -> usize {
        self.modified.len() + self.added.len() + self.removed.len()
    }
}

/// The files a change would matter to: the Scripture, and the two files that
/// decide what is done with it.
fn watched(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut files = biblecompose_project::discovery::scripture_files(root);
    files.push(root.join(crate::project::SETTINGS_FILE));
    files.push(root.join(crate::project::STYLES_FILE));
    files.sort();
    files.dedup();
    files
}

impl Fingerprint {
    /// Take one now.
    pub fn take(root: &Utf8Path) -> Fingerprint {
        let mut files = BTreeMap::new();
        for path in watched(root) {
            // A file that does not exist is recorded by its absence, so its
            // *appearance* later is an addition — which is what a settings
            // file being created by hand is.
            if let Some(stamp) = Stamp::of(&path) {
                files.insert(path, stamp);
            }
        }
        Fingerprint { files }
    }

    /// What is different now.
    pub fn changes(&self, root: &Utf8Path) -> Changes {
        let mut changes = Changes::default();
        let mut seen = BTreeMap::new();

        for path in watched(root) {
            let Some(stamp) = Stamp::of(&path) else {
                continue;
            };
            match self.files.get(&path) {
                Some(before) if *before == stamp => {}
                Some(_) => changes.modified.push(path.clone()),
                None => changes.added.push(path.clone()),
            }
            seen.insert(path, ());
        }

        for path in self.files.keys() {
            if !seen.contains_key(path) {
                changes.removed.push(path.clone());
            }
        }

        changes
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
        std::fs::write(root.join("JHN.usfm").as_std_path(), "\\id JHN\n\\c 1\n").expect("write");
        (dir, root)
    }

    /// A file that has not been touched is not a change, however many times it
    /// is asked about.
    #[test]
    fn an_untouched_project_reports_nothing() {
        let (_d, root) = project();
        let before = Fingerprint::take(&root);
        assert!(before.changes(&root).is_empty());
        assert!(before.changes(&root).is_empty());
    }

    #[test]
    fn an_edited_scripture_file_is_reported() {
        let (_d, root) = project();
        let before = Fingerprint::take(&root);

        std::fs::write(
            root.join("JHN.usfm").as_std_path(),
            "\\id JHN\n\\c 1\n\\p\n\\v 1 In the beginning.\n",
        )
        .expect("write");

        let changes = before.changes(&root);
        assert_eq!(changes.modified.len(), 1, "{changes:?}");
        assert!(changes.modified[0].ends_with("JHN.usfm"));
    }

    /// STY-006's other half: a style sheet written by hand while the window is
    /// open is a change to the project, not just to a file beside it.
    #[test]
    fn a_style_sheet_appearing_is_reported() {
        let (_d, root) = project();
        let before = Fingerprint::take(&root);

        std::fs::write(
            root.join("styles.toml").as_std_path(),
            "[chapter]\nweight = 400\n",
        )
        .expect("write");

        let changes = before.changes(&root);
        assert_eq!(changes.added.len(), 1, "{changes:?}");
        assert!(changes.added[0].ends_with("styles.toml"));
    }

    #[test]
    fn a_deleted_book_is_reported() {
        let (_d, root) = project();
        let before = Fingerprint::take(&root);
        std::fs::remove_file(root.join("JHN.usfm").as_std_path()).expect("remove");

        let changes = before.changes(&root);
        assert_eq!(changes.removed.len(), 1, "{changes:?}");
    }

    /// The build directory is not the project. Left out, a build would report
    /// its own intermediates as an external change the moment it finished.
    #[test]
    fn our_own_working_files_are_not_changes() {
        let (_d, root) = project();
        let before = Fingerprint::take(&root);

        let build = root.join(".biblecompose").join("build").join("current");
        std::fs::create_dir_all(build.as_std_path()).expect("mkdir");
        std::fs::write(build.join("document.xml").as_std_path(), "<x/>").expect("write");
        std::fs::create_dir_all(root.join("output").as_std_path()).expect("mkdir");
        std::fs::write(root.join("output").join("bible.pdf").as_std_path(), "%PDF").expect("write");

        assert!(before.changes(&root).is_empty());
    }

    /// A same-second rewrite that changes the length is caught, because
    /// timestamp resolution alone would miss it.
    #[test]
    fn a_length_change_is_enough() {
        let (_d, root) = project();
        let path = root.join("JHN.usfm");
        let before = Fingerprint::take(&root);

        // Restore the original timestamp, so only the length differs.
        let stamp = std::fs::metadata(path.as_std_path())
            .and_then(|m| m.modified())
            .expect("a modification time");
        std::fs::write(path.as_std_path(), "\\id JHN\n\\c 1\n\\c 2\n").expect("write");
        let file = std::fs::File::options()
            .write(true)
            .open(path.as_std_path())
            .expect("reopen");
        file.set_modified(stamp).expect("restore the timestamp");

        assert_eq!(before.changes(&root).modified.len(), 1);
    }
}
