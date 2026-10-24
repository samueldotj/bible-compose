//! Suggesting a font for a language, and packaging it with the book (FONT-006).
//!
//! [`font`](crate::font) answers "can this font set this Scripture", and
//! answers it honestly enough that a publisher setting Tamil in a Latin face
//! is stopped before the paper. What it leaves them with is a refusal and
//! nowhere to go: the machine has no Tamil serif, the picker can only offer
//! what the machine has, and the next step is a web search, a licence nobody
//! reads, and a file dropped in the right folder by hand.
//!
//! This module is that next step, done by the application. It knows which
//! openly licensed families cover which script, it fetches one into
//! `assets/fonts/`, and because that is where a project keeps the fonts it
//! ships (FONT-003), the book then travels with its own typeface — which is
//! the part a publisher actually wanted and the part a system install does not
//! give them.
//!
//! # Three ways to ask the same question
//!
//! A suggestion needs a script, and there are three sources for one, in
//! descending order of how much they can be trusted:
//!
//! 1. **The Scripture's own characters.** Nothing beats it, and the session
//!    already keeps the set for the coverage check, so it costs nothing.
//! 2. **The project's language tag.** All a new project has, which is exactly
//!    when a publisher most needs the answer, so [`SCRIPTS`] carries a list of
//!    the tags written in each script.
//! 3. **Nothing.** Then there is no suggestion, and saying so is right.
//!
//! [`suggest`] takes both and prefers the characters, so a project whose tag
//! says Hindi and whose text is Bengali gets the font the text needs.
//!
//! # Why the download is not in this crate
//!
//! It is in the shell, which is the only layer with any business opening a
//! socket, and this module takes bytes from a caller. That keeps the build
//! path and the command-line tool provably free of network code — NFR-004's
//! offline guarantee is not a promise about intent if an HTTP client is linked
//! into the thing that composes the page — and it makes every case below
//! testable without a server: a truncated file, a wrong digest, a file that is
//! not a font, a font that cannot draw the script it was offered for.
//!
//! # What is verified before anything is written
//!
//! All of it, and nothing is written until all of it passes. The catalogue
//! records a byte count and a SHA-256 for each file at a pinned revision, and
//! on top of that the bytes are parsed, asked what family they belong to, and
//! asked for a glyph. The digest alone would be enough against a truncated
//! download and nothing like enough against the failure that matters, which is
//! a catalogue row that is simply wrong — so the last two checks ask the
//! question a publisher cares about rather than the question a mirror can
//! answer.

use std::collections::BTreeSet;

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};

use crate::font::PROJECT_FONTS;

pub mod catalogue;

pub use catalogue::{HOST, REPOSITORY, REVISION, SCRIPTS, TYPEFACES};

/// A writing system, and how to recognise one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Script {
    /// The ISO 15924 code, lowercased. What a wire message and a test name it
    /// by, because the display name is prose and prose gets rewritten.
    pub id: &'static str,
    /// What the window calls it.
    pub name: &'static str,
    /// A printable base letter, for probing whether a font can draw the script.
    ///
    /// Never a combining mark: a mark has no standalone glyph, so probing with
    /// one would report every font as lacking every script.
    pub sample: char,
    /// The codepoints that mean this script is in use.
    pub ranges: &'static [(u32, u32)],
    /// BCP-47 primary subtags written in it, for a project with no text yet.
    ///
    /// A suggestion list and not a schema, the same way the window's language
    /// field is: a tag nobody listed falls through to the codepoints, which
    /// are better evidence in any case.
    pub languages: &'static [&'static str],
    /// The scripts whose presence already implies this one.
    ///
    /// Japanese and Korean are written with Han characters among the kana and
    /// the hangul, so a Japanese Gospel genuinely contains two scripts and
    /// genuinely needs one font. Without this, the window would offer a
    /// Japanese face and then a list of Chinese ones underneath it, as though
    /// the publisher had a second decision to make.
    pub subsumed_by: &'static [&'static str],
}

/// Which face a file is, as a build selects one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Face {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl Face {
    pub const fn as_str(self) -> &'static str {
        match self {
            Face::Regular => "regular",
            Face::Bold => "bold",
            Face::Italic => "italic",
            Face::BoldItalic => "bold-italic",
        }
    }
}

/// One file of a family, as the catalogue records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFile {
    /// Its name upstream, which is also the name it is written under.
    pub name: &'static str,
    pub face: Face,
    pub bytes: usize,
    /// Lowercase hex, 64 characters.
    pub sha256: &'static str,
    /// Whether it is a variable font, which decides what a build can ask of it.
    ///
    /// A project font crosses to the backend as a path rather than a family
    /// name (FONT-003), and a path names one file and therefore one instance.
    /// So a variable file gives a page its default instance and no other: a
    /// style asking for bold gets the regular, and FONT-005 says so before the
    /// build. [`Packaged::faces`] is what lets the window say it beforehand.
    pub variable: bool,
}

/// The licence file that travels with the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Licence {
    pub bytes: usize,
    pub sha256: &'static str,
}

/// One family the catalogue can fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Typeface {
    /// The family name a build resolves against, as the font's own name table
    /// spells it. Checked against the file when the catalogue is generated.
    pub family: &'static str,
    /// Its directory in the upstream repository.
    pub directory: &'static str,
    /// The [`Script::id`]s it was confirmed to draw.
    pub scripts: &'static [&'static str],
    /// One sentence on why it is worth choosing, shown beside the name.
    pub note: &'static str,
    pub licence: Licence,
    pub files: &'static [SourceFile],
}

impl Typeface {
    /// The faces a build could actually select from this family.
    ///
    /// Derived from the files rather than declared, so it cannot disagree with
    /// what would be downloaded. A variable file contributes its default
    /// instance only, for the reason [`SourceFile::variable`] gives.
    pub fn faces(&self) -> Vec<Face> {
        let mut faces: Vec<Face> = self.files.iter().map(|f| f.face).collect();
        faces.sort_unstable();
        faces.dedup();
        faces
    }

    /// How many bytes fetching the whole family costs.
    ///
    /// Shown before the download starts, because the answer ranges from 90 kB
    /// to 3 MB and on a translator's connection that is the difference between
    /// a click and a decision.
    pub fn bytes(&self) -> usize {
        self.licence.bytes + self.files.iter().map(|f| f.bytes).sum::<usize>()
    }

    /// Where one of its files is fetched from.
    ///
    /// The brackets in a variable font's name — `NotoSerifTamil[wdth,wght].ttf`
    /// — are not allowed in a URL path, and a client that validates its input
    /// rejects them rather than guessing, so they are escaped here. Nothing
    /// else in a Google Fonts file name needs it.
    pub fn url(&self, file: &str) -> String {
        format!(
            "{HOST}/{REPOSITORY}/{REVISION}/ofl/{}/{}",
            self.directory,
            file.replace('[', "%5B").replace(']', "%5D"),
        )
    }

    /// Where its licence is fetched from.
    pub fn licence_url(&self) -> String {
        self.url(LICENCE_FILE)
    }

    /// What the licence is called inside the project.
    ///
    /// Named after the family, because `assets/fonts/` holds more than one and
    /// a second `OFL.txt` would overwrite the first — which is how a project
    /// ends up shipping fonts under a licence file that does not cover them.
    /// The convention is the one `src/assets/fonts/` already uses for the
    /// window's own faces.
    pub fn licence_name(&self) -> String {
        format!("OFL-{}.txt", self.family.replace(' ', ""))
    }
}

/// The licence file's name upstream. The same in every family's directory.
const LICENCE_FILE: &str = "OFL.txt";

/// What the window needs in order to offer a font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub family: &'static str,
    pub note: &'static str,
    /// The script it is being suggested for, by display name.
    pub script: &'static str,
    pub faces: Vec<Face>,
    pub bytes: usize,
    /// Whether `assets/fonts/` already holds every file of it.
    ///
    /// Checked by name on disk rather than through `fontdb`, because the
    /// question is "has this project got these files" and not "what can this
    /// machine resolve" — a family installed system-wide is not a family the
    /// book carries.
    pub packaged: bool,
}

/// Which scripts a language tag is written in.
///
/// The primary subtag only: `pt-BR` is Latin because `pt` is, and a region or
/// a variant has never changed a script. A tag that *names* its script —
/// `mni-Beng` — is matched whole first, which is why that one is in the table.
pub fn scripts_for_language(tag: &str) -> Vec<&'static Script> {
    let tag = tag.trim().to_lowercase();
    if tag.is_empty() {
        return Vec::new();
    }

    let whole: Vec<&Script> = SCRIPTS
        .iter()
        .filter(|s| s.languages.iter().any(|l| *l == tag))
        .collect();
    if !whole.is_empty() {
        return whole;
    }

    let primary = tag.split('-').next().unwrap_or(&tag);
    SCRIPTS
        .iter()
        .filter(|s| s.languages.contains(&primary))
        .collect()
}

/// Which scripts a set of characters is written in.
///
/// Every character, because the question is presence: a script used in one
/// verse is still a script the font has to draw.
pub fn scripts_in(characters: &BTreeSet<char>) -> Vec<&'static Script> {
    SCRIPTS
        .iter()
        .filter(|script| {
            characters.iter().any(|c| {
                let code = *c as u32;
                script
                    .ranges
                    .iter()
                    .any(|(from, to)| code >= *from && code <= *to)
            })
        })
        .collect()
}

/// What to offer, for this language and this Scripture.
///
/// The characters win where there are any: a project's tag is what somebody
/// typed when they made the folder, and the text is what they have since put
/// in it. Where the two disagree the text is right, and the case is not
/// hypothetical — a translation drafted in one project folder and continued in
/// another carries the first one's tag.
///
/// Two scripts are then put aside, because a list of fonts is a decision and
/// every row that is not a real option makes the decision harder.
///
/// **Latin, once anything else is in play.** Almost every Scripture has Latin
/// in it — a book code, a copyright line, a transliterated name — so keeping
/// it would push the four families that can set a Tamil Bible below three that
/// cannot.
///
/// **A script another one subsumes.** Japanese and Korean carry Han
/// characters, so a Japanese Gospel matches two rows and needs one font.
pub fn suggest(
    language: &str,
    characters: &BTreeSet<char>,
    project_root: Option<&Utf8Path>,
) -> Vec<Suggestion> {
    let mut scripts = scripts_in(characters);
    if scripts.is_empty() {
        scripts = scripts_for_language(language);
    }

    // Latin first, so that Chinese with English book names — two scripts, both
    // of which the next rule would otherwise leave standing — comes out as
    // Han alone rather than as Latin followed by Han.
    if scripts.len() > 1 {
        scripts.retain(|s| s.id != "latn");
    }
    let present: Vec<&str> = scripts.iter().map(|s| s.id).collect();
    scripts.retain(|s| !s.subsumed_by.iter().any(|id| present.contains(id)));

    let mut out = Vec::new();
    for script in scripts {
        for typeface in TYPEFACES.iter().filter(|t| t.scripts.contains(&script.id)) {
            out.push(Suggestion {
                family: typeface.family,
                note: typeface.note,
                script: script.name,
                faces: typeface.faces(),
                bytes: typeface.bytes(),
                packaged: project_root.is_some_and(|root| is_packaged(typeface, root)),
            });
        }
    }
    out
}

/// The catalogue entry for a family, by name.
pub fn typeface(family: &str) -> Option<&'static Typeface> {
    TYPEFACES.iter().find(|t| t.family == family)
}

/// Whether the project already holds every file of a family.
fn is_packaged(typeface: &Typeface, project_root: &Utf8Path) -> bool {
    let dir = project_root.join(PROJECT_FONTS);
    typeface.files.iter().all(|f| dir.join(f.name).is_file())
}

/// One file, fetched.
///
/// Carries the name it was asked for so [`package`] can pair bytes with the
/// catalogue row rather than trusting the order they arrive in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// What was written, and what the publisher should know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packaged {
    pub family: String,
    /// The files now in `assets/fonts/`, by name, in catalogue order.
    pub files: Vec<String>,
    pub licence: String,
    /// The faces a build can select. Short of four means a style asking for
    /// one of the others will be set in the nearest the family has (FONT-005).
    pub faces: Vec<Face>,
}

/// Write a fetched family into the project, having checked every byte of it.
///
/// `fetched` must hold the licence and every file the catalogue lists, named
/// as the catalogue names them. Anything missing, anything extra, or anything
/// that fails a check and **nothing is written at all** — a half-packaged
/// family is a project that resolves a font whose bold is not there, which is
/// a worse place to be than one that resolved nothing.
pub fn package(
    typeface: &Typeface,
    fetched: &[Fetched],
    project_root: &Utf8Path,
) -> Result<Packaged, Diagnostic> {
    let find = |name: &str| {
        fetched.iter().find(|f| f.name == name).ok_or_else(|| {
            Diagnostic::error(
                code::UNRESOLVED,
                format!("{} was not downloaded with {}", name, typeface.family),
            )
            .help("the download was incomplete; nothing has been written to the project")
        })
    };

    // Checked in full before the first byte reaches the disk.
    let licence = find(LICENCE_FILE)?;
    verify_licence(typeface, &licence.bytes)?;

    let mut verified: Vec<(&SourceFile, &[u8])> = Vec::new();
    for file in typeface.files {
        let found = find(file.name)?;
        verify(typeface, file, &found.bytes)?;
        verified.push((file, &found.bytes));
    }

    let dir = project_root.join(PROJECT_FONTS);
    std::fs::create_dir_all(dir.as_std_path()).map_err(|e| {
        Diagnostic::error(
            code::UNRESOLVED,
            format!("could not make {PROJECT_FONTS}/ in the project"),
        )
        .at(SourceLoc::file(dir.clone()))
        .detail(e.to_string())
    })?;

    let licence_name = typeface.licence_name();
    write(&dir.join(&licence_name), &licence.bytes)?;
    let mut written = Vec::new();
    for (file, bytes) in verified {
        write(&dir.join(file.name), bytes)?;
        written.push(file.name.to_owned());
    }

    Ok(Packaged {
        family: typeface.family.to_owned(),
        files: written,
        licence: licence_name,
        faces: typeface.faces(),
    })
}

/// Write one file, replacing whatever was there.
///
/// Through a temporary beside it and a rename, so a font half-written by an
/// interrupted download does not become a font the next build tries to read.
/// The rename is on the same directory, which is what makes it atomic.
fn write(path: &Utf8Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    let failed = |e: std::io::Error| {
        Diagnostic::error(
            code::UNRESOLVED,
            format!(
                "could not write {}",
                path.file_name().unwrap_or(path.as_str())
            ),
        )
        .at(SourceLoc::file(path.to_owned()))
        .detail(e.to_string())
    };

    let partial = Utf8PathBuf::from(format!("{path}.part"));
    std::fs::write(partial.as_std_path(), bytes).map_err(failed)?;
    // Windows will not rename onto an existing file.
    let _ = std::fs::remove_file(path.as_std_path());
    std::fs::rename(partial.as_std_path(), path.as_std_path()).map_err(|e| {
        let _ = std::fs::remove_file(partial.as_std_path());
        failed(e)
    })
}

/// Lowercase hex, which is how the catalogue spells a digest.
fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut s, byte| {
            use std::fmt::Write;
            let _ = write!(s, "{byte:02x}");
            s
        })
}

fn verify_licence(typeface: &Typeface, bytes: &[u8]) -> Result<(), Diagnostic> {
    if bytes.len() != typeface.licence.bytes || digest(bytes) != typeface.licence.sha256 {
        return Err(tampered(typeface, LICENCE_FILE));
    }
    // The licence is the reason redistribution is permitted at all, so it is
    // worth one assertion that the thing being shipped beside the font is
    // actually a licence and not a repository's README.
    if !String::from_utf8_lossy(bytes).contains("SIL OPEN FONT LICENSE") {
        return Err(Diagnostic::error(
            code::UNRESOLVED,
            format!(
                "what was downloaded as {}'s licence is not the Open Font License",
                typeface.family
            ),
        )
        .help("nothing has been written to the project"));
    }
    Ok(())
}

/// Every check one font file has to pass, in the order that gives the clearest
/// answer when it fails.
///
/// Length first because a truncated or redirected download fails it and the
/// message can say so plainly. Then the digest. Only then the parse, the name
/// and the glyph — which are the checks that catch a catalogue row that is
/// wrong rather than a download that went astray, and which would otherwise
/// report a mirror's error page as a font with an odd family name.
fn verify(typeface: &Typeface, file: &SourceFile, bytes: &[u8]) -> Result<(), Diagnostic> {
    if bytes.len() != file.bytes {
        return Err(Diagnostic::error(
            code::UNRESOLVED,
            format!(
                "{} came to {} bytes rather than the {} expected",
                file.name,
                bytes.len(),
                file.bytes,
            ),
        )
        .help(
            "the download was interrupted or redirected; nothing has been written to the project",
        ));
    }
    if digest(bytes) != file.sha256 {
        return Err(tampered(typeface, file.name));
    }

    let face = ttf_parser::Face::parse(bytes, 0).map_err(|e| {
        Diagnostic::error(
            code::UNRESOLVED,
            format!("{} is not a font this release can read", file.name),
        )
        .help("nothing has been written to the project")
        .detail(e.to_string())
    })?;

    // Name 16 where the font has one, name 1 otherwise, which is the order a
    // resolver reads them in. A variable font whose default instance is not
    // Regular puts the instance's name in 1 and the family's in 16, so
    // accepting either is what makes the comparison mean "same family".
    let named = face.names().into_iter().any(|name| {
        matches!(name.name_id, 1 | 16) && name.to_string().as_deref() == Some(typeface.family)
    });
    if !named {
        return Err(Diagnostic::error(
            code::UNRESOLVED,
            format!("{} does not call itself {}", file.name, typeface.family),
        )
        .help(
            "the catalogue and the file disagree about the family name, so a build would \
             not resolve it — nothing has been written to the project",
        ));
    }

    for script in script_set(typeface) {
        if face.glyph_index(script.sample).is_none() {
            return Err(Diagnostic::error(
                code::MISSING_COVERAGE,
                format!(
                    "{} has no glyph for {} ({}), which it was offered for",
                    file.name, script.name, script.sample,
                ),
            )
            .help("nothing has been written to the project"));
        }
    }

    Ok(())
}

/// The scripts a catalogue entry claims, as rows.
fn script_set(typeface: &Typeface) -> impl Iterator<Item = &'static Script> + '_ {
    SCRIPTS.iter().filter(|s| typeface.scripts.contains(&s.id))
}

/// A file that is not the file the catalogue describes.
///
/// One message for a length that matched and a digest that did not, because
/// the two causes — a mirror serving something else, and a catalogue row that
/// was mistyped — are indistinguishable from here and a publisher's action is
/// the same either way.
fn tampered(typeface: &Typeface, name: &str) -> Diagnostic {
    Diagnostic::error(
        code::UNRESOLVED,
        format!("{name} is not the file {} was expecting", typeface.family),
    )
    .help(
        "nothing has been written to the project. The catalogue records the checksum of \
         every file at one pinned revision, so this means the download did not arrive \
         intact — try again, and report it if it persists",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A catalogue entry nobody can reach is a font nobody is ever offered.
    #[test]
    fn every_typeface_names_a_script_in_the_table() {
        for typeface in TYPEFACES {
            assert!(
                !typeface.scripts.is_empty(),
                "{} claims no script",
                typeface.family
            );
            for id in typeface.scripts {
                assert!(
                    SCRIPTS.iter().any(|s| s.id == *id),
                    "{} claims {id}, which is not a script in the table",
                    typeface.family,
                );
            }
        }
    }

    /// A script with nothing to offer makes the window promise a list and then
    /// show an empty one.
    #[test]
    fn every_script_has_something_to_offer() {
        for script in SCRIPTS {
            assert!(
                TYPEFACES.iter().any(|t| t.scripts.contains(&script.id)),
                "nothing in the catalogue covers {}",
                script.name,
            );
        }
    }

    /// Two rows with one family name would make `typeface()` return whichever
    /// came first and the other unreachable.
    #[test]
    fn families_and_file_names_are_unique() {
        let mut families = BTreeSet::new();
        for typeface in TYPEFACES {
            assert!(
                families.insert(typeface.family),
                "{} is listed twice",
                typeface.family
            );

            let mut names = BTreeSet::new();
            for file in typeface.files {
                assert!(
                    names.insert(file.name),
                    "{} lists {} twice",
                    typeface.family,
                    file.name,
                );
            }
            assert!(
                !typeface.files.is_empty(),
                "{} has no files",
                typeface.family
            );
        }
    }

    /// Every family needs a regular: it is the face the body text is set in,
    /// and a family that is only a bold is not a body font.
    #[test]
    fn every_family_has_a_regular() {
        for typeface in TYPEFACES {
            assert!(
                typeface.faces().contains(&Face::Regular),
                "{} has no regular face",
                typeface.family,
            );
        }
    }

    /// A digest the generator did not write would fail every download with a
    /// message about tampering, which is the worst way to learn of a typo.
    #[test]
    fn every_digest_is_lowercase_hex() {
        let ok = |sha: &str| {
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        };
        for typeface in TYPEFACES {
            assert!(
                ok(typeface.licence.sha256),
                "{}'s licence digest",
                typeface.family
            );
            for file in typeface.files {
                assert!(ok(file.sha256), "{} {}", typeface.family, file.name);
                assert!(file.bytes > 0, "{} {} is empty", typeface.family, file.name);
            }
        }
    }

    /// Ranges that run backwards match nothing, and a sample outside its own
    /// script's ranges would mean the probe and the detection disagree.
    #[test]
    fn script_ranges_hold_their_own_sample() {
        for script in SCRIPTS {
            assert!(!script.ranges.is_empty(), "{} has no ranges", script.name);
            for (from, to) in script.ranges {
                assert!(from <= to, "{} has a backwards range", script.name);
            }
            let code = script.sample as u32;
            assert!(
                script
                    .ranges
                    .iter()
                    .any(|(from, to)| code >= *from && code <= *to),
                "{}'s sample {} is outside its own ranges",
                script.name,
                script.sample,
            );
        }
    }

    /// A script subsumed by one that is not in the table would never step
    /// aside, and the rule would look as though it were working.
    #[test]
    fn every_subsuming_script_is_in_the_table() {
        for script in SCRIPTS {
            for id in script.subsumed_by {
                assert!(
                    SCRIPTS.iter().any(|s| s.id == *id),
                    "{} says it is subsumed by {id}, which is not a script in the table",
                    script.name,
                );
                assert_ne!(*id, script.id, "{} subsumes itself", script.name);
            }
        }
    }

    /// Japanese and Korean carry Han characters, so the Han row must step
    /// aside for them — otherwise a Japanese Gospel is offered a Japanese face
    /// and then a list of Chinese ones, as though that were a second question.
    #[test]
    fn han_steps_aside_for_japanese_and_korean() {
        let japanese: BTreeSet<char> = "はじめに言があった".chars().collect();
        let suggestions = suggest("ja", &japanese, None);
        assert!(!suggestions.is_empty());
        assert!(
            suggestions.iter().all(|s| s.script == "Japanese"),
            "{:?}",
            suggestions
                .iter()
                .map(|s| (s.family, s.script))
                .collect::<Vec<_>>(),
        );
        assert_eq!(suggestions[0].family, "Shippori Mincho");

        let korean: BTreeSet<char> = "태초에 말씀이".chars().collect();
        let korean = suggest("ko", &korean, None);
        assert!(korean.iter().all(|s| s.script == "Korean"));

        // Chinese is Han and stays Han, even with the Latin a book code puts
        // in every file.
        let chinese: BTreeSet<char> = "太初有道 (JHN 1:1)".chars().collect();
        let chinese = suggest("zh", &chinese, None);
        assert!(!chinese.is_empty());
        assert!(chinese.iter().all(|s| s.script == "Han"), "{chinese:?}");
    }

    /// One tag belonging to two scripts would offer both families and leave the
    /// publisher to guess, so the table must not say a language is written in
    /// two scripts unless it genuinely is.
    #[test]
    fn a_language_tag_leads_to_one_script() {
        let mut seen: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
        for script in SCRIPTS {
            for tag in script.languages {
                seen.entry(tag).or_default().push(script.name);
            }
        }
        for (tag, scripts) in seen {
            assert_eq!(scripts.len(), 1, "{tag} is listed under {scripts:?}");
        }
    }

    #[test]
    fn a_language_tag_finds_its_script() {
        let only = |tag: &str| {
            let found = scripts_for_language(tag);
            assert_eq!(found.len(), 1, "{tag} found {found:?}");
            found[0].name
        };
        assert_eq!(only("ta"), "Tamil");
        // A region subtag does not change the script.
        assert_eq!(only("pt-BR"), "Latin");
        assert_eq!(only("  HI  "), "Devanagari");
        // A tag that names its own script is matched whole, before the primary
        // subtag sends it to the wrong table row.
        assert_eq!(only("mni-Beng"), "Bengali");
        assert_eq!(only("mni"), "Meitei Mayek");
        assert!(scripts_for_language("").is_empty());
        assert!(scripts_for_language("zxx").is_empty());
    }

    #[test]
    fn the_text_outranks_the_tag() {
        // A project tagged Hindi whose Scripture is Tamil.
        let characters: BTreeSet<char> = "ஆதியிலே".chars().collect();
        let suggestions = suggest("hi", &characters, None);

        assert!(!suggestions.is_empty());
        assert!(
            suggestions.iter().all(|s| s.script == "Tamil"),
            "{:?}",
            suggestions.iter().map(|s| s.script).collect::<Vec<_>>(),
        );
    }

    /// Nearly every Scripture has a Latin character somewhere in it, so a
    /// suggestion list that keeps Latin buries the families that matter.
    #[test]
    fn latin_steps_aside_for_the_script_that_needs_a_font() {
        let characters: BTreeSet<char> = "ஆதியிலே (Genesis 1:1)".chars().collect();
        let suggestions = suggest("ta", &characters, None);

        assert!(suggestions.iter().all(|s| s.script == "Tamil"));
        // And a Latin-only project still gets the Latin faces.
        let latin: BTreeSet<char> = "In the beginning".chars().collect();
        assert!(suggest("en", &latin, None)
            .iter()
            .all(|s| s.script == "Latin"));
    }

    #[test]
    fn with_no_text_the_tag_decides() {
        let suggestions = suggest("am", &BTreeSet::new(), None);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| s.script == "Ethiopic"));
        assert_eq!(suggestions[0].family, "Abyssinica SIL");
    }

    #[test]
    fn nothing_known_suggests_nothing() {
        assert!(suggest("", &BTreeSet::new(), None).is_empty());
        assert!(suggest("zxx", &BTreeSet::new(), None).is_empty());
    }

    /// The SIL faces are offered before the Noto that merely covers the
    /// script, because they ship a drawn bold rather than a weight axis — and
    /// a project font is pinned to one instance.
    #[test]
    fn the_scripture_faces_come_first() {
        let first = |tag: &str| suggest(tag, &BTreeSet::new(), None)[0].family;
        assert_eq!(first("en"), "Charis SIL");
        assert_eq!(first("ar"), "Scheherazade New");
        assert_eq!(first("hi"), "Annapurna SIL");
        assert_eq!(first("my"), "Padauk");
    }

    /// What the window says about bold and italic before anything is fetched.
    #[test]
    fn a_variable_family_offers_one_face_and_says_so() {
        let charis = typeface("Charis SIL").expect("Charis SIL is in the catalogue");
        assert_eq!(
            charis.faces(),
            vec![Face::Regular, Face::Bold, Face::Italic, Face::BoldItalic],
        );
        assert!(charis.files.iter().all(|f| !f.variable));

        // One variable file and one variable italic: a build can select those
        // two and nothing else, however many weights the axis holds.
        let tamil = typeface("Noto Serif Tamil").expect("Noto Serif Tamil is in the catalogue");
        assert_eq!(tamil.faces(), vec![Face::Regular, Face::Italic]);
        assert!(tamil.files.iter().all(|f| f.variable));
    }

    #[test]
    fn a_url_escapes_the_brackets_a_variable_font_is_named_with() {
        let tamil = typeface("Noto Serif Tamil").expect("Noto Serif Tamil is in the catalogue");
        let url = tamil.url(tamil.files[0].name);

        assert!(url.starts_with(&format!(
            "{HOST}/{REPOSITORY}/{REVISION}/ofl/notoseriftamil/"
        )));
        assert!(!url.contains('['), "{url}");
        assert!(url.contains("%5Bwdth,wght%5D"), "{url}");
        assert!(tamil.licence_url().ends_with("/OFL.txt"));
    }

    /// Two families in one project must not land on one licence file.
    #[test]
    fn a_licence_is_named_after_its_family() {
        assert_eq!(
            typeface("Charis SIL").unwrap().licence_name(),
            "OFL-CharisSIL.txt",
        );
        assert_eq!(
            typeface("Noto Serif Tamil").unwrap().licence_name(),
            "OFL-NotoSerifTamil.txt",
        );
    }
}
