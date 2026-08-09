//! The architectural rules, asserted in CI.
//!
//! ADR-004 keeps the backend boundary thin with two rules, and notes that this
//! is "the kind of rule that erodes one convenient import at a time". So they
//! are checked rather than reviewed. P0.1.

use std::collections::BTreeMap;

use biblecompose_testkit::repo_root;

/// crate name → the crates it depends on (normal and dev, separately).
struct Manifest {
    name: String,
    deps: Vec<String>,
    dev_deps: Vec<String>,
}

fn manifests() -> Vec<Manifest> {
    let crates_dir = repo_root().join("crates");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(crates_dir.as_std_path()).expect("crates/ exists") {
        let dir = entry.expect("readable entry").path();
        let manifest = dir.join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest).expect("readable manifest");
        let value: toml::Value = text.parse().expect("valid TOML");

        let name = value["package"]["name"]
            .as_str()
            .expect("every package is named")
            .to_owned();

        out.push(Manifest {
            name,
            deps: bible_deps(&value, "dependencies"),
            dev_deps: bible_deps(&value, "dev-dependencies"),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(!out.is_empty(), "found no crates to check");
    out
}

fn bible_deps(value: &toml::Value, table: &str) -> Vec<String> {
    value
        .get(table)
        .and_then(toml::Value::as_table)
        .map(|t| {
            t.keys()
                .filter(|k| k.starts_with("biblecompose-"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// ADR-004: "biblecompose-sile is not a dependency of anything except
/// biblecompose-app."
#[test]
fn only_the_app_depends_on_the_backend() {
    let offenders: Vec<&str> = manifests()
        .iter()
        .filter(|m| m.name != "biblecompose-app" && m.deps.iter().any(|d| d == "biblecompose-sile"))
        .map(|m| m.name.as_str())
        .map(str::to_owned)
        .collect::<Vec<String>>()
        .leak()
        .iter()
        .map(String::as_str)
        .collect();

    assert!(
        offenders.is_empty(),
        "ADR-004: only biblecompose-app may depend on biblecompose-sile, but {offenders:?} do.\n\
         If a crate needs typesetting, it needs to ask the app for it."
    );
}

/// The test kit is consumed by the backend's own tests, so depending back
/// would be a cycle as well as a rule break.
#[test]
fn the_testkit_does_not_depend_on_the_backend() {
    let tk = manifests()
        .into_iter()
        .find(|m| m.name == "biblecompose-testkit")
        .expect("the test kit exists");
    assert!(
        !tk.deps.contains(&"biblecompose-sile".to_owned())
            && !tk.dev_deps.contains(&"biblecompose-sile".to_owned()),
        "biblecompose-testkit must not depend on biblecompose-sile, in either table"
    );
}

/// NFR-009: the core is testable without a GUI. There is no GUI crate yet —
/// it arrives at P2.1 — so this asserts the shape now, while it is free, and
/// starts failing the day someone wires a window into the CLI.
#[test]
fn the_cli_links_no_gui_crate() {
    let cli = manifests()
        .into_iter()
        .find(|m| m.name == "biblecompose-cli")
        .expect("the CLI exists");
    for d in cli.deps.iter().chain(cli.dev_deps.iter()) {
        assert!(
            !d.contains("gui") && !d.contains("tauri"),
            "NFR-009: the CLI must run headless, but it depends on {d}"
        );
    }
}

/// Nothing below the app orchestrates. `biblecompose-core` from SRS §12.1 was
/// dropped because its job had one caller; this stops it growing back.
#[test]
fn no_crate_depends_on_the_app_except_the_cli() {
    let allowed = ["biblecompose-cli", "biblecompose-tauri"];
    let offenders: Vec<String> = manifests()
        .iter()
        .filter(|m| !allowed.contains(&m.name.as_str()))
        .filter(|m| m.deps.iter().any(|d| d == "biblecompose-app"))
        .map(|m| m.name.clone())
        .collect();
    assert!(
        offenders.is_empty(),
        "only {allowed:?} may depend on biblecompose-app, but {offenders:?} do"
    );
}

/// Every crate in the workspace is one the architecture actually names.
#[test]
fn the_workspace_is_the_eight_crates_plus_the_test_kit() {
    let names: Vec<String> = manifests().into_iter().map(|m| m.name).collect();
    let expected = [
        "biblecompose-app",
        "biblecompose-cli",
        "biblecompose-config",
        "biblecompose-diagnostics",
        "biblecompose-project",
        "biblecompose-scripture",
        "biblecompose-sile",
        "biblecompose-tauri",
        "biblecompose-testkit",
    ];
    assert_eq!(
        names,
        expected.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        "ARCHITECTURE §3 lists eight crates plus a test kit"
    );
}

/// A dependency cycle would not compile, but a *layering* inversion might —
/// diagnostics is the bottom of the stack and must stay there.
#[test]
fn diagnostics_depends_on_nothing_of_ours() {
    let d = manifests()
        .into_iter()
        .find(|m| m.name == "biblecompose-diagnostics")
        .expect("the diagnostics crate exists");
    assert!(
        d.deps.is_empty(),
        "biblecompose-diagnostics is used by every layer and must depend on none: {:?}",
        d.deps
    );
}

/// A readable summary when something above fails.
#[test]
fn print_the_dependency_graph() {
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for m in manifests() {
        graph.insert(m.name, m.deps);
    }
    for (name, deps) in &graph {
        println!("{name} → {deps:?}");
    }
    assert_eq!(graph.len(), 9);
}

/// SILE-005 / DET-001: no `HashMap` or `HashSet` on the emission path.
///
/// Rust randomises hash iteration order per process, so one of these in
/// configuration resolution, style resolution or emission makes the golden
/// tests fail on roughly one machine in three — intermittently and
/// unreproducibly, which SRS-REVIEW F4 calls the worst available failure mode.
///
/// The golden files would eventually catch it. This catches it at the point of
/// writing, with a message that says why.
#[test]
fn the_emission_path_uses_ordered_maps_only() {
    // Crates whose output must be byte-reproducible.
    let on_path = [
        "biblecompose-scripture",
        "biblecompose-config",
        "biblecompose-sile",
    ];

    let mut offenders = Vec::new();
    for crate_name in on_path {
        let src = repo_root().join("crates").join(crate_name).join("src");
        for file in rust_files(&src) {
            let text = std::fs::read_to_string(file.as_std_path()).expect("readable source");
            for (n, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains("HashMap") || code.contains("HashSet") {
                    offenders.push(format!("{}:{}: {}", file, n + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "unordered collections on the emission path:\n  {}\n\n\
         Use BTreeMap or IndexMap. Rust randomises hash iteration order per \
         process, so this makes SILE-005 fail intermittently rather than \
         reliably.",
        offenders.join("\n  ")
    );
}

fn rust_files(dir: &camino::Utf8Path) -> Vec<camino::Utf8PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir.as_std_path()) else {
        return out;
    };
    for e in entries.flatten() {
        let path = camino::Utf8PathBuf::from_path_buf(e.path()).expect("UTF-8 path");
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension() == Some("rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}
