/**
 * Build easy-usfm and turn it into a bundle stage.
 *
 * The stage is what `BIBLECOMPOSE_EDITOR_BUNDLE` points at when the
 * application is built with `--features embedded-editor` — the editor and
 * nothing else, carried inside the executable so that opening a book's USFM
 * needs nothing installed.
 *
 *     node scripts/stage-editor.mjs <easy-usfm checkout> <stage>
 *     node scripts/stage-editor.mjs --verify <stage>
 *
 * The sibling of `stage-backend.mjs`, and it does the same job for the other
 * thing this application carries.
 *
 * # `tauri build`, and never `cargo build`
 *
 * **This is the whole reason the script exists rather than a `cp`.** A Tauri
 * application decides where its interface comes from at build time, and only
 * the Tauri CLI switches it to the bundled assets: `cargo build --release`
 * leaves it pointing at `devUrl`, which on easy-usfm is
 * `http://localhost:1420`. The executable runs, opens a window, takes the file
 * it was given — and shows *"Hmmm… can't reach this page"*, because nothing is
 * serving that port on anybody's machine but a developer's.
 *
 * That is not a guess. A `cargo build --release` of the editor was staged by
 * hand while this feature was being written, and it did exactly that; the same
 * mistake made three separate attempts to verify the feature report failure
 * when the feature was fine. Staged into a release it would have shipped a
 * 9 MB editor that cannot draw itself, on every machine.
 *
 * `--no-bundle`, because the installers are not wanted: this stage is embedded
 * in another application's executable, and a `.msi` of the editor inside a
 * `.msi` of BibleCompose is a thing nobody asked for.
 *
 * # Three shapes go in
 *
 * Windows lands a single `easy-usfm-tauri.exe`. macOS lands an
 * `easy-usfm.app` directory, which is the program on that platform and has to
 * travel whole. Linux lands a bare executable. Whatever it is, it is copied
 * into the stage with the layout `editor::locate` expects: the executable at
 * the top, named `easy-usfm-tauri` (`.exe` on Windows), and anything it needs
 * beside it.
 */

import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { platform } from "node:os";

const WINDOWS = platform() === "win32";

/** What `editor::locate` looks for inside the stage. */
const EXE_NAME = WINDOWS ? "easy-usfm-tauri.exe" : "easy-usfm-tauri";

/** Where the Tauri CLI leaves a `--no-bundle` release build. */
const BUILT = join("target", "release", EXE_NAME);

/** The macOS application directory, when there is one. */
const APP = join("target", "release", "bundle", "macos", "easy-usfm.app");

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

function bytes(dir) {
  let total = 0;
  for (const file of walk(dir)) total += statSync(file).size;
  return total;
}

function mb(n) {
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/**
 * Build the editor, with the CLI and not with cargo.
 *
 * `npm ci` is not run here: a checkout that cannot build is the caller's
 * problem to notice, and a script that installs dependencies into somebody
 * else's repository on their behalf is a script that surprises them.
 */
function build(checkout) {
  if (!existsSync(join(checkout, "package.json"))) {
    die(`${checkout} does not look like an easy-usfm checkout: no package.json`);
  }
  console.log(`stage-editor: building the editor in ${checkout}`);
  try {
    // Through npm so the repository's own pinned CLI is the one that runs,
    // rather than whatever `tauri` happens to be on the PATH.
    execFileSync("npm", ["run", "tauri", "--", "build", "--no-bundle"], {
      cwd: checkout,
      stdio: "inherit",
      shell: WINDOWS,
    });
  } catch {
    die("stage-editor: the editor did not build");
  }
}

function stage(checkout, into) {
  build(checkout);

  rmSync(into, { recursive: true, force: true });
  mkdirSync(into, { recursive: true });

  const app = join(checkout, APP);
  if (existsSync(app)) {
    // macOS: the `.app` is the program. Copied whole, and the executable
    // inside it is what `editor::locate` will find once the stage is walked.
    cpSync(app, join(into, "easy-usfm.app"), { recursive: true });
    console.log(`stage-editor: staged easy-usfm.app (${mb(bytes(into))})`);
    return;
  }

  const exe = join(checkout, BUILT);
  if (!existsSync(exe)) {
    die(
      `stage-editor: ${exe} is not there after a build.\n` +
        "  The Tauri CLI leaves a --no-bundle build at target/release/ — if this\n" +
        "  checkout names its binary something else, this script needs to know.",
    );
  }
  cpSync(exe, join(into, EXE_NAME));
  console.log(`stage-editor: staged ${EXE_NAME} (${mb(bytes(into))})`);
}

/**
 * Check a stage before it is embedded.
 *
 * Cheap, and the alternative is finding out from a publisher. The two things
 * worth asserting are that the executable is there under the name the Rust
 * looks for, and that this is not a `cargo build` wearing a release's clothes.
 */
function verify(into) {
  if (!existsSync(into)) die(`stage-editor: ${into} does not exist`);

  const found = [...walk(into)];
  if (found.length === 0) die(`stage-editor: ${into} is empty`);

  const exe = found.find((p) => p.endsWith(EXE_NAME));
  if (!exe) {
    die(
      `stage-editor: no ${EXE_NAME} anywhere in ${into}.\n` +
        `  editor::locate looks for exactly that name.`,
    );
  }

  // **The interface has to be inside the executable**, and this is how to tell.
  //
  // A production build compiles `dist` in, so the hashed name Vite gave the
  // entry chunk — `assets/index-<hash>.js`, out of `dist/index.html` — is a
  // string in the binary. A `cargo build` embeds no assets and the name is
  // nowhere in it. Measured on both: two hits against none.
  //
  // **Looking for `devUrl` instead does not work, which is worth writing down
  // because it is the obvious check.** `generate_context!` compiles the whole
  // of `tauri.conf.json` into every build, `devUrl` with it, so
  // `localhost:1420` is in a correct production binary as well. That check was
  // written first and rejected a perfectly good stage.
  const blob = readFileSync(exe);
  if (!blob.includes(Buffer.from("assets/index-"))) {
    die(
      `stage-editor: ${exe} does not carry an interface.\n` +
        "  It was built with `cargo build` rather than `tauri build`, so its\n" +
        "  assets are not in it and it will try to reach a development server.\n" +
        "  Embedded, it shows \"Hmmm… can't reach this page\" on every machine\n" +
        "  that is not a developer's.",
    );
  }

  console.log(`stage-editor: ${into} looks like an editor (${mb(bytes(into))})`);
}

const args = process.argv.slice(2);
if (args[0] === "--verify") {
  if (args.length !== 2) {
    console.error("usage: node scripts/stage-editor.mjs --verify <stage>");
    process.exit(2);
  }
  verify(resolve(args[1]));
} else {
  if (args.length !== 2) {
    console.error("usage: node scripts/stage-editor.mjs <easy-usfm checkout> <stage>");
    process.exit(2);
  }
  const into = resolve(args[1]);
  stage(resolve(args[0]), into);
  verify(into);
}
