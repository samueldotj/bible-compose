# ADR-007 — The editor is a window in this process, not a program it starts

**Status:** **Accepted**
**Supersedes:** the bundled `easy-usfm-tauri` executable (`embedded-editor`, `BIBLECOMPOSE_EDITOR`, `BIBLECOMPOSE_EDITOR_BUNDLE`) that 0.3.0 to 0.4.3 shipped
**Relates to:** [ADR-001](001-usfm-core.md), [ADR-003](003-gui.md) (its "door to embedding easy-usfm's editor"), [ADR-006](006-single-binary.md), ARCHITECTURE §7.6

## Context

Double-clicking a book opens its USFM in easy-usfm. Until now that meant a second program: a 9 MB `easy-usfm-tauri` built by `tauri build`, embedded in this executable with rust-embed, unpacked into the cache folder on first use, started with `Command::spawn`, and asked to close at exit with `taskkill` or `SIGTERM`.

Each of those pieces has failed or come close to it:

- **0.3.0 to 0.4.1 shipped the placeholder.** The bundle was set on the wrong release step, so every installer said it carried no editor.
- **0.4.0's Linux job built the editor in place of the application.** The editor's checkout had a `tauri.conf.json` of its own.
- **A `cargo build` of the editor shows "can't reach this page".** It looks for a development server, and the stage script exists mostly to refuse one.
- **0.4.3 had to hide `taskkill`'s console window.** Closing editors meant starting a process for each one.

It also cost 9 MB per installer for a second copy of Tauri, WebView bindings and the engine, and the editor ran on its own release schedule (`EASY_USFM_REF`) apart from the `usfm-core` revision pinned in `Cargo.toml`.

## Decision

**easy-usfm's desktop shell becomes a Tauri plugin crate, `tauri-plugin-easy-usfm`, and this application registers it.** Opening a book opens an `editor-…` window in this process, served from `editor/index.html` in this application's own assets.

- **Native half.** A git dependency at **the same revision as `usfm-core`**, so the engine and the editor's shell can't drift apart. easy-usfm's own window is now a thin host over the same plugin, so a book edited from either application is saved by the same save ladder.
- **Interface.** easy-usfm's static frontend build is copied into `public/editor/` by `scripts/stage-editor.mjs`. The script refuses a checkout whose HEAD isn't the revision `Cargo.lock` resolved the plugin to. Vite serves the folder in development and copies it into `dist/` for a release.
- **Per window.** The plugin keeps everything a window holds by window label: the file it was opened on, its file watch, its recent list, its menu. Events go only to the window they concern. Opening a book that already has a window brings that window forward, so two editors are never open on one file.

### As configured here

- **Book only.** A window offers no New, Open or Open Recent, and the plugin refuses to open any path other than the book the window was opened on, whatever the page asks. File ends in Close, not Exit, because Exit would quit BibleCompose.
- **Its own recovery.** Snapshots and locks live in `editor-recovery/` in this application's data folder, apart from easy-usfm's.
- **No update check of its own.** The plugin's `updater` feature is left off. The editor is part of this application, so it is updated with it, by the one click-to-check [ADR-008](008-click-to-update.md) adds.
- **Its own permissions.** Editor windows get `capabilities/editor.json`: the plugin's commands, the dialogs, setting their title, closing themselves, and handing a link to the operating system. This application's own commands are declared in `build.rs` and granted to `main` alone, so an editor window, which renders a book's text, can't start a build or fetch a font.
- **Menus.** Windows and Linux give each editor window its own menu bar. macOS has one menu bar per application: the editor's goes up while an editor is in front, and the default goes back when `main` is.
- **Closing.** Closing the main window asks each editor to close. Each editor's unsaved-changes guard runs, and an editor kept open by its answer keeps the application running.

## Consequences

- **One binary, and smaller.** The editor's interface is about 1.1 MB of static files, replacing a 9 MB executable. Nothing is unpacked, started or killed.
- **The CSP widens for the whole application.** Tauri sets one policy per application, so `main` now allows `'wasm-unsafe-eval'` and `blob:` workers too, because the editor runs its engine as WebAssembly in a worker. The same change tightens `object-src`, `frame-src`, `base-uri` and `form-action`, which nothing in `main` used.
- **Changing the editor means bumping one revision.** That revision moves the engine, the shell and the interface together. Working on both repositories at once uses a local `[patch]` pointing at `../easy-usfm`.
- **easy-usfm's release no longer matters to this one.** BibleCompose carries whatever its pinned revision says, not easy-usfm's installers.
