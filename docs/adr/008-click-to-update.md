# ADR-008 — A copy updates itself only when somebody asks it to

**Status:** **Accepted**
**Amends:** ARCHITECTURE §14's "exactly one request", and [ADR-007](007-editor-in-process.md)'s "no update check"
**Relates to:** FONT-006, NFR-004, ARCHITECTURE §7.6, [RELEASING.md](../RELEASING.md)

## Context

A new version reaches a publisher only if they come back to the releases page, see it, pick the right file and install it over the old one. Most never do. That leaves them on whatever they first installed, bugs included.

Tauri's updater plugin does this job well. It reads a manifest, downloads the installer for this platform, checks it against a key compiled into the copy, and installs it. But it is a second way for the application to reach the network. Until now the application promised exactly one, the font download. That promise is not just politeness here: some publishers work on machines that must never phone home, or in places where a program that does would be noticed.

## Decision

**Add "Check for updates", and make it the same kind of request as the font download.**

- **Only when clicked.** The button is on the start screen. Nothing checks at start-up, on a timer or after a build. `session.update` starts `idle`, and only the button's handler leaves that state.
- **One address, saying nothing.** The check asks `releases/latest/download/latest.json` on this project's GitHub. The address has no query string and no version or platform in the path. It is a constant in `biblecompose-tauri::update` rather than in the config, so the test that pins it reads the same value the code uses.
- **Only this project's downloads.** The manifest names a download, and `update.rs` refuses any URL outside `…/bible-compose/releases/download/` before fetching it. The updater then checks the bytes against the copy's public key. An installer whose signature doesn't match is never run.
- **No restart over somebody's work.** Installing is refused while a build is running or an editor window is open, with a sentence saying which. It is not forced.
- **Rust only.** The plugin is registered for its Rust API. No capability grants the window `updater:` permissions, so the webview, which renders text that arrived by email, can reach updates only through the two commands this application declares.
- **Drafts are never offered.** `releases/latest` skips drafts, and the release workflow only makes drafts. A person publishing the draft is what ships an update.
- **No key, no updates.** The public key is written into the config at release time, from the `TAURI_UPDATER_PUBKEY` variable, and only when the `TAURI_SIGNING_PRIVATE_KEY` secret is there too. A development build or an unsigned release has an empty key. It answers "this copy cannot update itself" without asking the network anything.

## Consequences

- **Two requests, both clicked for.** ARCHITECTURE §14 now says so. `biblecompose-testkit/tests/offline.rs` still holds the shell to one HTTP client of its own (`ureq`, for fonts). The updater uses the framework's `reqwest`, which the shell already reached. A test pins the updater to `update.rs`.
- **The pipeline is unchanged.** Nothing under the shell links the updater, so NFR-004's dependency-graph test passes as before, and the CLI has no update check.
- **A signing key to keep.** If the private key is lost, every installed copy will refuse every future update. They are then stranded until someone reinstalls by hand. RELEASING.md says where the key goes and what to back up.
- **Per platform.** Windows runs the NSIS or MSI installer in passive mode. The NSIS installer is per-machine, so expect a UAC prompt. macOS swaps the `.app` in place and Linux swaps the AppImage. A `.deb` install updates only if the bundler signed the `.deb`. If not, the manifest leaves it out, and it updates through the package manager as before.
- **easy-usfm's own updater stays off.** The editor is part of this application (ADR-007), so updating BibleCompose updates it too.
