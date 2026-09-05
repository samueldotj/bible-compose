/**
 * The words for settings keys, and the structures they title.
 *
 * Here and not in `biblecompose-config`, because these are words shown to a
 * person: they get translated, and the schema does not. The config crate
 * describes *what* each key is; this decides what to call it.
 *
 * A key with no entry still renders — as its own dotted name — so adding a
 * setting to the schema never produces a blank row, only an untranslated one.
 *
 * **The `EN_` maps are the English half of the catalogue in `i18n.ts`** and
 * are exported for it to assemble. Everything read at runtime goes through
 * `locale()`, so a second locale replaces the words without touching the tabs,
 * the groups, or the order of anything (NFR-012). The structures stay here
 * beside the form they describe; only the words travel.
 */

import { locale } from "./i18n";
import { STYLE_GROUPS } from "./styles";

export interface Group {
  readonly id: string;
  readonly title: string;
  readonly keys: readonly string[];
}



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
  // What a figure with no file does to the build.
  { id: "figures", title: "Figures", keys: ["assets.missing_figure"] },
  // What the PDF says about itself, and what it is called. None of it
  // changes a page, which is why it has a tab of its own rather than a
  // corner of Contents.
  {
    id: "metadata",
    title: "PDF metadata",
    keys: ["project.author", "project.subject", "output.name", "output.anchors"],
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
  "numbering.hide_first_verse_number",
  "numbering.show_chapter_labels",
  "contents.show_book_introductions",
  "contents.show_introductory_outlines",
  "contents.show_section_headings",
  "contents.drop_caps",
  // Beside the Drop caps switch, since it is meaningless without it.
  "contents.drop_cap_lines",
  "typography.justify",
  "typography.keep_poetry_indentation",
  "notes.show_footnotes",
  "notes.show_cross_references",
  "notes.footnote_callers",
  "notes.cross_reference_callers",
  "notes.restart_numbering",
  "notes.cross_reference_placement",
  "headers.header_left",
  "headers.header_center",
  "headers.header_right",
  "headers.footer_left",
  "headers.footer_center",
  "headers.footer_right",
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
  return locale().labels[key] ?? key;
}

/** What an empty field means, where empty means something specific. */
export function placeholderFor(key: string): string | undefined {
  return locale().placeholders[key];
}




export function wordsFor(choice: string): string {
  const known = locale().choices[choice];
  if (known !== undefined) return known;
  const words = choice.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
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
   * Whether this tab is the templates — the three editions a project can be
   * started from.
   *
   * Its own tab rather than a section of Page, where it began: a template
   * rewrites a dozen settings across every other tab at once, and a control
   * that does that sitting above the margin fields read as one more margin
   * field. Second in the strip because it is the second decision — which
   * Scripture, then which kind of book — and everything after it is
   * adjustment.
   */
  readonly template?: boolean;
  /**
   * Which set of switches the example page carries, if this tab has one.
   *
   * "Reference range in head" names a thing without showing it. A publisher
   * who has not seen one cannot tell from the words whether they want it — so
   * the page is the control, and the two tabs that use it take a switch set
   * each: what is in the text, and what surrounds it.
   */
  readonly example?: "contents" | "headers";
  /**
   * Where a setting belonging to no group ends up. Exactly one tab claims
   * them, so a key added to the schema is visible somewhere rather than
   * nowhere.
   */
  readonly orphans?: boolean;
  /**
   * The books, which are a tab of their own rather than a column beside every
   * other one.
   *
   * They were in a permanent left-hand pane, and it cost the whole window a
   * third of its width on every tab — including the ones where the answer to
   * "which books" has already been given and the question is what the page
   * looks like. A whole Bible is sixty-six rows and wants the width; a settings
   * form beside it had none to spare.
   */
  readonly books?: boolean;
}

/**
 * Outward from the words.
 *
 * Which books there are, then what is printed in the text, then what surrounds
 * it, then the shape of the sheet it all sits on, and last how it is set. Each
 * one is a smaller decision than the one before it and is easier to make once
 * the earlier ones are made — the page size is worth arguing about after you
 * know whether the edition carries footnotes, not before.
 */
export const TABS: readonly Tab[] = [
  { id: "scripture", title: "Scripture", settingGroups: [], books: true },
  { id: "template", title: "Template", settingGroups: [], template: true },
  // Claims the strays now that the Project tab is gone. Exactly one tab does,
  // so a key added to the schema is visible somewhere rather than nowhere.
  { id: "contents", title: "Contents", settingGroups: [], example: "contents", orphans: true },
  {
    id: "headers",
    title: "Headers & Footers",
    settingGroups: [],
    example: "headers",
  },
  { id: "page", title: "Page", settingGroups: [], diagram: true },
  { id: "styles", title: "Styles", settingGroups: ["typography"], styles: true },
  { id: "figures", title: "Figures", settingGroups: ["figures"] },
  // Last, because it is the one decision that changes nothing on a page.
  { id: "metadata", title: "PDF metadata", settingGroups: ["metadata"] },
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
