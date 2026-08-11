/**
 * `tauri dev` with a typesetting backend attached.
 *
 * ADR-006 carries SILE inside the executable, but the `embedded-sile` feature
 * is off by default — a plain `cargo build` should not hand 78 MB to
 * rust-embed — and `tauri dev` builds with `--no-default-features` besides. So
 * a development window has no backend, `discover()` falls through to whatever
 * `sile` is on PATH, and the window reports that it could not run the
 * typesetting backend. That has now cost this project two debugging sessions,
 * which is one more than a launcher script costs.
 *
 * What it does: find a runtime a packaged build already unpacked into the
 * cache, and point the dev window at it. Nothing is downloaded and nothing is
 * unpacked here — if no packaged build has ever run on this machine there is
 * nothing to find, and the script says so and starts anyway, because a window
 * with no backend is still worth having for everything that is not a build.
 */

import { spawn } from "node:child_process";
import { existsSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { homedir, platform } from "node:os";
import { join, dirname } from "node:path";

const repo = dirname(dirname(fileURLToPath(import.meta.url)));

/** Where `bundle::ensure` unpacks, per platform. Mirrors `src/bundle.rs`. */
function cacheRoot() {
  const home = homedir();
  switch (platform()) {
    case "win32":
      return join(process.env.LOCALAPPDATA ?? join(home, "AppData", "Local"), "biblecompose", "sile");
    case "darwin":
      return join(home, "Library", "Caches", "biblecompose", "sile");
    default:
      return join(process.env.XDG_CACHE_HOME ?? join(home, ".cache"), "biblecompose", "sile");
  }
}

/**
 * The most recently unpacked runtime.
 *
 * Entries are content-addressed by bundle hash, so several can coexist and any
 * of them is a working SILE. The newest is the one the most recent packaged
 * build produced, which is the one a developer means.
 */
function findRuntime() {
  const root = cacheRoot();
  if (!existsSync(root)) return null;

  const exe = platform() === "win32" ? "sile.exe" : "sile";
  const found = readdirSync(root)
    .map((name) => join(root, name))
    .filter((dir) => existsSync(join(dir, exe)))
    .map((dir) => ({ dir, at: statSync(dir).mtimeMs }))
    .sort((a, b) => b.at - a.at);

  return found[0] ? join(found[0].dir, exe) : null;
}

const env = { ...process.env };
if (env.BIBLECOMPOSE_SILE) {
  console.log(`dev: BIBLECOMPOSE_SILE is already set to ${env.BIBLECOMPOSE_SILE}`);
} else {
  const exe = findRuntime();
  if (exe) {
    env.BIBLECOMPOSE_SILE = exe;
    console.log(`dev: using the unpacked backend at ${exe}`);
  } else {
    console.warn(
      `dev: no unpacked backend under ${cacheRoot()} — builds will fail until one exists.\n` +
        `dev: run a packaged build once, or set BIBLECOMPOSE_SILE yourself.`,
    );
  }
}

// The repository's own class and packages, ahead of the unpacked runtime's
// copy. SILE_PATH is last-wins, and this is the whole point of a dev window:
// editing sile/classes/biblecompose.lua must take effect without repacking.
env.BIBLECOMPOSE_SILE_PATH = join(repo, "sile");

const npm = platform() === "win32" ? "npm.cmd" : "npm";
const child = spawn(npm, ["run", "tauri", "dev", ...process.argv.slice(2)], {
  cwd: repo,
  env,
  stdio: "inherit",
  shell: platform() === "win32",
});
child.on("exit", (code) => process.exit(code ?? 0));
