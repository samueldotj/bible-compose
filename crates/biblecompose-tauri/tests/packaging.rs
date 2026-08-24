//! What the installer says about itself has to be true (P6.1).
//!
//! Every claim here is one that a build cannot check and a release depends on:
//! a version in two files, a signing field that must stay empty in the
//! repository, and a bundle target list that must stay explicit. None of them
//! fails a compile, and all of them fail a publisher.

use biblecompose_testkit::repo_root;

fn config() -> String {
    std::fs::read_to_string(
        repo_root()
            .join("crates/biblecompose-tauri/tauri.conf.json")
            .as_std_path(),
    )
    .expect("the Tauri configuration is readable")
}

/// One value from a flat JSON object, read as text.
///
/// Text rather than parsed, for the same reason the offline test reads the
/// content-security policy that way: a JSON dependency in a test crate to
/// reach four keys is a poor trade, and the file is written by a script that
/// formats it predictably.
fn field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let at = text.find(&needle)? + needle.len();
    let rest = text[at..].trim_start();
    if let Some(body) = rest.strip_prefix('"') {
        return body.find('"').map(|end| body[..end].to_owned());
    }
    // A bare value — `null`, a number, `true`.
    Some(
        rest.chars()
            .take_while(|c| !matches!(c, ',' | '\n' | '}'))
            .collect::<String>()
            .trim()
            .to_owned(),
    )
}

/// **The two version numbers agree.**
///
/// The crate's version is what `--version` prints and what the build stamps
/// into a fingerprint; the bundle's is what the installer registers with the
/// operating system and what an upgrade compares against. A release where they
/// disagree installs over itself, or refuses to.
#[test]
fn the_crate_and_the_bundle_are_the_same_version() {
    let manifest = std::fs::read_to_string(
        repo_root()
            .join("crates/biblecompose-tauri/Cargo.toml")
            .as_std_path(),
    )
    .expect("the manifest is readable");
    let crate_version = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("version = "))
        .map(|v| v.trim().trim_matches('"').to_owned())
        .expect("the crate declares a version");

    assert_eq!(
        field(&config(), "version").as_deref(),
        Some(crate_version.as_str()),
        "Cargo.toml says {crate_version} and tauri.conf.json says otherwise"
    );
}

/// **No signing credential is in the repository**, and the fields that would
/// hold one are empty.
///
/// The identity is supplied by a secret at build time and read from the
/// environment. A thumbprint or an identity committed here would be a
/// credential in version control — and, worse, one that looks like
/// configuration.
#[test]
fn nothing_here_is_a_signing_credential() {
    let text = config();
    assert_eq!(
        field(&text, "signingIdentity").as_deref(),
        Some("null"),
        "a signing identity is committed; it belongs in a secret"
    );
    for forbidden in [
        "certificateThumbprint",
        "APPLE_CERTIFICATE",
        "-----BEGIN",
        "MIIK",
    ] {
        assert!(
            !text.contains(forbidden),
            "{forbidden} appears in the Tauri configuration"
        );
    }
}

/// **The bundle targets are named**, not `all`.
///
/// `all` means whatever this Tauri release supports on this runner, which is a
/// list that changes underneath a project. A release that quietly stopped
/// producing an `.msi` would look exactly like a successful build.
#[test]
fn the_bundle_targets_are_written_out() {
    let text = config();
    let at = text.find("\"targets\":").expect("targets are configured");
    let rest = &text[at..];
    assert!(
        !rest.starts_with("\"targets\": \"all\""),
        "the target list is `all`, which is whatever today's Tauri decides"
    );
    for target in ["msi", "nsis", "dmg", "deb", "appimage"] {
        assert!(
            text.contains(&format!("\"{target}\"")),
            "no {target} in the target list — every platform NFR-001 names \
             needs one installer and one portable form"
        );
    }
}

/// The Linux package depends on nothing, because the typesetter is inside the
/// executable (ADR-006). A `.deb` naming SILE would be naming something it
/// does not use, and would fail to install where the distribution has no such
/// package — which is most of them.
#[test]
fn the_linux_package_declares_no_dependencies() {
    let text = config();
    let at = text
        .find("\"depends\":")
        .expect("deb dependencies are configured");
    let rest = text[at..].trim_start_matches("\"depends\":").trim_start();
    assert!(
        rest.starts_with("[]"),
        "the .deb declares dependencies: {}",
        &rest[..rest.len().min(60)]
    );
}
