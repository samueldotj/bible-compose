//! FONT-001 and FONT-002 — the font is checked before a page is set.
//!
//! The case that matters is the one the spike watched succeed: a Latin font
//! asked to set Tamil produces a valid PDF with correct geometry, embedded
//! fonts and an extractable text layer, and every glyph on the page an empty
//! box. Exit code zero. Measured at 95.5% `.notdef` on a real corpus book.
//!
//! Two vendored directories make the test hermetic: `tests/fonts` has a Latin
//! face and no Indic, `spike/assets/fonts` has a Tamil face and no Latin. One
//! covers the script under test and one does not, which is exactly the pair a
//! coverage check needs.

use biblecompose_app::font::{self, ResolvedFont};
use biblecompose_diagnostics::{Diagnostics, Severity};
use biblecompose_scripture::plan::BookPlan;
use biblecompose_scripture::{fixtures, ScriptureDocument};
use biblecompose_testkit::repo_root;
use camino::{Utf8Path, Utf8PathBuf};

fn latin() -> Vec<Utf8PathBuf> {
    vec![repo_root().join("tests/fonts")]
}

fn tamil() -> Vec<Utf8PathBuf> {
    vec![repo_root().join("spike/assets/fonts")]
}

/// A real Tamil book from the corpus, normalized the way a build would.
fn tamil_scripture() -> (tempfile::TempDir, ScriptureDocument) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
    let source = repo_root().join("corpus/books/LAM-freebiblesindia-tamil.usfm");
    std::fs::copy(source.as_std_path(), root.join("LAM.usfm").as_std_path()).expect("copy");

    let loaded = biblecompose_app::project::load(&root, &BookPlan::canonical());
    (dir, loaded.document)
}

fn check(family: &str, doc: &ScriptureDocument, dirs: &[Utf8PathBuf]) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    font::preflight(family, doc, Utf8Path::new("."), dirs, &mut diagnostics);
    diagnostics
}

// ------------------------------------------------------------- FONT-002

/// The acceptance criterion: a Latin-only font configured for Tamil Scripture
/// produces a coverage error with an example reference, before SILE runs.
#[test]
fn a_latin_font_against_tamil_scripture_is_a_blocking_error() {
    let (_dir, doc) = tamil_scripture();
    let d = check("DejaVu Serif", &doc, &latin());

    let gap = d
        .iter()
        .find(|d| d.code.as_str() == "FONT-002")
        .expect("the coverage gap is reported");

    assert_eq!(gap.severity, Severity::Error);
    assert!(d.has_blocking(), "a book nobody can read is not a build");

    // Names the font, and points at Scripture rather than at a codepoint.
    assert!(gap.message.contains("DejaVu Serif"), "{}", gap.message);
    assert!(
        gap.message.contains("first at"),
        "no example reference: {}",
        gap.message
    );

    // And says what to do, in terms of the outcome being avoided.
    let help = gap.help.as_deref().unwrap_or_default();
    assert!(help.contains("empty boxes"), "{help}");
    assert!(gap.detail.is_some(), "the missing characters are listed");
}

/// The control. The same Scripture with a font that covers it is clean —
/// otherwise the check above would pass for the wrong reason.
#[test]
fn a_tamil_font_against_tamil_scripture_is_clean() {
    let (_dir, doc) = tamil_scripture();
    let d = check("Noto Serif Tamil", &doc, &tamil());

    let complaints: Vec<String> = d
        .iter()
        .filter(|d| d.code.as_str() == "FONT-002")
        .map(|d| d.to_string())
        .collect();
    assert!(complaints.is_empty(), "{complaints:?}");
}

#[test]
fn a_latin_font_against_latin_scripture_is_clean() {
    let d = check("DejaVu Serif", &fixtures::kitchen_sink(), &latin());
    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
}

/// The example points at a verse, which is the difference between a message a
/// publisher can act on and a list of hex values.
#[test]
fn the_example_names_a_place_in_the_text() {
    let (_dir, doc) = tamil_scripture();
    let gaps = font::gaps(
        &font::resolve("DejaVu Serif", Utf8Path::new("."), &latin()).expect("the vendored face"),
        &doc,
    )
    .expect("readable");

    let worst = gaps.first().expect("Tamil is not covered by DejaVu");
    let reference = worst.reference.as_ref().expect("an example reference");
    assert!(reference.chapter >= 1, "{reference}");
    assert!(
        worst.count > 100,
        "the commonest gap first: {}",
        worst.count
    );
}

// ------------------------------------------------------------- FONT-001

#[test]
fn a_font_nobody_has_is_a_blocking_error_that_names_it() {
    let d = check("Nonesuch Antiqua", &fixtures::john_1_1_5(), &latin());
    let only = d.iter().next().expect("reported");

    assert_eq!(only.code.as_str(), "FONT-001");
    assert_eq!(only.severity, Severity::Error);
    assert!(only.message.contains("Nonesuch Antiqua"));
    assert!(only.help.as_deref().unwrap().contains("assets/fonts"));
}

// ------------------------------------------------------------- FONT-003

/// A font the project ships resolves before anything installed, and is marked
/// as the project's — which is what makes the backend take a path rather than
/// a name.
#[test]
fn a_project_font_wins_and_is_marked_as_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
    let fonts = root.join(font::PROJECT_FONTS);
    std::fs::create_dir_all(fonts.as_std_path()).expect("mkdir");
    std::fs::copy(
        repo_root()
            .join("spike/assets/fonts/NotoSerifTamil-Regular.ttf")
            .as_std_path(),
        fonts.join("NotoSerifTamil-Regular.ttf").as_std_path(),
    )
    .expect("copy");

    let found: ResolvedFont =
        font::resolve("Noto Serif Tamil", &root, &latin()).expect("the project's own font");

    assert!(found.from_project, "a project font is the project's");
    assert!(found.path.starts_with(&root), "{}", found.path);
}

/// And a system or bundled font is not, so it keeps being named rather than
/// pinned to one file.
#[test]
fn a_backend_font_is_not_a_project_font() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");

    let found = font::resolve("DejaVu Serif", &root, &latin()).expect("the vendored face");
    assert!(!found.from_project);
}

// ------------------------------------------------------------- FONT-004

/// Tamil Scripture is not hyphenated, and the build says so.
///
/// The measured defect: SILE ships auto-generated Tamil patterns, they fire,
/// and one book of Lamentations came out with 510 hyphens in it against 7 in
/// the source text.
#[test]
fn tamil_scripture_is_not_hyphenated_and_the_build_says_so() {
    use biblecompose_app::hyphenation;

    let (_dir, doc) = tamil_scripture();
    let mut d = Diagnostics::new();
    let plan = hyphenation::decide("ta", true, &doc, &mut d);

    assert!(!plan.enabled, "Tamil does not break words across lines");
    assert_eq!(plan.language, "ta", "the tag is not rewritten to hide it");

    let said = d
        .iter()
        .find(|d| d.code.as_str() == "FONT-004")
        .expect("a setting that did nothing has to be mentioned");
    assert_eq!(
        said.severity,
        Severity::Info,
        "nothing is wrong with the project"
    );
    assert!(said.message.contains("Tamil"), "{}", said.message);
    assert!(!d.has_blocking());
}

/// And Latin Scripture still is, or the fix would be a regression dressed as
/// a diagnostic.
#[test]
fn latin_scripture_is_still_hyphenated() {
    use biblecompose_app::hyphenation;

    let mut d = Diagnostics::new();
    let plan = hyphenation::decide("en", true, &fixtures::kitchen_sink(), &mut d);
    assert!(plan.enabled);
    assert!(d.is_empty());
}

/// The script is read from the text, not from the language tag: a tag can be
/// absent, wrong, or describe a book that is mostly in another script.
#[test]
fn the_script_decides_rather_than_the_tag() {
    use biblecompose_app::hyphenation;

    let (_dir, doc) = tamil_scripture();
    assert_eq!(hyphenation::non_hyphenating_script(&doc), Some("Tamil"));
    assert_eq!(
        hyphenation::non_hyphenating_script(&fixtures::kitchen_sink()),
        None
    );

    // Tagged English, set in Tamil — the text wins.
    let mut d = Diagnostics::new();
    assert!(!hyphenation::decide("en", true, &doc, &mut d).enabled);
}

// ------------------------------------------------------------- GUI-003

/// The picker's list agrees with what a build would resolve, and answers the
/// question the pre-flight would otherwise answer too late.
#[test]
fn the_picker_says_which_fonts_can_set_the_scripture() {
    use biblecompose_app::font::{characters, choices, Source};

    let (_dir, doc) = tamil_scripture();
    let characters = characters(&doc);
    assert!(characters.len() > 50, "a real book, not a fixture");

    // Both vendored directories, so the list has one font that covers the
    // book and one that does not — and neither depends on this machine.
    let dirs = [latin(), tamil()].concat();
    let list = choices(Utf8Path::new("."), &dirs, &characters);

    let latin_face = list
        .iter()
        .find(|c| c.family == "DejaVu Serif")
        .expect("the Latin face is offered");
    let tamil_face = list
        .iter()
        .find(|c| c.family == "Noto Serif Tamil")
        .expect("the Tamil face is offered");

    assert_eq!(tamil_face.missing, Some(0), "it can set the book");

    // Exactly the Tamil letters, and not the punctuation and digits it shares
    // with Latin — a count, not a guess at one.
    let tamil_letters = characters
        .iter()
        .filter(|c| ('\u{0B80}'..='\u{0BFF}').contains(c))
        .count();
    assert!(tamil_letters > 30, "a real Tamil book: {tamil_letters}");
    assert_eq!(
        latin_face.missing,
        Some(tamil_letters),
        "the Latin face is missing the script and nothing else"
    );

    // Both came from the backend's directories rather than the project's,
    // which is what decides whether they travel with the book.
    assert_eq!(latin_face.source, Source::Backend);
    assert_eq!(tamil_face.source, Source::Backend);
}

/// A font the project ships is offered first and marked as the project's, the
/// same way `resolve` prefers it (FONT-003).
#[test]
fn a_project_font_is_offered_first() {
    use biblecompose_app::font::{choices, Source};

    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
    let fonts = root.join(font::PROJECT_FONTS);
    std::fs::create_dir_all(fonts.as_std_path()).expect("mkdir");
    std::fs::copy(
        repo_root()
            .join("spike/assets/fonts/NotoSerifTamil-Regular.ttf")
            .as_std_path(),
        fonts.join("NotoSerifTamil-Regular.ttf").as_std_path(),
    )
    .expect("copy");

    let list = choices(&root, &latin(), &Default::default());
    let first = list.first().expect("at least the project's own");

    assert_eq!(first.source, Source::Project);
    assert_eq!(first.family, "Noto Serif Tamil");
    assert_eq!(
        first.missing, None,
        "no Scripture to check against is not the same as covering it"
    );
}

// ------------------------------------------------- fonts a style names

fn styles_from(toml: &str) -> biblecompose_config::ResolvedStyles {
    let doc = biblecompose_config::ConfigDocument::parse("styles.toml", toml.to_owned())
        .expect("valid fixture");
    let (styles, d) = biblecompose_config::cascade::resolve(Some(&doc), false);
    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    styles
}

/// A style naming a font nobody has is a sentence, not a shaper crash.
///
/// Before this it reached SILE as a family fontconfig could not resolve, and
/// died as `attempt to index local 'file' (a nil value)` from inside
/// harfbuzz — a nil font file, several frames from anything that names one.
#[test]
fn a_style_font_nobody_has_is_reported_against_the_style() {
    let mut d = Diagnostics::new();
    let found = font::preflight_styles(
        &styles_from("[heading.s1]\nfont_family = \"Nonesuch Display\"\n"),
        Utf8Path::new("."),
        &latin(),
        &mut d,
    );

    assert!(found.is_empty(), "nothing resolved");
    let only = d.iter().next().expect("reported");
    assert_eq!(only.code.as_str(), "FONT-001");
    assert_eq!(only.severity, Severity::Error);
    assert!(
        only.message.contains("Nonesuch Display"),
        "{}",
        only.message
    );
    assert!(
        only.message.contains("heading.s1"),
        "the style that asked is named: {}",
        only.message
    );
}

/// One missing font is one diagnostic, however many selectors inherit it.
#[test]
fn a_font_many_styles_inherit_is_reported_once() {
    let mut d = Diagnostics::new();
    font::preflight_styles(
        &styles_from("[heading.s1]\nfont_family = \"Nonesuch Display\"\n"),
        Utf8Path::new("."),
        &latin(),
        &mut d,
    );

    assert_eq!(
        d.len(),
        1,
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    // s2, s3 and s4 inherit it, and naming all four buries the two facts that
    // matter — the font's name, and that nobody has it.
    let message = d.iter().next().expect("reported").message.clone();
    assert!(message.contains("and 1 more"), "{message}");
}

/// A font a style names and the machine has resolves, and says where from.
#[test]
fn a_style_font_that_exists_resolves() {
    let mut d = Diagnostics::new();
    let found = font::preflight_styles(
        &styles_from("[heading.s1]\nfont_family = \"DejaVu Serif\"\n"),
        Utf8Path::new("."),
        &latin(),
        &mut d,
    );

    assert!(
        d.is_empty(),
        "{:?}",
        d.iter().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    let face = found.get("DejaVu Serif").expect("the vendored face");
    assert!(!face.from_project);
}

/// A project font in a style crosses to the backend as a path, for the same
/// reason the body font does: fontconfig has never heard of it (FONT-003).
#[test]
fn a_project_font_in_a_style_crosses_as_a_path() {
    use biblecompose_app::backend_input::style_rules_with;

    let dir = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("UTF-8 path");
    let fonts = root.join(font::PROJECT_FONTS);
    std::fs::create_dir_all(fonts.as_std_path()).expect("mkdir");
    std::fs::copy(
        repo_root()
            .join("spike/assets/fonts/NotoSerifTamil-Regular.ttf")
            .as_std_path(),
        fonts.join("NotoSerifTamil-Regular.ttf").as_std_path(),
    )
    .expect("copy");

    let styles = styles_from("[heading.s1]\nfont_family = \"Noto Serif Tamil\"\n");
    let mut d = Diagnostics::new();
    let resolved = font::preflight_styles(&styles, &root, &latin(), &mut d);
    assert!(d.is_empty());

    let rule = style_rules_with(&styles, &resolved)
        .into_iter()
        .find(|r| r.selector == "heading.s1")
        .expect("a rule for the style");

    let named: Vec<&str> = rule.properties.iter().map(|(k, _)| k.as_str()).collect();
    assert!(named.contains(&"font_file"), "{named:?}");
    assert!(
        !named.contains(&"font_family"),
        "one way of naming a font, not two: {named:?}"
    );

    // And without resolution — the golden path — it stays a family name, so
    // the emitted document does not depend on this machine's fonts.
    let hermetic = biblecompose_app::backend_input::style_rules(&styles)
        .into_iter()
        .find(|r| r.selector == "heading.s1")
        .expect("a rule");
    assert!(hermetic.properties.iter().any(|(k, _)| k == "font_family"));
}
