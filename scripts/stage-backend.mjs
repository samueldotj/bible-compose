/**
 * Turn an unpacked SILE runtime into a bundle stage (P5.7).
 *
 * The stage is what `BIBLECOMPOSE_SILE_BUNDLE` points at when the application
 * is built with `--features embedded-sile` — everything the typesetter needs
 * and nothing else, carried inside the executable ([ADR-006]).
 *
 *     node scripts/stage-backend.mjs <runtime> <stage>
 *     node scripts/stage-backend.mjs --verify <stage>
 *
 * # What comes out, and why it is less than what went in
 *
 * **The networking rocks go.** `luasec` and `luasocket` arrive with SILE's
 * standard rock set and nothing in this application's use of SILE opens a
 * connection; spike F-16 confirmed a build succeeds without them. NFR-004 says
 * a build needs no network, and the strongest form of that claim is a shipped
 * runtime with no socket code in it at all — which is a claim a person can
 * check with `ls` rather than one they have to take on trust.
 *
 * **The fontconfig cache goes.** It is 2.9 MB of one machine's font list,
 * written at first run and named after that machine's architecture. Shipping
 * it would send the build machine's fonts to every user, and fontconfig
 * rebuilds it on first run anyway.
 *
 * **And the application's own class goes in.** SILE resolves
 * `classes/biblecompose.lua` from its own tree, so a stage built from an
 * unpacked runtime carries whatever class was in that runtime — which is to
 * say, the one from whenever it was last unpacked. That is not a hypothetical:
 * a stale class in a local cache silently shadowed the repository's for six
 * weeks of this project's history, and the symptom was an error message about
 * an undeclared class option. The class is copied last, over the top, so the
 * stage always holds the class that shipped with the code that built it.
 *
 * # Two shapes go in, one comes out
 *
 * The Windows runtime is cross-built and lands **flat** — `sile.exe` with the
 * Lua tree and the DLLs beside it. Linux and macOS are built from source and
 * `make install`ed to a prefix, which lands as an ordinary Unix tree: the
 * executable in `bin/`, everything it reads in `share/sile/`.
 *
 * The bundle wants the flat shape, because that is what `bundle::ensure`
 * unpacks and runs. So a prefix tree is flattened here rather than taught to
 * the Rust: one shape crosses the seam, and the knowledge that `make install`
 * has opinions stays in the script that deals with `make install`.
 *
 * **And the default face goes in.** A runtime built from source carries no
 * fonts, so the shipped application could not set a page at all: the built-in
 * `typography.font_family` is DejaVu Serif, and on a machine without it the
 * pre-flight blocks the build with FONT-001 before anything else happens. That
 * is the pre-flight working and the bundle being incomplete. The faces are the
 * ones in `tests/fonts/`, which are already the application's default and are
 * already recorded there as redistributable.
 *
 * # Why a script rather than a build step
 *
 * Because the runtime is produced differently on each platform — cross-built
 * for Windows by `spike/s1-windows-cross.sh`, natively elsewhere — and the one
 * thing that must be identical is what gets *removed*. A rule enforced in one
 * place is a rule; three build recipes that each remember to delete `ssl.lua`
 * are three chances to forget.
 *
 * [ADR-006]: ../docs/adr/006-single-binary.md
 */

import { cpSync, existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

/**
 * Paths, relative to the runtime root, that must not reach a shipped bundle.
 *
 * Matched against the whole relative path with the platform's separators
 * normalised, and against a leading segment, so `lua_modules/share/lua/5.1/ssl`
 * takes its directory with it.
 */
const REMOVE = [
  // TLS and sockets (spike F-16, NFR-004).
  // The trailing `$` matters: without it the *files* under `socket/` go and
  // the directory stays, empty — which leaves a stage that passes a check for
  // networking code while still carrying something called `socket`.
  { pattern: /(^|[/])(socket|ssl|mime)([/]|\.lua$|$)/, why: "networking" },
  { pattern: /(^|[/])ltn12\.lua$/, why: "networking (luasocket's filter library)" },
  { pattern: /(^|[/])(socket|ssl|mime)\.(so|dll|dylib)$/, why: "networking" },
  // One machine's font list, rebuilt on first run wherever it lands.
  { pattern: /^fccache([/]|$)/, why: "a machine-specific fontconfig cache" },
];

/** Nothing matching these may exist in a finished stage. */
function offending(stage) {
  const found = [];
  for (const path of walk(stage)) {
    const rel = relative(stage, path).split(sep).join("/");
    for (const rule of REMOVE) {
      if (rule.pattern.test(rel)) found.push({ rel, why: rule.why });
    }
  }
  return found;
}

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) yield* walk(path);
    else yield path;
  }
}

function bytes(dir) {
  let total = 0;
  for (const path of walk(dir)) total += statSync(path).size;
  return total;
}

function mb(n) {
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/** Delete everything the rules name, directories included. */
function prune(stage) {
  const removed = [];
  // Directories first and by depth, so removing `ssl/` does not leave a walk
  // iterating paths that no longer exist.
  const candidates = [];
  const collect = (dir) => {
    for (const entry of readdirSync(dir)) {
      const path = join(dir, entry);
      const rel = relative(stage, path).split(sep).join("/");
      const rule = REMOVE.find((r) => r.pattern.test(rel));
      if (rule) {
        candidates.push({ path, rel, why: rule.why });
        continue;
      }
      if (statSync(path).isDirectory()) collect(path);
    }
  };
  collect(stage);

  for (const { path, rel, why } of candidates) {
    const size = statSync(path).isDirectory() ? bytes(path) : statSync(path).size;
    rmSync(path, { recursive: true, force: true });
    removed.push({ rel, why, size });
  }
  return removed;
}

/**
 * Copy a runtime into the stage, in the flat shape the bundle unpacks.
 *
 * A tree that already has the executable at its top is copied as it is. A
 * prefix install is rearranged: `bin/` and `share/sile/` are the two places
 * anything lives, and `lib/` carries the shared objects SILE built for itself
 * — `librusile`, `libtexpdf` — which have to travel with it.
 *
 * What does *not* travel is the system libraries it linked against. On Linux
 * that is HarfBuzz, fontconfig and ICU, and the smoke test cannot see the
 * difference because the machine that builds is the machine that runs. Said
 * out loud in `docs/RELEASING.md` rather than discovered by a publisher.
 */
function flatten(runtime, stage) {
  const flat = ["sile", "sile.exe"].some((n) => existsSync(join(runtime, n)));
  if (flat) {
    cpSync(runtime, stage, { recursive: true });
    return;
  }

  const exe = ["bin/sile", "bin/sile.exe"]
    .map((n) => join(runtime, ...n.split("/")))
    .find((p) => existsSync(p));
  const resources = join(runtime, "share", "sile");
  if (!exe || !existsSync(resources)) {
    console.error(
      `${runtime} is neither a flat runtime nor a prefix install — ` +
        `expected either sile at the top, or bin/sile and share/sile`,
    );
    process.exit(1);
  }

  cpSync(resources, stage, { recursive: true });
  cpSync(exe, join(stage, exe.endsWith(".exe") ? "sile.exe" : "sile"));
  // Every shared object under `lib`, wherever it sits in there, copied to the
  // top of the stage.
  //
  // **`rusile` is the reason this recurses.** `make install` puts it in
  // `lib/sile/`, one level down, because that is where Lua's `cpath` looks in
  // a prefix install — and a flat stage has no `lib` at all, so SILE looks for
  // it beside the executable. That is where the Windows cross-build has always
  // put it. Copying only the top of `lib` left it behind, and the symptom was
  // the backend refusing to report a version at all.
  //
  // The static archives and pkg-config files stay behind: nothing will read
  // them, and they are a third of the directory.
  const libs = join(runtime, "lib");
  if (existsSync(libs)) {
    for (const path of walk(libs)) {
      if (/\.(so|dylib)(\.\d+)*$/.test(path)) {
        cpSync(path, join(stage, path.split(sep).pop()));
      }
    }
  }
  console.log(`  flattened a prefix install: ${exe} and share/sile`);
}

/** This repository, whichever directory the script was run from. */
function root() {
  return new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
}

function verify(stage) {
  const problems = offending(stage);
  if (problems.length > 0) {
    console.error(`${problems.length} file(s) that must not ship:`);
    for (const p of problems) console.error(`  ${p.rel} — ${p.why}`);
    process.exit(1);
  }
  // The two things whose absence would be silent: a stage with no typesetter,
  // and one with no class for it to typeset with.
  const exe = ["sile", "sile.exe"].find((n) => existsSync(join(stage, n)));
  if (!exe) {
    console.error(`no sile executable in ${stage} — this is not a runtime`);
    process.exit(1);
  }
  if (!existsSync(join(stage, "classes", "biblecompose.lua"))) {
    console.error(`no classes/biblecompose.lua in ${stage} — this would build nothing`);
    process.exit(1);
  }
  // A bundle with no font blocks every build on a machine that has not got
  // one installed, which is the machine this exists for.
  const fonts = join(stage, "fonts");
  const faces = existsSync(fonts) ? readdirSync(fonts).filter((f) => /\.(ttf|otf)$/i.test(f)) : [];
  if (faces.length === 0) {
    console.error(`no fonts in ${stage} — every build would be blocked by FONT-001`);
    process.exit(1);
  }
  // **What Lua modules the stage actually has**, printed rather than assumed.
  //
  // A bundle missing a rock fails at run time with `module 'x' not found` and
  // a list of the places it looked — which is a long way from the machine that
  // built it, and reads as a path problem when it is a packaging one. Listing
  // them here costs a directory read and turns the next such failure into a
  // diff against this output.
  const rocks = join(stage, "lua_modules", "share", "lua", "5.1");
  const modules = existsSync(rocks)
    ? readdirSync(rocks)
        .filter((e) => e !== "sile")
        .sort()
    : [];
  console.log(`  lua modules (${modules.length}): ${modules.join(" ") || "none"}`);
  const top = readdirSync(stage)
    .filter((e) => /\.(so|dylib|dll)(\.\d+)*$/.test(e))
    .sort();
  console.log(`  shared objects (${top.length}): ${top.join(" ") || "none"}`);
  // `vstruct` is the one SILE reaches for first, when it opens a font — so a
  // stage without it cannot set a single page, and says so here rather than
  // on a publisher's machine.
  if (!modules.some((m) => m === "vstruct" || m === "vstruct.lua")) {
    console.error(`no vstruct in ${rocks} — SILE cannot read a font without it`);
    process.exit(1);
  }

  console.log(`stage-backend: ${stage} is clean, ${mb(bytes(stage))}, ${exe}`);
}

const args = process.argv.slice(2);
if (args[0] === "--verify") {
  const stage = args[1];
  if (!stage) {
    console.error("usage: stage-backend.mjs --verify <stage>");
    process.exit(2);
  }
  verify(stage);
} else {
  const [runtime, stage] = args;
  if (!runtime || !stage) {
    console.error("usage: stage-backend.mjs <runtime> <stage>");
    process.exit(2);
  }
  if (!existsSync(runtime)) {
    console.error(`${runtime} does not exist`);
    process.exit(1);
  }
  rmSync(stage, { recursive: true, force: true });
  mkdirSync(stage, { recursive: true });
  const before = bytes(runtime);
  flatten(runtime, stage);

  // The repository's own class and packages, over the top of whatever the
  // runtime brought. Last, so it wins.
  const ours = join(root(), "sile");
  if (existsSync(ours)) {
    cpSync(ours, stage, { recursive: true });
    console.log(`  copied ${relative(process.cwd(), ours)} over the runtime's own`);
  } else {
    console.error(`${ours} does not exist — the stage would carry no class`);
    process.exit(1);
  }

  // The default face, so a fresh machine can set a page without installing
  // anything. A runtime built from source has no `fonts/` of its own; one
  // unpacked from a SILE distribution does, and this adds to it rather than
  // replacing it.
  const fonts = join(root(), "tests", "fonts");
  if (!existsSync(fonts)) {
    console.error(`${fonts} does not exist — the bundle would carry no font`);
    process.exit(1);
  }
  mkdirSync(join(stage, "fonts"), { recursive: true });
  let faces = 0;
  for (const entry of readdirSync(fonts)) {
    if (/\.(ttf|otf)$/i.test(entry)) {
      cpSync(join(fonts, entry), join(stage, "fonts", entry));
      faces += 1;
    }
  }
  console.log(`  added ${faces} default face(s)`);

  const removed = prune(stage);
  for (const r of removed) {
    console.log(`  removed ${r.rel} (${mb(r.size)}) — ${r.why}`);
  }
  console.log(`stage-backend: ${mb(before)} in, ${mb(bytes(stage))} out`);
  verify(stage);
}
