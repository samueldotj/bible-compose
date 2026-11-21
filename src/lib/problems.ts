/**
 * The problems list as text a person can paste into an email or an issue.
 *
 * A project with a systematic fault reports the same thing thousands of times
 * — one per paragraph, one per verse — and ten thousand lines is not
 * something anyone reads. So the report says each distinct problem once, how
 * many times it happened, and every place it happened, compactly, per file.
 *
 * "The same problem" is the same severity, code and help, with a message that
 * differs only in its numbers: `chapter 3 has no verse 7` and `chapter 4 has
 * no verse 1` are one problem in two places.
 */
import type { Diagnostic, Severity } from "./services/backend";

/** What the report needs to say in words; the caller supplies the locale. */
export interface ReportPhrases {
  readonly problems: (count: number) => string;
  readonly errors: (count: number) => string;
  readonly warnings: (count: number) => string;
  readonly notes: (count: number) => string;
  readonly occurrences: (count: number) => string;
}

export interface ProblemGroup {
  readonly severity: Severity;
  readonly code: string;
  /** The message itself when every occurrence says the same; otherwise with `#` for each number. */
  readonly message: string;
  readonly help?: string;
  readonly count: number;
  /** File label → `line:column` places in it, in the order they were reported. */
  readonly places: ReadonlyMap<string, readonly string[]>;
  /** Occurrences with no location at all. */
  readonly unplaced: number;
}

const RANK: Record<Severity, number> = { error: 0, warning: 1, info: 2 };

const template = (message: string) => message.replace(/\d+/g, "#");

/**
 * Group diagnostics into distinct problems, errors first, then the most
 * frequent first, so the top of the report is what to fix.
 *
 * `fileLabel` turns a path into what the report calls it — a book code where
 * the project knows one.
 */
export function groupProblems(
  diagnostics: readonly Diagnostic[],
  fileLabel: (path: string) => string,
): ProblemGroup[] {
  interface Building {
    severity: Severity;
    code: string;
    first: string;
    same: boolean;
    template: string;
    help?: string;
    count: number;
    places: Map<string, string[]>;
    unplaced: number;
    order: number;
  }
  const groups = new Map<string, Building>();
  for (const d of diagnostics) {
    const shape = template(d.message);
    const key = `${d.severity}\u0000${d.code}\u0000${shape}\u0000${d.help ?? ""}`;
    let g = groups.get(key);
    if (!g) {
      g = {
        severity: d.severity,
        code: d.code,
        first: d.message,
        same: true,
        template: shape,
        help: d.help,
        count: 0,
        places: new Map(),
        unplaced: 0,
        order: groups.size,
      };
      groups.set(key, g);
    }
    g.count += 1;
    if (d.message !== g.first) g.same = false;
    const at = d.location;
    if (!at) {
      g.unplaced += 1;
      continue;
    }
    const file = fileLabel(at.path);
    const place = at.line === undefined ? "" : at.column === undefined ? `${at.line}` : `${at.line}:${at.column}`;
    const list = g.places.get(file);
    if (list) list.push(place);
    else g.places.set(file, [place]);
  }
  return [...groups.values()]
    .sort((a, b) => RANK[a.severity] - RANK[b.severity] || b.count - a.count || a.order - b.order)
    .map((g) => ({
      severity: g.severity,
      code: g.code,
      message: g.same ? g.first : g.template,
      help: g.help,
      count: g.count,
      places: g.places,
      unplaced: g.unplaced,
    }));
}

/** `["44:1", "44:1", "50:1"]` → `["44:1 ×2", "50:1"]`, keeping first-seen order. */
function tally(places: readonly string[]): string[] {
  const counts = new Map<string, number>();
  for (const p of places) counts.set(p, (counts.get(p) ?? 0) + 1);
  return [...counts].map(([p, n]) => (n === 1 ? p : `${p} ×${n}`));
}

/** The whole report, as plain text. */
export function problemReport(
  diagnostics: readonly Diagnostic[],
  fileLabel: (path: string) => string,
  words: ReportPhrases,
): string {
  const count = (s: Severity) => diagnostics.filter((d) => d.severity === s).length;
  const lines = [
    `${words.problems(diagnostics.length)}: ${words.errors(count("error"))}, ` +
      `${words.warnings(count("warning"))}, ${words.notes(count("info"))}`,
  ];
  for (const g of groupProblems(diagnostics, fileLabel)) {
    lines.push("", `${g.severity} ${g.code} — ${words.occurrences(g.count)}`, `  ${g.message}`);
    if (g.help) lines.push(`  ${g.help}`);
    for (const [file, places] of g.places) {
      const shown = tally(places.filter((p) => p !== ""));
      lines.push(shown.length ? `  ${file} ${shown.join(", ")}` : `  ${file}`);
    }
  }
  return lines.join("\n") + "\n";
}
