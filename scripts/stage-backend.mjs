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
  { pattern: /(^|[/])(socket|ssl|mime)([/]|\.lua$)/, why: "networking" },
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
  cpSync(runtime, stage, { recursive: true });

  // The repository's own class and packages, over the top of whatever the
  // runtime brought. Last, so it wins.
  const ours = join(new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"), "sile");
  if (existsSync(ours)) {
    cpSync(ours, stage, { recursive: true });
    console.log(`  copied ${relative(process.cwd(), ours)} over the runtime's own`);
  } else {
    console.error(`${ours} does not exist — the stage would carry no class`);
    process.exit(1);
  }

  const removed = prune(stage);
  for (const r of removed) {
    console.log(`  removed ${r.rel} (${mb(r.size)}) — ${r.why}`);
  }
  console.log(`stage-backend: ${mb(before)} in, ${mb(bytes(stage))} out`);
  verify(stage);
}
