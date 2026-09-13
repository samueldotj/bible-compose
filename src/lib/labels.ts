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
 * `locale()`, so a second locale replaces the words without touching the
 * sections, the groups, or the order of anything (NFR-012). The structures
 * stay here beside the window they describe; only the words travel.
 */

import { locale, t, word } from "./i18n";
import { STYLE_GROUPS } from "./styles";
import type { ExampleTab } from "./switches";

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
  // A setting, but one about how the numbers *look*, so it sits with their
  // styles rather than with the switches that say whether they show.
  { id: "margin_numbers", title: "Numbers in the margin", keys: ["numbering.margin_gap"] },
  // What the PDF says about itself, and what it is called. The publication's
  // name and language lead: they are the PDF's title and language, and this
  // is the one section about the file rather than the page.
  {
    id: "metadata",
    title: "PDF metadata",
    keys: [
      "project.name",
      "project.language",
      "project.author",
      "project.subject",
      "output.name",
      "output.anchors",
    ],
  },
];

/**
 * Keys the generic settings section does not show, because another part of
 * the window owns them.
 *
 * `books.order` and `books.include` are edited on the book list itself — the
 * switches and the drag handles *are* the control — and a second set of
 * fields holding the same two values is a second place for them to be edited
 * from and to disagree.
 *
 * Listed rather than silently dropped: the Contents inspector sweeps up every
 * key no group claims, precisely so a setting added to the schema is visible
 * somewhere rather than nowhere, and an exception to that has to be written
 * down.
 */
export const EDITED_ELSEWHERE: ReadonlySet<string> = new Set([
  // The switches and the drag handles on the book list are the control.
  "books.order",
  "books.include",
  // Every one of these is a row in an inspector beside the page it governs.
  "numbering.show_chapter_numbers",
  "numbering.show_verse_numbers",
  "numbering.hide_first_verse_number",
  "numbering.show_chapter_labels",
  "numbering.chapter_number_placement",
  "numbering.verse_number_placement",
  "quotes.start",
  "quotes.line_indent",
  "quotes.hang",
  "quotes.indent_gap",
  "contents.show_book_introductions",
  "contents.show_introductory_outlines",
  "contents.show_section_headings",
  "contents.drop_caps",
  "contents.drop_cap_of",
  "contents.drop_cap_lines",
  "contents.book_starts",
  "contents.chapter_starts",
  "contents.verse_starts",
  "contents.start_verses",
  "typography.justify",
  "typography.keep_poetry_indentation",
  "notes.show_footnotes",
  "notes.show_cross_references",
  "notes.footnote_callers",
  "notes.cross_reference_callers",
  "notes.restart_numbering",
  "notes.cross_reference_placement",
  // The six slots a side, on the Headers & footers inspector and on the page.
  "headers.left_page.header_left",
  "headers.left_page.header_center",
  "headers.left_page.header_right",
  "headers.left_page.footer_left",
  "headers.left_page.footer_center",
  "headers.left_page.footer_right",
  "headers.right_page.header_left",
  "headers.right_page.header_center",
  "headers.right_page.header_right",
  "headers.right_page.footer_left",
  "headers.right_page.footer_center",
  "headers.right_page.footer_right",
  // Every page measurement is a row of the Trim & margins inspector, drawn
  // as a guide on the spread beside it.
  "page.size",
  "page.columns",
  "page.margin_top",
  "page.margin_bottom",
  "page.margin_inner",
  "page.margin_outer",
  "page.column_gap",
  "page.header_gap",
  "page.footer_gap",
  // Questions about the build you are about to run: on the Build inspector
  // and along the status bar.
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
 * Which kind of centre pane a section has.
 *
 * Most sections show the proof spread, because most decisions are about the
 * page. Books is a table, Template a gallery, Build a list of problems, and
 * the Styles inspector's Inspect view a table over every element.
 */
export type Canvas = "spread" | "books" | "templates" | "build";

/**
 * One entry of the rail: a section of the window, with its inspector.
 *
 * `example` names the switch set the inspector carries when the section is
 * about what is on the page; `settingGroups` the groups of plain settings it
 * shows; `orphans` whether it sweeps up the keys no group and no switch
 * claims, so a setting added to the schema is visible somewhere.
 */
export interface Tab {
  readonly id: string;
  /** The English title, translatable through `word()` by `tab:<id>`. */
  readonly title: string;
  /** The rail section it sits under. */
  readonly nav: "publication" | "text" | "page" | "type" | "output";
  readonly canvas: Canvas;
  readonly settingGroups: readonly string[];
  readonly example?: ExampleTab;
  readonly orphans?: boolean;
  /** The Styles section, whose rail entry unfolds into the style groups. */
  readonly styles?: boolean;
  /** The Headers & footers section: slot controls on the page itself. */
  readonly headers?: boolean;
  /** Trim & margins: guides on the page, measurements in the inspector. */
  readonly trim?: boolean;
  /** The inspector's own width. The book and template inspectors are narrower. */
  readonly narrow?: boolean;
}

/**
 * Outward from the words.
 *
 * Which books there are, then what is printed in the text, then the shape of
 * the sheet, then how the type is set, and last what the file says about
 * itself and how it is made. Each is a smaller decision than the one before
 * and easier once the earlier ones are made.
 */
export const TABS: readonly Tab[] = [
  { id: "books", title: "Books", nav: "publication", canvas: "books", settingGroups: [], narrow: true },
  {
    id: "template",
    title: "Template",
    nav: "publication",
    canvas: "templates",
    settingGroups: [],
    narrow: true,
  },
  // Claims the strays. Exactly one section does, so a key added to the schema
  // is visible somewhere rather than nowhere.
  {
    id: "contents",
    title: "Contents",
    nav: "text",
    canvas: "spread",
    settingGroups: [],
    example: "contents",
    orphans: true,
  },
  { id: "paragraph", title: "Paragraph", nav: "text", canvas: "spread", settingGroups: [], example: "paragraph" },
  { id: "notes", title: "Notes", nav: "text", canvas: "spread", settingGroups: [], example: "notes" },
  { id: "page", title: "Trim & margins", nav: "page", canvas: "spread", settingGroups: [], trim: true },
  { id: "headers", title: "Headers & footers", nav: "page", canvas: "spread", settingGroups: [], headers: true },
  { id: "figures", title: "Figures", nav: "page", canvas: "spread", settingGroups: ["figures"] },
  { id: "styles", title: "Styles", nav: "type", canvas: "spread", settingGroups: [], styles: true },
  { id: "metadata", title: "PDF metadata", nav: "output", canvas: "spread", settingGroups: ["metadata"] },
  { id: "build", title: "Build", nav: "output", canvas: "build", settingGroups: [] },
];

/** The rail's sections, in order, each with its English title. */
export const NAV: readonly { id: Tab["nav"]; title: string }[] = [
  { id: "publication", title: "Publication" },
  { id: "text", title: "Text" },
  { id: "page", title: "Page" },
  { id: "type", title: "Type" },
  { id: "output", title: "Output" },
];

/** A tab's title in the locale in force. */
export function tabTitle(tab: Tab): string {
  return word(`tab:${tab.id}`, tab.title);
}

/** A rail section's title. */
export function navTitle(id: Tab["nav"]): string {
  switch (id) {
    case "publication":
      return t("navPublication");
    case "text":
      return t("navText");
    case "page":
      return t("navPage");
    case "type":
      return t("navType");
    default:
      return t("navOutput");
  }
}

/**
 * The Styles section's own entries, unfolded under it in the rail.
 *
 * Typography leads because it is the one most people change, and because it
 * is the only one of these that is a *setting* — a body font is chosen once
 * for the publication, where every other section is keyed by marker. Inspect
 * is last: it is where you go when the rows above have not answered the
 * question, and it answers for every element rather than the curated ones.
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
    // The chapter-and-verse section carries the one setting about how the
    // numbers look; the others carry none.
    settingGroups: (g.id === "numbers" ? ["margin_numbers"] : []) as readonly string[],
    styleGroups: [g.id],
  })),
  { id: "inspect", title: "Inspect", settingGroups: [], styleGroups: [], inspector: true },
];

export function subTabTitle(sub: SubTab): string {
  return word(`subtab:${sub.id}`, sub.title);
}
