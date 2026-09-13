/**
 * Everything the window can take you to, and a way to find it by name.
 *
 * Eleven sections, a Styles section with nine entries, a hundred and forty
 * style selectors and fifty-odd settings is more than anyone holds in their
 * head, and "where is the thing that hides the first verse number" is a
 * question the palette answers in three keystrokes.
 *
 * **The index is built from what the window already knows**, not from a
 * list kept here: the settings come from the schema the backend sent, the
 * styles from the editor's own groups, the presets from the backend. A
 * setting added next release is searchable the day it appears, because
 * nothing here has to hear about it. What this file decides is only where
 * each thing *lives* — which section, which entry — so that choosing a hit
 * can go there.
 */

import { EDITED_ELSEWHERE, GROUPS, STYLE_TABS, TABS, labelFor, subTabTitle, tabTitle } from "./labels";
import { locale, t } from "./i18n";
import { STYLE_GROUPS } from "./styles";
import { SWITCH_GROUPS } from "./switches";
import type { Preset, Setting } from "./services/backend";

/** One thing the search can find. */
export interface Hit {
  /** Stable, unique: what a list keys on. */
  readonly id: string;
  /** What it is called. */
  readonly title: string;
  /** Where it is — `Contents › Numbering` — for the eye. */
  readonly path: string;
  /** A line under the name: the value in force, or what the thing does. */
  readonly hint?: string;
  /** The section to open, or none for something visible from every section. */
  readonly tab?: string;
  /** The entry of the Styles section, when that is the section. */
  readonly subtab?: string;
  /** The `data-search-key` of the element to light up, when there is one. */
  readonly key?: string;
}

/** Where a setting is edited: its section, and the group on that section. */
export interface Home {
  readonly tab?: string;
  readonly subtab?: string;
  readonly section?: string;
}

const title = (tab: string | undefined) => {
  const found = TABS.find((x) => x.id === tab);
  return found ? tabTitle(found) : "";
};

/**
 * Where a setting lives.
 *
 * A grouped key lives where its group is shown. The rest are the ones the
 * generic section does not show because some part of the window owns them
 * ({@link EDITED_ELSEWHERE}), and which part is a matter of the key's
 * prefix — a page measurement is on Trim & margins, a head slot on Headers &
 * footers, a book on the book list. Anything else is an orphan, and the
 * section that claims orphans shows it.
 */
export function homeOf(key: string): Home | null {
  for (const group of GROUPS) {
    if (!group.keys.includes(key)) continue;
    const tab = TABS.find((x) => x.settingGroups.includes(group.id));
    if (tab) return { tab: tab.id, section: group.title };
    const sub = STYLE_TABS.find((s) => s.settingGroups.includes(group.id));
    if (sub) return { tab: "styles", subtab: sub.id, section: group.title };
  }
  if (key.startsWith("page.")) return { tab: "page" };
  if (key.startsWith("headers.")) return { tab: "headers" };
  if (key.startsWith("books.")) return { tab: "books" };
  if (key === "output.keep_intermediates" || key === "strict") return { tab: "build" };
  // A switch on one of the proof-spread sections: the group it is in says
  // which — and a setting folded into another's dropdown is on that row.
  for (const group of SWITCH_GROUPS) {
    if (group.switches.some((s) => s.key === key || s.combined?.where === key)) {
      return { tab: group.tab, section: group.title };
    }
  }
  if (EDITED_ELSEWHERE.has(key)) {
    const tab = TABS.find((x) => x.example === "contents");
    return tab ? { tab: tab.id } : null;
  }
  const orphans = TABS.find((x) => x.orphans);
  return orphans ? { tab: orphans.id } : null;
}

/** A setting's value as a person reads it, with where it came from. */
function hintFor(setting: Setting): string {
  const value =
    setting.kind === "boolean"
      ? setting.value === "true"
        ? "on"
        : "off"
      : setting.kind === "choice"
        ? (locale().choices[setting.value] ?? setting.value.replace(/_/g, " "))
        : setting.value || "—";
  const at = setting.location;
  const from = setting.overridden
    ? at
      ? at.line
        ? `${at.path.split(/[/\\]/).pop()}:${at.line}`
        : at.path
      : t("setInProject")
    : t("builtInDefault");
  return `${value} · ${from}`;
}

/** The whole index, from what the window holds right now. */
export function index(settings: readonly Setting[], presets: readonly Preset[] | null): Hit[] {
  const out: Hit[] = [];
  const help = locale().help;

  for (const tab of TABS) {
    out.push({
      id: `tab:${tab.id}`,
      title: tabTitle(tab),
      path: t("tabWord"),
      hint: help[`tab:${tab.id}`],
      tab: tab.id,
    });
  }
  for (const sub of STYLE_TABS) {
    out.push({
      id: `subtab:${sub.id}`,
      title: subTabTitle(sub),
      path: t("sectionWord"),
      hint: help[`subtab:${sub.id}`],
      tab: "styles",
      subtab: sub.id,
    });
  }

  for (const setting of settings) {
    const home = homeOf(setting.key);
    if (home === null) continue;
    const where = [
      home.tab ? title(home.tab) : "",
      home.subtab ? STYLE_TABS.find((s) => s.id === home.subtab)?.title : undefined,
      home.section,
    ]
      .filter((p): p is string => Boolean(p))
      .join(" › ");
    // A setting folded into another's dropdown lights that row.
    const folded = SWITCH_GROUPS.flatMap((g) => g.switches).find(
      (s) => s.combined?.where === setting.key,
    );
    out.push({
      id: `setting:${setting.key}`,
      title: labelFor(setting.key),
      path: where,
      hint: hintFor(setting),
      tab: home.tab,
      subtab: home.subtab,
      key: folded?.key ?? setting.key,
    });
  }

  for (const group of STYLE_GROUPS) {
    for (const row of group.rows) {
      const path = `${title("styles")} › ${group.title}`;
      out.push({
        id: `style:${row.selector}`,
        title: row.label,
        path,
        hint: row.selector,
        tab: "styles",
        subtab: group.id,
        key: `style:${row.selector}`,
      });
      for (const property of row.properties) {
        out.push({
          id: `style:${row.selector}.${property.name}`,
          title: `${row.label} · ${property.label}`,
          path,
          hint: help[`property:${property.name}`],
          tab: "styles",
          subtab: group.id,
          key: `style:${row.selector}.${property.name}`,
        });
      }
    }
  }

  for (const preset of presets ?? []) {
    out.push({
      id: `preset:${preset.id}`,
      title: preset.title,
      path: title("template"),
      hint: preset.description,
      tab: "template",
      key: `preset:${preset.id}`,
    });
  }

  return out;
}

/**
 * The hits for a query, best first.
 *
 * Every word of the query has to appear somewhere in a hit — its title, its
 * path, or the key underneath — and a hit whose *title* begins with the
 * query outranks one that merely contains it, which outranks one matched
 * only by where it lives. Twelve at most: a list longer than that is a
 * list nobody reads.
 */
export function search(hits: readonly Hit[], query: string, limit = 12): Hit[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [];
  const words = q.split(/\s+/);
  const scored: { hit: Hit; score: number }[] = [];
  for (const hit of hits) {
    const t = hit.title.toLowerCase();
    const haystack = `${t} ${hit.path.toLowerCase()} ${hit.key?.toLowerCase() ?? ""}`;
    if (!words.every((w) => haystack.includes(w))) continue;
    const score = t.startsWith(q)
      ? 0
      : t.split(/[\s·]+/).some((word) => word.startsWith(q))
        ? 1
        : t.includes(q)
          ? 2
          : 3;
    scored.push({ hit, score });
  }
  // At an equal match, what is most often meant comes first: a setting,
  // then a section, then a style, then a template.
  const kind = (hit: Hit) =>
    hit.id.startsWith("setting:") ? 0 : hit.id.startsWith("tab:") || hit.id.startsWith("subtab:") ? 1 : hit.id.startsWith("style:") ? 2 : 3;
  scored.sort(
    (a, b) => a.score - b.score || kind(a.hit) - kind(b.hit) || a.hit.title.localeCompare(b.hit.title),
  );
  return scored.slice(0, limit).map((s) => s.hit);
}
