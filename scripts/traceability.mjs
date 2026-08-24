/**
 * Every MUST in the SRS, and where it is answered (P6.6).
 *
 *     node scripts/traceability.mjs           # rewrite docs/TRACEABILITY.md
 *     node scripts/traceability.mjs --check   # fail if it is out of date
 *
 * # What it can and cannot tell you
 *
 * It finds each requirement's id in the test suite and reports the `#[test]`
 * that follows it. That is evidence somebody *meant* to check the requirement
 * — it is not proof the check is any good, and no generated table could be.
 *
 * The value is in the blank rows. A requirement that no test mentions and that
 * nobody has written a sentence about is one nobody has thought about, and
 * this refuses to generate a table containing one. Everything else here exists
 * to make that refusal meaningful: the hand-resolved rows had their tests read
 * rather than guessed at, and the exceptions carry their reasoning so that
 * "there is no test" is a decision on the record instead of an absence.
 *
 * It earned its keep on the first run by finding BLD-003 — the PDF's name was
 * supposed to come from the publication's and was always `bible.pdf`.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative, sep } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const OUT = join(ROOT, "docs", "TRACEABILITY.md");

/**
 * Where a requirement is answered when no test names it.
 *
 * Every one of these was resolved by reading the test. They are here rather
 * than as a comment in the test because the requirement is the thing being
 * traced, and a reader of this table should not have to go looking.
 */
const BY_HAND = {
  "PRJ-007": ["crates/biblecompose-app/tests/figures.rs", "climbing_out_of_the_project_is_refused"],
  "FUN-004": ["crates/biblecompose-app/tests/acceptance.rs", "h_invalid_config"],
  "FUN-005": [
    "crates/biblecompose-tauri/tests/commands.rs",
    "a_folder_of_usfm_opens_with_its_books_and_its_defaults",
  ],
  "BLD-001": [
    "crates/biblecompose-tauri/tests/commands.rs",
    "a_folder_of_usfm_opens_with_its_books_and_its_defaults",
  ],
  "BLD-002": ["crates/biblecompose-app/tests/acceptance.rs", "a_defaults_only"],
  "BLD-006": ["crates/biblecompose-app/tests/acceptance.rs", "j_cancel"],
  "FUN-001": ["crates/biblecompose-testkit/tests/normalize_corpus.rs", "normalizing_the_corpus_loses_nothing"],
  "SCR-002": ["crates/biblecompose-app/tests/style_matrix.rs", "the_matrix_covers_every_selector_class"],
  "SCR-006": ["crates/biblecompose-app/tests/acceptance.rs", "f_figure"],
  "USFM-001": ["crates/biblecompose-config/tests/document.rs", "a_syntax_error_has_a_position"],
  "USFM-002": ["crates/biblecompose-scripture/tests/normalize.rs", "a_cross_reference_is_not_a_note"],
  "USFM-005": [
    "crates/biblecompose-testkit/tests/normalize_corpus.rs",
    "normalizing_the_corpus_loses_nothing",
  ],
  "USFM-006": ["crates/biblecompose-app/tests/acceptance.rs", "g_invalid_usfm"],
  "SILE-001": ["crates/biblecompose-testkit/tests/architecture.rs", "no_crate_depends_on_the_app_except_the_cli"],
  "SILE-007": ["crates/biblecompose-app/tests/backend_failure.rs", "a_known_failure_is_named"],
  "SILE-008": ["crates/biblecompose-app/tests/hygiene.rs", "a_finished_build_leaves_no_scripture_on_disk"],
  "PDF-004": ["crates/biblecompose-app/tests/corpus_build.rs", "no_corpus_book_is_blocked_by_anything_but_a_font"],
  "GUI-001": [
    "crates/biblecompose-tauri/tests/commands.rs",
    "a_folder_of_usfm_opens_with_its_books_and_its_defaults",
  ],
  "GUI-005": ["crates/biblecompose-app/tests/acceptance.rs", "h_invalid_config"],
  "GUI-009": ["crates/biblecompose-app/tests/drafts.rs", "a_draft_is_written_beside_the_real_pdf"],
  "GUI-012": ["crates/biblecompose-app/tests/reuse.rs", "a_second_identical_build_does_not_run_the_backend"],
  "NFR-003": ["crates/biblecompose-app/tests/opening.rs", "a_whole_canon_opens_in_well_under_a_second"],
  "NFR-005": ["crates/biblecompose-app/tests/metadata.rs", "a_title_in_another_script_survives"],
  "NFR-006": ["crates/biblecompose-app/tests/style_golden.rs", "the_styles_block_is_byte_stable"],
  "NFR-007": ["crates/biblecompose-project/tests/discovery.rs", "generated_directories_never_become_inputs"],
  "NFR-008": [
    "crates/biblecompose-config/tests/settings.rs",
    "an_unknown_version_produces_exactly_one_diagnostic",
  ],
  "DIA-001": ["crates/biblecompose-app/tests/acceptance.rs", "h_invalid_config"],
  "DIA-003": ["crates/biblecompose-app/tests/figures.rs", "omit_warns_and_names_the_figure_to_withhold"],
};

/** Requirements answered by a decision rather than a test, with the reason. */
const EXCEPTIONS = {
  "GUI-007":
    "**No automated test, and the requirement is met by design.** Every build " +
    "writes the backend's whole output to a file and the window reports where; " +
    "the panel shows each message with its raw detail, which is selectable text " +
    "in a webview. What cannot be asserted is that a person can *copy* it — that " +
    "is the operating system's clipboard, and a test of it would be a test of the " +
    "webview.",
  "GUI-010":
    "**Satisfied vacuously, which is the stronger answer.** There are no unsaved " +
    "edits to indicate or protect: every settings and style change is written to " +
    "the file as it is made (CFG-005), so closing a project cannot discard " +
    "anything. The requirement anticipates a dialog with an OK button, and this " +
    "window does not have one.",
};

function musts() {
  const srs = readFileSync(join(ROOT, "docs", "SRS-v0.1.md"), "utf8");
  const out = [];
  for (const line of srs.split("\n")) {
    const m = line.match(/^\|\s*([A-Z]+-\d+)\s*\|\s*(.+?)\s*\|\s*MUST\s*\|/);
    if (m) out.push({ id: m[1], text: m[2] });
  }
  return out;
}

/** id → [[file, test]], for every id named above a `#[test]`. */
function evidence(ids) {
  const files = execFileSync("git", ["ls-files"], { cwd: ROOT, encoding: "utf8" })
    .split("\n")
    .filter((f) => f.endsWith(".rs") && f.includes("test"));

  const found = Object.fromEntries(ids.map((i) => [i, []]));
  for (const file of files) {
    const lines = readFileSync(join(ROOT, file.split("/").join(sep)), "utf8").split("\n");
    // Only functions marked `#[test]`. A helper named `check` sitting under a
    // requirement's id is not evidence that the requirement is checked.
    const tests = [];
    for (let n = 0; n < lines.length; n++) {
      if (!/^\s*#\[test\]/.test(lines[n])) continue;
      for (let m = n + 1; m < Math.min(n + 6, lines.length); m++) {
        const fn = lines[m].match(/^\s*fn (\w+)/);
        if (fn) {
          tests.push([m, fn[1]]);
          break;
        }
      }
    }
    for (let n = 0; n < lines.length; n++) {
      for (const id of ids) {
        if (!lines[n].includes(id)) continue;
        // A module doc comment names what the *file* is about, so the test
        // that happens to come first is not the answer — the file is. Naming
        // one arbitrary test would read as precision and be a coin toss.
        if (/^\s*\/\/!/.test(lines[n])) {
          found[id].push([file, null]);
          continue;
        }
        const next = tests.find(([at]) => at > n);
        if (next) found[id].push([file, next[1]]);
      }
    }
  }
  return found;
}

function render() {
  const all = musts();
  const found = evidence(all.map((m) => m.id));
  const rows = [];
  let exceptions = 0;
  let blank = 0;

  for (const { id, text } of all) {
    const short = text.length <= 90 ? text : `${text.slice(0, 87).trimEnd()}…`;
    if (EXCEPTIONS[id]) {
      rows.push(`| **${id}** | ${short} | *exception* — ${EXCEPTIONS[id]} |`);
      exceptions += 1;
      continue;
    }
    // A named test beats a whole file, because it is the more specific claim.
    const seen = [...new Map(found[id].map((h) => [h.join("::"), h])).values()];
    seen.sort((a, b) => (b[1] ? 1 : 0) - (a[1] ? 1 : 0));
    const hit = seen[0] ?? BY_HAND[id];
    if (!hit) {
      rows.push(`| **${id}** | ${short} | **nothing** |`);
      blank += 1;
      continue;
    }
    const [file, fn] = hit;
    const more = seen.length > 1 ? ` (+${seen.length - 1} more)` : "";
    const where = fn
      ? `[\`${fn}\`](../${file})`
      : `[${file.split("/").pop()}](../${file}) *(the whole suite)*`;
    rows.push(`| **${id}** | ${short} | ${where}${more} |`);
  }

  const verified = rows.length - exceptions - blank;
  const body = `# MUST traceability

Every MUST in [SRS v0.1](SRS-v0.1.md), and where it is answered.

**Generated, then read.** \`scripts/traceability.mjs\` finds each requirement's
id in the test suite and reports the \`#[test]\` that follows it. Rows it cannot
resolve that way were resolved by reading the test; the two it cannot resolve at
all are recorded below as exceptions, with their reasoning.

It is worth exactly what it is worth. A test that names a requirement is
evidence somebody meant to check it, not proof the check is good. **The value is
in the absence of blank rows** — and in having to write a sentence for anything
that would otherwise have one.

It earned its keep on the first run by finding a real gap: BLD-003 says the
PDF's name is derived from the publication's, and it was always \`bible.pdf\`.

| Requirement | | Verified by |
|---|---|---|
${rows.join("\n")}

---

**${rows.length} MUST requirements.** ${verified} are verified by a test;
${exceptions} are recorded exceptions; ${blank} are unanswered.

\`node scripts/traceability.mjs --check\` fails if this file is out of date, and
\`node scripts/traceability.mjs\` rewrites it. Neither will produce a table with
a blank row in it.
`;

  return { body, blank };
}

const { body, blank } = render();

if (blank > 0) {
  console.error(
    `${blank} MUST requirement(s) have nothing pointing at them. Add a test, ` +
      `name the requirement in one that exists, or record an exception in ` +
      `scripts/traceability.mjs with the reasoning.`,
  );
  process.exit(1);
}

if (process.argv.includes("--check")) {
  const current = (() => {
    try {
      return readFileSync(OUT, "utf8");
    } catch {
      return "";
    }
  })();
  if (current !== body) {
    console.error(
      `${relative(ROOT, OUT)} is out of date — run \`node scripts/traceability.mjs\``,
    );
    process.exit(1);
  }
  console.log("traceability: up to date");
} else {
  writeFileSync(OUT, body);
  console.log(`traceability: ${relative(ROOT, OUT)} written`);
}
