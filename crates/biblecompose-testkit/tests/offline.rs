//! NFR-004, as a property of the dependency graph rather than a promise.
//!
//! "BibleCompose shall not require an internet connection for project load,
//! validation, composition, or PDF generation." The obvious way to check that
//! is to pull the plug and build something, which proves it on one machine on
//! one day. This proves it about the code: **nothing under the shell can open
//! a socket, because nothing under the shell links anything that could.**
//!
//! # Why the shell is exempt
//!
//! Two reasons now, and the second one arrived deliberately.
//!
//! `tauri` depends on `reqwest`, and there is nothing to be done about that —
//! it is how the framework serves the window's own assets. So the rule is
//! drawn where it can be held: the CLI and every crate the build pipeline is
//! made of are clean, and the window is fenced by its content-security policy
//! and its capability list instead, both of which are asserted below.
//!
//! And since FONT-006 the shell has one HTTP client of its own, `ureq`, for
//! fetching a font a publisher asked for by name (ARCHITECTURE §7.6). That is
//! the only request this application can make, and putting it *here* rather
//! than in `biblecompose-app` is the whole reason the test above still passes:
//! the crate that composes a page has no client linked into it, so "a build
//! needs no network" stays a property of the graph. The tests below hold that
//! arrangement in place — one client, named, reached from one module.
//!
//! And since ADR-008 there is a second request, "Check for updates", which
//! goes through Tauri's updater plugin and so through the framework's own
//! `reqwest` rather than a client of ours. It is held the same way: named in
//! one module, which pins the address it asks, and granted to no window.
//!
//! Read from `Cargo.lock`, which is committed, so this is a fact about the
//! versions actually built and not about what a resolver might pick.

use std::collections::{BTreeMap, BTreeSet};

use biblecompose_testkit::repo_root;

/// Crates whose presence means something in the tree can make a request.
///
/// Transport, not convenience: `url` parses and does not fetch, and excluding
/// it keeps this list about the capability rather than about the vocabulary.
const NETWORKING: &[&str] = &[
    "reqwest",
    "hyper",
    "hyper-util",
    "h2",
    "ureq",
    "curl",
    "isahc",
    "attohttpc",
    "surf",
    "tungstenite",
    "tokio-tungstenite",
    "quinn",
    "trust-dns-resolver",
    "hickory-resolver",
];

/// The one crate allowed to reach them, and why.
const THE_SHELL: &str = "biblecompose-tauri";

/// package name → its dependencies, from the lock file.
fn locked() -> BTreeMap<String, Vec<String>> {
    let text = std::fs::read_to_string(repo_root().join("Cargo.lock").as_std_path())
        .expect("Cargo.lock is committed");
    let value: toml::Value = text.parse().expect("valid TOML");
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in value["package"].as_array().expect("a package array") {
        let name = package["name"].as_str().expect("a name").to_owned();
        // A dependency is `name` or `name version`; only the name matters.
        let deps: Vec<String> = package
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .map(|d| {
                d.iter()
                    .filter_map(toml::Value::as_str)
                    .map(|s| s.split_whitespace().next().unwrap_or(s).to_owned())
                    .collect()
            })
            .unwrap_or_default();
        // Two versions of one crate appear twice; the union of their
        // dependencies is the safe reading for a reachability question.
        out.entry(name).or_default().extend(deps);
    }
    out
}

fn reachable_from(graph: &BTreeMap<String, Vec<String>>, root: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![root.to_owned()];
    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        for dep in graph.get(&name).into_iter().flatten() {
            stack.push(dep.clone());
        }
    }
    seen
}

/// **Nothing that loads, validates, composes or publishes can make a request.**
#[test]
fn the_pipeline_cannot_reach_the_network() {
    let graph = locked();
    let ours: Vec<String> = graph
        .keys()
        .filter(|k| k.starts_with("biblecompose-") && *k != THE_SHELL)
        .cloned()
        .collect();
    assert!(ours.len() >= 6, "expected the workspace, found {ours:?}");

    for crate_name in ours {
        let reachable = reachable_from(&graph, &crate_name);
        let found: Vec<&str> = NETWORKING
            .iter()
            .copied()
            .filter(|n| reachable.contains(*n))
            .collect();
        assert!(
            found.is_empty(),
            "{crate_name} can reach {found:?} — NFR-004 says a build needs no \
             network, and a crate that links an HTTP client is one refactor \
             away from using it"
        );
    }
}

/// The one client the shell carries on purpose (FONT-006).
const THE_CLIENT: &str = "ureq";

/// The exemption is real, and narrow: only the shell, only one client of its
/// own, and only for fetching a font somebody asked for.
///
/// Written as an exact comparison rather than a ban, so that adding a second
/// client fails here and has to be argued for. A general-purpose fetcher is
/// not something that should be able to arrive in this application by being
/// convenient for one feature.
#[test]
fn the_shell_carries_exactly_one_client_of_its_own() {
    let graph = locked();
    let direct = graph.get(THE_SHELL).cloned().unwrap_or_default();

    let own: Vec<&str> = direct
        .iter()
        .map(String::as_str)
        .filter(|d| NETWORKING.contains(d))
        .collect();
    assert_eq!(
        own,
        vec![THE_CLIENT],
        "the shell may depend on {THE_CLIENT} directly and on nothing else that \
         can open a socket — FONT-006 is one request to one host, and a second \
         client means somebody added a second reason"
    );

    // And the framework's own, still reached and still not ours to remove.
    let reachable = reachable_from(&graph, THE_SHELL);
    assert!(
        reachable.contains("reqwest"),
        "if the shell no longer reaches the framework's client, say so here"
    );
}

/// **One egress point.** The client is named in one module of the shell, and
/// that module checks the host before it connects.
///
/// The dependency graph says the shell *may* make a request; this says where
/// from. Without it, the rule degrades the first time somebody reaches for
/// `ureq::get` in a command because it was two lines shorter than routing
/// through `download`, and the host check — the thing that makes "one host"
/// true — would be bypassed with nothing to notice it.
#[test]
fn the_client_is_reached_from_one_module() {
    let src = repo_root().join("crates/biblecompose-tauri/src");
    let mut users: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(src.as_std_path()).expect("the shell has sources") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable source");
        if text.contains(THE_CLIENT) {
            users.push(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }
    users.sort();

    assert_eq!(
        users,
        vec!["download.rs"],
        "{THE_CLIENT} is meant to be named in download.rs and nowhere else — it \
         is the module that refuses any host but the font catalogue's, and a \
         request made around it is a request nobody checked"
    );
}

/// The updater plugin, the second way out (ADR-008).
const THE_UPDATER: &str = "tauri_plugin_updater";

/// **The update check is reached from one module, too.** `lib.rs` registers
/// the plugin and nothing else; `update.rs` is where the address it asks is
/// pinned and every download it names is checked. A call made around that
/// module is an update nobody checked the source of.
///
/// And no window is granted the plugin's own commands, so the webview cannot
/// reach it except through the two commands `update.rs` sits behind.
#[test]
fn the_updater_is_reached_from_one_module() {
    let src = repo_root().join("crates/biblecompose-tauri/src");
    for entry in std::fs::read_dir(src.as_std_path()).expect("the shell has sources") {
        let path = entry.expect("readable entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if !name.ends_with(".rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable source");
        let uses: Vec<&str> = text
            .lines()
            .filter(|l| l.contains(THE_UPDATER))
            .map(str::trim)
            .collect();
        match name.as_str() {
            "update.rs" => {}
            "lib.rs" => assert_eq!(
                uses,
                vec![".plugin(tauri_plugin_updater::Builder::new().build())"],
                "lib.rs registers the updater and does nothing else with it"
            ),
            _ => assert!(uses.is_empty(), "{name} reaches the updater: {uses:?}"),
        }
    }

    let dir = repo_root().join("crates/biblecompose-tauri/capabilities");
    for entry in std::fs::read_dir(dir.as_std_path()).expect("capabilities exist") {
        let path = entry.expect("readable entry").path();
        let text = std::fs::read_to_string(&path).expect("readable capability");
        assert!(
            !text.contains("updater:") && !text.contains("process:"),
            "{} grants the window the updater's own commands",
            path.display()
        );
    }
}

/// And the window itself is fenced, so a compromised frontend cannot fetch
/// either. BibleCompose renders content that arrived by email.
#[test]
fn the_window_may_not_load_anything_remote() {
    let config = std::fs::read_to_string(
        repo_root()
            .join("crates/biblecompose-tauri/tauri.conf.json")
            .as_std_path(),
    )
    .expect("the Tauri configuration");
    // Read as text rather than parsed. This asserts one string in a file that
    // is otherwise none of its business, and a JSON dependency in the test
    // crate to reach one key is a poor trade.
    let csp = config
        .lines()
        .find(|l| l.trim_start().starts_with("\"csp\""))
        .expect("a content-security policy is set");
    assert!(
        csp.contains("default-src 'self'"),
        "the window should load nothing it did not ship with: {csp}"
    );
    // **One kind of `http://` is not remote.** On Windows, Tauri serves a
    // custom protocol — `bcpdf:`, the pages of the built PDF (GUI-008) — at
    // `http://<scheme>.localhost`, and the policy has to name it there too.
    // `.localhost` never leaves the machine (RFC 6761), and the webview
    // answers it itself without asking the network. Anything else, or any
    // origin with a port or a path, is a server somewhere.
    let remote: Vec<&str> = csp
        .split(|c: char| c.is_whitespace() || c == ';' || c == '"')
        .filter(|token| token.starts_with("http://") || token.starts_with("https://"))
        .filter(|token| {
            let host = token.split_once("://").map_or("", |(_, rest)| rest);
            let local = host.strip_suffix(".localhost").is_some_and(|name| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric())
            });
            !local
        })
        .collect();
    assert!(
        remote.is_empty(),
        "the policy names a remote origin, {remote:?}: {csp}"
    );

    // And no capability grants HTTP. The list is meant to stay short.
    let dir = repo_root().join("crates/biblecompose-tauri/capabilities");
    for entry in std::fs::read_dir(dir.as_std_path()).expect("capabilities exist") {
        let path = entry.expect("readable entry").path();
        let text = std::fs::read_to_string(&path).expect("readable capability");
        assert!(
            !text.contains("http:"),
            "{} grants an HTTP permission",
            path.display()
        );
    }
}
