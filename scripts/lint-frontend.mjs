/**
 * Bans the build enforces, because a convention is not a control.
 *
 * # No component talks to Tauri
 *
 * ADR-003: "No Svelte component imports Tauri APIs; every privileged call goes
 * through a typed service interface." The reason is stated there and is worth
 * repeating where the rule lives: without it the frontend gradually becomes
 * untestable — a component that calls `invoke` cannot be rendered in a test
 * without a Tauri host — and un-portable, which matters because ADR-003 is now
 * decided on a narrower argument than it was and may be revisited at P6.2.
 *
 * The discipline is only cheap if it starts at the first component, which is
 * why this exists before there are any.
 *
 * # No raw markup
 *
 * Scripture and project files arrive by email and USB from third parties. The
 * control is that *no path exists* from file content to raw markup, not that
 * escaping is applied carefully.
 *
 * # No user-facing literal in a component
 *
 * NFR-012: the release ships in English and the architecture supports another
 * locale. That only holds if adding one is a matter of writing a catalogue, so
 * a sentence typed into a component is the thing that breaks it — and it
 * breaks it silently, because English keeps working. This catches the two
 * shapes that account for nearly all of them: text between tags, and the
 * attributes a person reads.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const SRC = join(ROOT, "src");

/** Where privileged calls are allowed to live. */
const SERVICE_DIR = join("src", "lib", "services");

const RULES = [
  {
    name: "tauri-import",
    // `@tauri-apps/...`, in an import or a dynamic import.
    pattern: /(?:from\s*["']|import\s*\(\s*["'])@tauri-apps\//,
    message:
      "imports a Tauri API directly. Route it through a typed interface in " +
      `${SERVICE_DIR} (ADR-003).`,
    exempt: (rel) => rel.startsWith(SERVICE_DIR),
  },
  {
    name: "hard-coded-string",
    // Text between tags that starts with a capital and runs on, or a
    // `title=`, `placeholder=` or `aria-label=` holding the same. Anything
    // interpolated is `{...}` and does not match, which is the whole point.
    pattern:
      /(?:>\s*[A-Z][A-Za-z][A-Za-z ,'&?!.’…-]{3,}<)|(?:(?:title|placeholder|aria-label)="[A-Z][^"]{3,}")/,
    message:
      "has a user-facing string written into it. Put it in the catalogue in " +
      "src/lib/i18n.ts and read it with t() (NFR-012).",
    // Only components. The catalogue is *made* of strings, and a service or a
    // model has none a person reads.
    exempt: (rel) => !rel.endsWith(".svelte"),
  },
  {
    name: "raw-markup",
    pattern: /\{@html\b|\.innerHTML\s*=/,
    message: "uses raw markup. Project files come from third parties.",
    exempt: () => false,
  },
];

function* walk(dir) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return;
  }
  for (const entry of entries) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      if (entry === "node_modules" || entry === "generated") continue;
      yield* walk(path);
    } else if (/\.(svelte|ts|js|mts|mjs)$/.test(entry)) {
      yield path;
    }
  }
}

const problems = [];
let scanned = 0;

for (const path of walk(SRC)) {
  scanned += 1;
  const rel = relative(ROOT, path).split(sep).join(sep);
  const text = readFileSync(path, "utf8");

  for (const rule of RULES) {
    if (rule.exempt(rel)) continue;
    text.split("\n").forEach((line, i) => {
      if (rule.pattern.test(line)) {
        problems.push(`${rel}:${i + 1}: ${rule.message}`);
      }
    });
  }
}

if (problems.length > 0) {
  console.error(`${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}

console.log(`lint-frontend: ${scanned} file(s), no problems`);
