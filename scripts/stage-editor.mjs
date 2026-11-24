/**
 * Build easy-usfm's interface and stage it where this application serves it.
 *
 *     node scripts/stage-editor.mjs <easy-usfm checkout>
 *     node scripts/stage-editor.mjs --verify
 *
 * The editor is a window in this process (ADR-007): its native half is the
 * `tauri-plugin-easy-usfm` crate this workspace depends on, and its interface
 * is the static build of easy-usfm's frontend, copied into `public/editor/`.
 * Vite serves that folder as it is in development and copies it into `dist/`
 * for a release, where Tauri compiles it into the executable with everything
 * else. `editor::open` opens `editor/index.html`.
 *
 * # The two halves come from one revision
 *
 * **This is the check the script exists for.** The interface calls the
 * plugin's commands by name with arguments of a shape, and the two are only
 * known to agree when they come from the same commit. So the checkout's HEAD
 * is compared with the revision `Cargo.lock` resolved the plugin to, and a
 * stage from any other commit is refused. A workspace patched onto a local
 * checkout (a `[patch]` to a path) has no revision to compare and is told so.
 *
 * # `npm run build`, not `tauri build`
 *
 * Only the interface is wanted. The editor's own Tauri application — its
 * executable, its installers — is easy-usfm's and nothing here carries it.
 * `npm run build` compiles the engine to WebAssembly (`wasm-pack`, so the
 * `wasm32-unknown-unknown` target has to be installed) and builds the page.
 */

import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { platform } from "node:os";

const WINDOWS = platform() === "win32";
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** Where the interface is staged, and so where `editor::EDITOR_PAGE` points. */
const STAGE = join(ROOT, join("public", "editor"));

/** The crate whose revision the interface must match. */
const PLUGIN = "tauri-plugin-easy-usfm";

function die(message) {
  console.error(message);
  process.exit(1);
}

function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(path);
    else yield path;
  }
}

function mb(n) {
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/**
 * The revision `Cargo.lock` resolved the plugin to, or `null` when the
 * workspace builds it from a path.
 */
function lockedRevision() {
  const lock = readFileSync(join(ROOT, "Cargo.lock"), "utf8");
  const entry = lock
    .split("[[package]]")
    .find((block) => block.includes(`name = "${PLUGIN}"`));
  if (!entry) die(`stage-editor: ${PLUGIN} is not in Cargo.lock — build the workspace once first`);
  const source = /source = "git\+[^"#]*#([0-9a-f]{40})"/.exec(entry);
  return source ? source[1] : null;
}

function headOf(checkout) {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], { cwd: checkout, encoding: "utf8" }).trim();
  } catch {
    die(`stage-editor: ${checkout} is not a git checkout`);
  }
}

function build(checkout) {
  if (!existsSync(join(checkout, "package.json"))) {
    die(`${checkout} does not look like an easy-usfm checkout: no package.json`);
  }
  if (!existsSync(join(checkout, "crates", PLUGIN))) {
    die(
      `stage-editor: ${checkout} has no crates/${PLUGIN}.\n` +
        "  It predates the editor running inside BibleCompose; check out the\n" +
        "  revision Cargo.lock names instead.",
    );
  }

  const locked = lockedRevision();
  const head = headOf(checkout);
  if (locked === null) {
    console.warn(
      `stage-editor: ${PLUGIN} is built from a path, so there is no revision to check.\n` +
        `  Staging ${checkout} at ${head.slice(0, 12)} as it stands.`,
    );
  } else if (locked !== head) {
    die(
      `stage-editor: ${checkout} is at ${head.slice(0, 12)} but Cargo.lock builds ` +
        `${PLUGIN} from ${locked.slice(0, 12)}.\n` +
        "  The interface and the plugin must come from one revision — check that one out.",
    );
  }

  // `npm ci` is not run here: a checkout that cannot build is the caller's
  // problem to notice, and a script that installs dependencies into somebody
  // else's repository on their behalf is a script that surprises them.
  console.log(`stage-editor: building the editor's interface in ${checkout}`);
  try {
    execFileSync("npm", ["run", "build"], { cwd: checkout, stdio: "inherit", shell: WINDOWS });
  } catch {
    die("stage-editor: the editor's interface did not build");
  }
}

function stage(checkout) {
  build(checkout);
  const dist = join(checkout, "dist");
  if (!existsSync(join(dist, "index.html"))) {
    die(`stage-editor: ${dist} has no index.html after a build`);
  }
  rmSync(STAGE, { recursive: true, force: true });
  mkdirSync(STAGE, { recursive: true });
  cpSync(dist, STAGE, { recursive: true });
}

/**
 * Check a stage before it is built in.
 *
 * Cheap, and the alternative is finding out from a publisher. A page, the
 * script the page loads, and the engine the worker runs.
 */
function verify() {
  if (!existsSync(join(STAGE, "index.html"))) {
    die(`stage-editor: no editor staged at ${STAGE} — run with an easy-usfm checkout`);
  }
  const files = [...walk(STAGE)];
  const page = readFileSync(join(STAGE, "index.html"), "utf8");
  if (!/assets\/index-[^"']+\.js/.test(page)) {
    die(`stage-editor: ${STAGE}/index.html loads no built script — is this a production build?`);
  }
  if (!files.some((f) => f.endsWith(".wasm"))) {
    die(`stage-editor: ${STAGE} carries no WebAssembly — the editor would have no engine`);
  }
  const bytes = files.reduce((total, f) => total + statSync(f).size, 0);
  console.log(`stage-editor: ${STAGE} looks like an editor (${files.length} files, ${mb(bytes)})`);
}

const args = process.argv.slice(2);
if (args[0] === "--verify") {
  if (args.length !== 1) {
    console.error("usage: node scripts/stage-editor.mjs --verify");
    process.exit(2);
  }
  verify();
} else {
  if (args.length !== 1) {
    console.error("usage: node scripts/stage-editor.mjs <easy-usfm checkout>");
    process.exit(2);
  }
  stage(resolve(args[0]));
  verify();
}
