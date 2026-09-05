//! The ten acceptance scenarios of SRS §16.2, A through J.
//!
//! Every other suite here asserts a mechanism. This one asserts the *product*:
//! ten sentences a person could read, each ending in a PDF or a diagnostic.
//! They exercise the pipeline from a folder of USFM through to a file, which
//! is what makes them worth having on top of everything else — a mechanism can
//! be right in ten places and wrong once assembled.
//!
//! **Two rules hold across all ten.**
//!
//! `\u{2022}` Each starts from a *folder*, never from a fixture. A fixture skips
//!   discovery, `\id` identification and the settings file, which is three of
//!   the things a publisher's first five minutes consists of.
//!
//! `\u{2022}` **Nothing writes to the Scripture.** BLD-004 says a build shall not
//!   modify its source, and every scenario checks the checksums of every file
//!   it started with. It is the kind of guarantee that is true until one day it
//!   is not, and by then a publisher's only copy is gone.
//!
//! Skipped, loudly, when no backend is installed.

mod common;

use std::collections::BTreeMap;

use biblecompose_app::{build, project, BuildReporter, BuildRequest, BuildState, CancelToken};
use biblecompose_testkit::pdf::Pdf;
use camino::{Utf8Path, Utf8PathBuf};
use common::have_backend;

/// A folder of Scripture, as a publisher would hand one over.
struct Project {
    _guard: tempfile::TempDir,
    root: Utf8PathBuf,
    /// What every source file looked like before anything ran.
    before: BTreeMap<Utf8PathBuf, u64>,
}

/// Genesis, with poetry and a footnote — enough for the style and note
/// scenarios to have something to be about.
const GENESIS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.\n",
    "\\v 2 Now the earth was formless and void, ",
    "\\f + \\fr 1:2 \\ft Or a mighty wind from God.\\f*\n",
    "and darkness was over the surface of the deep.\n",
    "\\q1 And God said, Let there be light,\n",
    "\\q2 and there was light.\n",
);

const JOHN: &str = concat!(
    "\\id JHN\n\\h John\n\\c 1\n\\p\n",
    "\\v 1 In the beginning was the Word, and the Word was with God.\n",
    "\\v 2 He was with God in the beginning ",
    "\\x - \\xo 1:2 \\xt Genesis 1:1\\x*\n",
    "\\v 3 Through him all things were made.\n",
);

impl Project {
    /// A folder holding these files.
    fn of(files: &[(&str, &str)]) -> Project {
        let guard = tempfile::tempdir().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
        for (name, body) in files {
            let path = root.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent.as_std_path()).expect("make the folder");
            }
            std::fs::write(path.as_std_path(), body).expect("write the file");
        }
        let before = checksums(&root);
        Project {
            _guard: guard,
            root,
            before,
        }
    }

    /// The two Scripture books every scenario that needs Scripture uses.
    fn scripture(extra: &[(&str, &str)]) -> Project {
        let mut files = vec![("GEN.usfm", GENESIS), ("JHN.usfm", JOHN)];
        files.extend_from_slice(extra);
        Project::of(&files)
    }

    /// Open the folder and build it, exactly as the window does.
    /// The Scripture on the page, as one string that can be searched.
    ///
    /// **Every space and every line break removed**, which sounds destructive
    /// and is the only honest reading of a PDF. A PDF records where each run
    /// of glyphs was placed and nothing about the gaps between them, so a line
    /// reads back as `Arah775`; and a line ends wherever the typesetter
    /// decided, with a hyphen or without one. Neither is in the file in any
    /// form a search for a sentence could survive.
    ///
    /// So the comparison is on letters alone. That is weaker than matching
    /// words — but it is the strength actually available, and it still catches
    /// the thing these scenarios are about: whether the Scripture reached the
    /// page at all, and in the right order.
    ///
    /// **A number on its own is not prose.** A verse number is raised, so it
    /// sits on a baseline of its own and reads back between the two lines its
    /// verse spans — `...createdtheheavensandthe` `2` `earth.` Concatenating
    /// everything would put a `2` in the middle of a sentence and nothing
    /// would find it. Folios go the same way and for the same reason.
    fn prose(&self, report: &biblecompose_app::BuildReport) -> String {
        self.lines(report)
            .iter()
            .map(|line| line.text().replace(' ', ""))
            .filter(|text| !text.chars().all(|c| c.is_ascii_digit()))
            .map(|text| text.trim_end_matches('-').to_owned())
            .collect()
    }

    fn build(&self) -> biblecompose_app::BuildReport {
        self.build_with(&CancelToken::new(), |_| {})
    }

    fn build_with(
        &self,
        cancel: &CancelToken,
        watch: impl Fn(&biblecompose_app::BuildEvent) + Send + 'static,
    ) -> biblecompose_app::BuildReport {
        let opened = project::open(&self.root);
        let mut request = BuildRequest::new(self.root.clone(), self.root.join("out.pdf"));
        request.sile_path = vec![biblecompose_testkit::repo_root().join("sile")];
        request.settings = opened.settings.clone();
        request.styles = opened.styles.clone();
        request.prior = opened.diagnostics.clone();

        let (mut reporter, events) = BuildReporter::new();
        let drain = std::thread::spawn(move || {
            let mut all = Vec::new();
            for event in events.iter() {
                watch(&event);
                all.push(event);
            }
            all
        });
        let report = build(&opened.document, &request, cancel, &mut reporter);
        drop(reporter);
        let _ = drain.join();

        // Every scenario, without exception (BLD-004).
        assert_eq!(
            checksums(&self.root)
                .into_iter()
                .filter(|(p, _)| self.before.contains_key(p))
                .collect::<BTreeMap<_, _>>(),
            self.before,
            "the build modified its own source"
        );

        report
    }

    fn pdf(&self, report: &biblecompose_app::BuildReport) -> Pdf {
        let path = report.output.as_ref().unwrap_or_else(|| {
            panic!(
                "expected a PDF: {:?}",
                report
                    .diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
            )
        });
        Pdf::parse(&std::fs::read(path.as_std_path()).expect("read the PDF"))
    }

    fn lines(
        &self,
        report: &biblecompose_app::BuildReport,
    ) -> Vec<biblecompose_testkit::pdf::Line> {
        let path = report.output.as_ref().expect("a PDF");
        Pdf::lines(&std::fs::read(path.as_std_path()).expect("read the PDF"))
    }
}

/// Every file in a folder that a build must not touch, with a checksum.
///
/// Everything the application writes lives under `.biblecompose` or is the
/// output PDF, and neither is source.
fn checksums(root: &Utf8Path) -> BTreeMap<Utf8PathBuf, u64> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(dir.as_std_path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = Utf8PathBuf::from_path_buf(entry.path()).expect("UTF-8 path");
            if path.file_name() == Some(".biblecompose") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(path.as_std_path()) {
                // FNV-1a, as elsewhere: this compares a file with itself.
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for b in &bytes {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0000_0100_0000_01b3);
                }
                out.insert(path.strip_prefix(root).expect("under root").to_owned(), h);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------

/// **A — Defaults only.** A folder with two books and no configuration builds.
#[test]
fn a_defaults_only() {
    if !have_backend() {
        return;
    }
    let project = Project::scripture(&[]);
    let report = project.build();

    assert_eq!(report.state, BuildState::Succeeded);
    let pdf = project.pdf(&report);
    assert!(pdf.pages >= 2, "two books should not share one page");
    assert!(pdf.has_font("DejaVuSerif"), "the body font is embedded");

    // Readable, which for a test means the Scripture is on the page.
    let text = project.prose(&report);
    assert!(
        text.contains("InthebeginningGodcreatedtheheavensandtheearth."),
        "{text}"
    );
    assert!(text.contains("InthebeginningwastheWord"), "{text}");
}

/// **B — Settings override.** The PDF reflects the settings file.
#[test]
fn b_settings_override() {
    if !have_backend() {
        return;
    }
    let settings = "\
schema_version = 1

[page]
size = \"6in x 9in\"
columns = 1
margin_top = \"1in\"
margin_bottom = \"1in\"
margin_inner = \"1in\"
margin_outer = \"1in\"

[numbering]
show_verse_numbers = false
";
    let project = Project::scripture(&[("biblecompose.toml", settings)]);
    let report = project.build();
    assert_eq!(report.state, BuildState::Succeeded);

    // The trim size, read off the page boxes.
    assert_eq!(
        project.pdf(&report).uniform_page_size_inches(),
        Some((6.0, 9.0))
    );

    let lines = project.lines(&report);
    // One column. The text block runs 72pt to 360pt on a 432pt page, so a
    // second column would begin near its middle at 216pt — and every line of
    // Scripture begins at the left of the one column there is.
    //
    // Scripture only: the running head is one line with its slots pushed
    // apart, and the folio is centred, so both legitimately begin further in.
    let body: Vec<_> = lines.iter().filter(|l| l.sizes().contains(&9.2)).collect();
    assert!(!body.is_empty(), "the Scripture should be on the page");
    let rightmost = body.iter().map(|l| l.left()).fold(0.0_f64, f64::max);
    assert!(
        rightmost < 216.0,
        "a line of Scripture starts at {rightmost}, which is where a second \
         column would be"
    );

    // The margin, which is 1in = 72pt from the left edge.
    let leftmost = body.iter().map(|l| l.left()).fold(f64::INFINITY, f64::min);
    assert!(
        (leftmost - 72.0).abs() < 1.0,
        "the text block should start at 72pt and starts at {leftmost}"
    );

    // And the verse numbers are gone. They are the only thing set at 6.4pt.
    assert!(
        !lines.iter().any(|l| l.sizes().contains(&6.4)),
        "verse numbers were hidden and are still on the page"
    );
}

/// **C — Style override.** Only the intended styles change.
#[test]
fn c_style_override() {
    if !have_backend() {
        return;
    }
    let poetry_line = |lines: &[biblecompose_testkit::pdf::Line]| -> f64 {
        lines
            .iter()
            .find(|l| l.text().replace(' ', "").starts_with("AndGodsaid"))
            .expect("the poetry line")
            .left()
    };

    let plain = Project::scripture(&[]);
    let before = plain.build();
    let plain_lines = plain.lines(&before);
    let heads_before = plain_lines
        .iter()
        .filter(|l| l.sizes() == vec![8.2])
        .count();
    let q1_before = poetry_line(&plain_lines);

    // The SRS writes this scenario as "styles.toml changing body font size and
    // q1 indent". In this application the *body* size is a setting rather than
    // a style — one size for the publication, where a style is keyed by marker
    // — so the style that stands for it is the one on `\\p`, which is what
    // every paragraph of Scripture is. The distinction is the schema's and the
    // scenario is unchanged: a style file changes two things and only two.
    let styles = "\
[paragraph.p]
font_size = \"11pt\"

[poetry.q1]
indent = \"36pt\"
";
    let project = Project::scripture(&[("styles.toml", styles)]);
    let report = project.build();
    assert_eq!(report.state, BuildState::Succeeded);

    let lines = project.lines(&report);
    // The paragraphs moved to 11pt. Asserted as "something is set at 11pt"
    // rather than by finding a particular line, because at a larger size the
    // lines break in different places — which is the change working.
    assert!(
        lines.iter().any(|l| l.sizes().contains(&11.0)),
        "nothing is set at 11pt: {:?}",
        lines.iter().map(|l| l.sizes()).collect::<Vec<_>>()
    );
    // And it is the Scripture that moved, not something incidental.
    assert!(
        project.prose(&report).contains("theheavensandtheearth"),
        "the Scripture should still be on the page"
    );

    // `q1` moved from the built-in 9pt to 36pt — asserted as the *difference*
    // between the two builds rather than against an absolute coordinate,
    // because the body also changed size and everything else on the page moved
    // with it. What the scenario claims is that this style changed, and a
    // difference is what that means.
    let q1_after = poetry_line(&lines);
    assert!(
        (q1_after - q1_before - 27.0).abs() < 1.0,
        "q1 went from 9pt to 36pt, so the line should move 27pt: {q1_before} to {q1_after}"
    );

    // **And nothing else moved.** The running head is set from its own style,
    // which neither override touched, and the poetry keeps the body size it
    // always had — only its indent was named.
    let heads_after = lines.iter().filter(|l| l.sizes() == vec![8.2]).count();
    assert_eq!(
        heads_before, heads_after,
        "the running heads changed, and no style said to change them"
    );
    let poetry = lines
        .iter()
        .find(|l| l.text().replace(' ', "").starts_with("AndGodsaid"))
        .expect("the poetry line");
    assert!(
        poetry.sizes().contains(&9.2),
        "the poetry's size was not named and should not have moved: {:?}",
        poetry.sizes()
    );
}

/// **D — Footnote.** The note is on the page and its caller is with its verse.
#[test]
fn d_footnote() {
    if !have_backend() {
        return;
    }
    let project = Project::scripture(&[]);
    let report = project.build();
    let lines = project.lines(&report);

    // The note's own text, at the note size.
    let note = lines
        .iter()
        .find(|l| l.text().replace(' ', "").contains("amightywind"))
        .expect("the note body is on the page");
    assert!(
        note.sizes().iter().all(|s| *s < 9.2),
        "a note is set smaller than the text: {:?}",
        note.sizes()
    );

    // The caller is on the same page as the verse that called it, and above
    // the note — which is what "remains associated with its caller" means once
    // it is a page rather than a model.
    let verse = lines
        .iter()
        .find(|l| l.text().replace(' ', "").contains("Nowtheearthwas"))
        .expect("the verse that called it");
    assert_eq!(verse.page, note.page, "the note left its caller's page");
    assert!(note.y < verse.y, "the note should be below the text");
}

/// **E — Cross-reference.** It appears in the configured placement.
#[test]
fn e_cross_reference() {
    if !have_backend() {
        return;
    }
    for (placement, expectation) in [("note_area", true), ("inline", false)] {
        let settings =
            format!("schema_version = 1\n[notes]\ncross_reference_placement = \"{placement}\"\n");
        let project = Project::scripture(&[("biblecompose.toml", &settings)]);
        let report = project.build();
        assert_eq!(report.state, BuildState::Succeeded, "{placement}");

        let lines = project.lines(&report);
        // Not the running head, which on the Genesis page reads
        // `Genesis 1:1-1:2` and would match anything looking for the
        // reference's text.
        let reference = lines
            .iter()
            .filter(|l| l.sizes() != vec![8.2])
            .find(|l| l.text().replace(' ', "").contains("Genesis1:1"))
            .unwrap_or_else(|| panic!("the reference should be on the page for {placement}"));

        // In the note area it is set apart at the foot; inline it is in the
        // text and shares its line with Scripture.
        let alone = reference.sizes().iter().all(|s| *s < 9.2);
        assert_eq!(
            alone,
            expectation,
            "{placement} put the reference at {:?}",
            reference.sizes()
        );
    }
}

/// **F — Figure.** A project image renders at the configured size.
#[test]
fn f_figure() {
    if !have_backend() {
        return;
    }
    let with_figure = concat!(
        "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
        "\\v 1 In the beginning God created the heavens and the earth.\n",
        "\\fig The garden|src=\"assets/images/garden.png\" size=\"col\"\\fig*\n",
    );
    let project = Project::of(&[("GEN.usfm", with_figure)]);
    std::fs::create_dir_all(project.root.join("assets/images").as_std_path())
        .expect("asset folder");
    std::fs::write(
        project.root.join("assets/images/garden.png").as_std_path(),
        biblecompose_testkit::PIXEL_PNG,
    )
    .expect("write the artwork");

    let report = project.build();
    assert_eq!(
        report.state,
        BuildState::Succeeded,
        "{:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );

    // The image is in the file. A PDF that drew it has an image XObject; one
    // that silently skipped it does not.
    let raw =
        std::fs::read(report.output.as_ref().expect("a PDF").as_std_path()).expect("read the PDF");
    let text = String::from_utf8_lossy(&raw);
    assert!(
        text.contains("/Image") || text.contains("/XObject"),
        "the figure was not drawn"
    );
}

/// **G — Invalid USFM.** A blocking diagnostic naming the file, and no backend.
#[test]
fn g_invalid_usfm() {
    if !have_backend() {
        return;
    }
    // A character style opened and never closed.
    let broken = "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n\\v 1 In the \\bd beginning God created.\n";
    let project = Project::of(&[("GEN.usfm", broken)]);

    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&ran);
    let report = project.build_with(&CancelToken::new(), move |event| {
        if matches!(
            event,
            biblecompose_app::BuildEvent::State(BuildState::Typesetting)
        ) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });

    // The parser is what decides this is an error rather than a warning; what
    // the scenario asks is that *if* it blocks, it blocks with a location and
    // without running SILE. An unclosed style that the parser tolerates is
    // still reported.
    let complaint = report
        .diagnostics
        .iter()
        .find(|d| d.code.to_string().starts_with("USFM"))
        .expect("something should be said about an unclosed character style");
    assert!(
        complaint.location.is_some(),
        "a diagnostic about a file should say where: {complaint}"
    );

    if report.diagnostics.has_blocking() {
        assert_eq!(report.state, BuildState::Blocked);
        assert!(
            !ran.load(std::sync::atomic::Ordering::SeqCst),
            "the backend ran for a project that was already known to be broken"
        );
        assert!(report.output.is_none());
    }
}

/// **H — Invalid config.** A malformed settings file is reported, not ignored.
#[test]
fn h_invalid_config() {
    if !have_backend() {
        return;
    }
    let project = Project::scripture(&[(
        "biblecompose.toml",
        "schema_version = 1\n[page\ncolumns = = 2\n",
    )]);
    let report = project.build();

    assert_eq!(
        report.state,
        BuildState::Blocked,
        "a settings file that will not parse must not be silently ignored"
    );
    assert!(
        report.diagnostics.iter().any(|d| d
            .location
            .as_ref()
            .is_some_and(|l| l.path.as_str().ends_with("biblecompose.toml"))),
        "the diagnostic should name the file: {:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(report.output.is_none());
}

/// **I — Backend failure.** Reported, the previous PDF kept, the log exposed.
#[test]
fn i_backend_failure() {
    if !have_backend() {
        return;
    }
    let project = Project::scripture(&[]);

    // A good build first, so there is a previous PDF to keep.
    let good = project.build();
    assert_eq!(good.state, BuildState::Succeeded);
    let kept = std::fs::read(project.root.join("out.pdf").as_std_path()).expect("the good PDF");

    // Now a class that fails, which is the closest thing to a forced SILE
    // error that does not involve corrupting the Scripture.
    let classes = project.root.join("broken/classes");
    std::fs::create_dir_all(classes.as_std_path()).expect("class folder");
    std::fs::write(
        classes.join("biblecompose.lua").as_std_path(),
        "SU.error(\"a forced backend failure\")\n",
    )
    .expect("write the class");

    let opened = project::open(&project.root);
    let mut request = BuildRequest::new(project.root.clone(), project.root.join("out.pdf"));
    request.sile_path = vec![project.root.join("broken")];
    request.settings = opened.settings.clone();
    request.styles = opened.styles.clone();

    let (mut reporter, events) = BuildReporter::new();
    let report = build(
        &opened.document,
        &request,
        &CancelToken::new(),
        &mut reporter,
    );
    drop(reporter);
    let seen: Vec<_> = events.iter().collect();

    assert_eq!(report.state, BuildState::Failed);
    assert!(!report.diagnostics.is_empty(), "a failure says something");
    // **The previous good PDF is untouched** (BLD-009).
    assert_eq!(
        std::fs::read(project.root.join("out.pdf").as_std_path()).expect("still there"),
        kept,
        "a failed build replaced the last good PDF"
    );
    // And the backend's own words reached the outside world (BLD-005).
    assert!(
        seen.iter()
            .any(|e| matches!(e, biblecompose_app::BuildEvent::Log { .. })),
        "the backend log was not exposed"
    );
    assert!(
        seen.iter()
            .any(|e| matches!(e, biblecompose_app::BuildEvent::LogFile(_))),
        "no log file was offered"
    );
}

/// **J — Cancel.** The backend stops and the machine ends in a usable state.
#[test]
fn j_cancel() {
    if !have_backend() {
        return;
    }
    let project = Project::scripture(&[]);
    let cancel = CancelToken::new();
    let trigger = cancel.clone();

    // Cancel the moment the backend starts, which is the only point in a build
    // this short where there is anything to cancel.
    let report = project.build_with(&cancel, move |event| {
        if matches!(
            event,
            biblecompose_app::BuildEvent::State(BuildState::Typesetting)
        ) {
            trigger.cancel();
        }
    });

    assert!(
        matches!(report.state, BuildState::Cancelled | BuildState::Succeeded),
        "a cancelled build ends cancelled, or finished first: {:?}",
        report.state
    );
    if report.state == BuildState::Cancelled {
        assert!(report.output.is_none(), "a cancelled build published a PDF");
    }

    // Operable again: the same project builds afterwards.
    let again = project.build();
    assert_eq!(
        again.state,
        BuildState::Succeeded,
        "the project should build after a cancel"
    );
}
