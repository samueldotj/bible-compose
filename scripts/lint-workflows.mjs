/**
 * Every workflow file parses, and carries nothing a YAML reader will refuse.
 *
 * **A workflow that only runs on a tag is only broken on a tag.** `release.yml`
 * fires on `v*` and nothing else, so a file that GitHub cannot read would be
 * discovered at the exact moment it is least wanted — after the version is
 * bumped, the changelog written and the tag pushed.
 *
 * That is not hypothetical. A script writing this repository's release workflow
 * put a literal NUL byte into a `find -printf` argument; the YAML was invalid
 * from that moment, everything else went on working, and nothing would have
 * said so until a release.
 *
 * No dependency: a full YAML parser is not needed to catch the things that
 * actually go wrong in a generated file. What is checked is what a reader
 * refuses outright — control characters YAML forbids, and tabs used for
 * indentation — plus the two structural keys whose absence means the file is
 * not a workflow at all.
 */

import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const DIR = join(ROOT, ".github", "workflows");

const problems = [];
let scanned = 0;

for (const entry of readdirSync(DIR)) {
  if (!/\.ya?ml$/.test(entry)) continue;
  const path = join(DIR, entry);
  const rel = relative(ROOT, path);
  const text = readFileSync(path, "utf8");
  scanned += 1;

  text.split("\n").forEach((line, i) => {
    const where = `${rel}:${i + 1}`;
    // YAML permits tab, line feed and carriage return, and forbids the rest of
    // C0 outright — the same rule the emitter applies to XML, and for the same
    // reason: there is no escape that makes them legal.
    const control = [...line].find((c) => c !== "\t" && c.charCodeAt(0) < 0x20);
    if (control) {
      problems.push(
        `${where}: contains U+${control.charCodeAt(0).toString(16).padStart(4, "0")}, ` +
          `which YAML cannot carry in any form`,
      );
    }
    if (/^\t| \t/.test(line)) {
      problems.push(`${where}: indented with a tab, which YAML forbids`);
    }
  });

  // Not a parse, and not pretending to be. These two are what separates a
  // workflow from a file that happens to be in the folder.
  for (const key of ["jobs:", "on:"]) {
    if (!text.split("\n").some((l) => l.startsWith(key))) {
      problems.push(`${rel}: has no top-level \`${key}\``);
    }
  }
}

if (problems.length > 0) {
  console.error(`${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}

console.log(`lint-workflows: ${scanned} file(s), no problems`);
