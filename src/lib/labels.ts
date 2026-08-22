/**
 * The words for settings keys.
 *
 * Here and not in `biblecompose-config`, because these are words shown to a
 * person: they get translated, and the schema does not. The config crate
 * describes *what* each key is; this decides what to call it.
 *
 * A key with no entry still renders — as its own dotted name — so adding a
 * setting to the schema never produces a blank row, only an untranslated one.
 */

import { STYLE_GROUPS } from "./styles";

export interface Group {
  readonly id: string;
  readonly title: string;
  readonly keys: readonly string[];
}

export const LABELS: Readonly<Record<string, string>> = {
  "project.name": "Publication",
  "project.language": "Language",
  "page.size": "Trim size",
  "page.columns": "Columns",
  "page.margin_top": "Top margin",
  "page.margin_bottom": "Bottom margin",
  "page.margin_inner": "Inner margin",
  "page.margin_outer": "Outer margin",
  "page.column_gap": "Column gap",
  "page.header_gap": "Header gap",
  "page.footer_gap": "Footer gap",
  "typography.font_family": "Font",
  "typography.font_size": "Body size",
  "typography.leading": "Leading",
  "typography.hyphenation": "Hyphenate",
  "numbering.show_chapter_numbers": "Chapter numbers",
  "numbering.show_verse_numbers": "Verse numbers",
  "notes.show_footnotes": "Footnotes",
  "notes.show_cross_references": "Cross-references",
  "headers.enabled": "Running heads",
  "headers.show_book_name": "Book name in head",
  "headers.show_reference_range": "Reference range in head",
  "headers.show_page_number": "Page numbers",
  "output.keep_intermediates": "Keep intermediates",
  strict: "Strict settings",
};

/** Placeholder text where an empty field means something specific. */
export const PLACEHOLDERS: Readonly<Record<string, string>> = {
  "project.name": "the folder's name",
};

export const GROUPS: readonly Group[] = [
  {
    id: "typography",
    title: "Typography",
    keys: [
      "typography.font_family",
      "typography.font_size",
      "typography.leading",
      "typography.hyphenation",
    ],
  },
];

/**
 * Keys the settings form does not show, because another part of the window
 * owns them.
 *
 * `books.order` and `books.include` are edited on the book list itself — the
 * ticks and the drag handles *are* the control — and a second set of fields
 * holding the same two values is a second place for them to be edited from
 * and to disagree.
 *
 * Listed rather than silently dropped: the form sweeps up every key no group
 * claims, precisely so a setting added to the schema is visible somewhere
 * rather than nowhere, and an exception to that has to be written down.
 */
export const EDITED_ELSEWHERE: ReadonlySet<string> = new Set([
  // The ticks and the drag handles on the book list are the control.
  "books.order",
  "books.include",
  // What the publication is called and what language it is in: set once, when
  // the folder is opened, so they sit under the button that opens it.
  "project.name",
  "project.language",
  // And every one of these is a switch beside the thing it turns on, on the
  // example page. Same reason as the measurements below: a name in a list and
  // a picture are two places to look, and one of them is the control.
  "numbering.show_chapter_numbers",
  "numbering.show_verse_numbers",
  "notes.show_footnotes",
  "notes.show_cross_references",
  "headers.enabled",
  "headers.show_book_name",
  "headers.show_reference_range",
  "headers.show_page_number",
  // Every page measurement is a field *on the page diagram*, sitting on the
  // thing it measures. A second list of the same nine numbers underneath it
  // would be the drawing and the form disagreeing about which is the control.
  "page.size",
  "page.columns",
  "page.margin_top",
  "page.margin_bottom",
  "page.margin_inner",
  "page.margin_outer",
  "page.column_gap",
  "page.header_gap",
  "page.footer_gap",
  // Questions about the build you are about to run, which belong beside the
  // button that runs it rather than three tabs away from it.
  "output.keep_intermediates",
  "strict",
]);

export function labelFor(key: string): string {
  return LABELS[key] ?? key;
}

/**
 * How the configuration is split across tabs.
 *
 * Four rather than two, because one tab had become a scroll: the page
 * geometry, what appears on it, and how it is set are three separate decisions
 * a publisher makes at three separate times.
 *
 * Typography sits with the styles even though it is a *setting*. The split
 * between the two files is about scope — one body font for the publication,
 * many styles keyed by marker — and that is a distinction the schema has to
 * make and a person choosing a typeface does not.
 */
export interface Tab {
  readonly id: string;
  readonly title: string;
  /** Which of `GROUPS` this tab shows. */
  readonly settingGroups: readonly string[];
  /** Whether the style editor appears below them. */
  readonly styles?: boolean;
  /**
   * Whether the page is drawn above them.
   *
   * Nine numbers in a column do not say which margin is against the spine,
   * and that is the one thing about them a publisher has to get right.
   */
  readonly diagram?: boolean;
  /**
   * Whether a page of Scripture is set above them, with the switches that
   * decide what is on it.
   *
   * "Reference range in head" names a thing without showing it. A publisher
   * who has not seen one cannot tell from the words whether they want it.
   */
  readonly example?: boolean;
  /**
   * Where a setting belonging to no group ends up. Exactly one tab claims
   * them, so a key added to the schema is visible somewhere rather than
   * nowhere.
   */
  readonly orphans?: boolean;
}

export const TABS: readonly Tab[] = [
  { id: "page", title: "Page", settingGroups: [], diagram: true },
  // Claims the strays now that the Project tab is gone. Exactly one tab does,
  // so a key added to the schema is visible somewhere rather than nowhere.
  { id: "appears", title: "What appears", settingGroups: [], example: true, orphans: true },
  { id: "styles", title: "Styles", settingGroups: ["typography"], styles: true },
];

/**
 * The Styles tab's own tabs.
 *
 * One section at a time. Stacked, the seven of them are several screens of
 * form, and a publisher adjusting the poetry indents has no use for the
 * character styles while they do it.
 *
 * Typography leads because it is the one most people change, and because it is
 * the only one of these that is a *setting* — a body font is chosen once for
 * the publication, where every other section is keyed by marker.
 */
export interface SubTab {
  readonly id: string;
  readonly title: string;
  readonly settingGroups: readonly string[];
  readonly styleGroups: readonly string[];
  /** The read-only view over every selector, rather than a form (STY-008). */
  readonly inspector?: boolean;
}

export const STYLE_TABS: readonly SubTab[] = [
  { id: "typography", title: "Typography", settingGroups: ["typography"], styleGroups: [] },
  ...STYLE_GROUPS.map((g) => ({
    id: g.id,
    title: g.title,
    settingGroups: [] as readonly string[],
    styleGroups: [g.id],
  })),
  // Last, because it is where you go when the form above has not answered the
  // question — and it answers for every element rather than the curated ones.
  { id: "inspect", title: "Inspect", settingGroups: [], styleGroups: [], inspector: true },
];
