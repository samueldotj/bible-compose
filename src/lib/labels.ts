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
  "project.author": "Publisher",
  "project.subject": "Subject",
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
  "numbering.hide_first_verse_number": "Hide first verse number",
  "numbering.show_chapter_labels": "Chapter labels",
  "contents.show_book_introductions": "Book introductions",
  "contents.show_introductory_outlines": "Introductory outlines",
  "contents.show_section_headings": "Section headings",
  "typography.justify": "Justify paragraphs",
  "typography.keep_poetry_indentation": "Keep poetry indentation",
  "notes.show_footnotes": "Footnotes",
  "notes.show_cross_references": "Cross-references",
  "notes.footnote_callers": "Footnote marks",
  "notes.cross_reference_callers": "Reference marks",
  "notes.restart_numbering": "Marks start again",
  "notes.cross_reference_placement": "References go",
  "headers.header_left": "Left",
  "headers.header_center": "Centre",
  "headers.header_right": "Right",
  "headers.footer_left": "Left",
  "headers.footer_center": "Centre",
  "headers.footer_right": "Right",
  "assets.missing_figure": "A figure with no file",
  "output.keep_intermediates": "Keep intermediates",
  strict: "Strict settings",
};

/** Placeholder text where an empty field means something specific. */
export const PLACEHOLDERS: Readonly<Record<string, string>> = {
  "project.name": "the folder's name",
  // Both of these end up in the PDF's properties and nowhere else, so the
  // placeholder says what leaving them empty costs, which is nothing.
  "project.author": "left out of the PDF",
  "project.subject": "left out of the PDF",
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
  "numbering.hide_first_verse_number",
  "numbering.show_chapter_labels",
  "contents.show_book_introductions",
  "contents.show_introductory_outlines",
  "contents.show_section_headings",
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
  return LABELS[key] ?? key;
}

/**
 * The words for one option of a `choice` setting.
 *
 * The spellings themselves are the schema's and are not translated — they are
 * what goes in the file. These are what a person reads in a dropdown, so they
 * live here with the rest of the words.
 *
 * Only the ones a rule would get wrong are listed. Everything else falls
 * through to un-snaking, which turns `first_reference` into "First reference"
 * and is right far more often than it is worth an entry.
 */
const CHOICE_WORDS: Readonly<Record<string, string>> = {
  stop: "Stops the build",
  omit: "Is left out",
  note_area: "In the note area",
  inline: "In the text",
  end_of_paragraph: "Under the paragraph",
  none: "No mark",
  numbers: "1, 2, 3",
  letters: "a, b, c",
  symbols: "*, †, ‡",
  alt_book_name: "Alt book name",
};

export function wordsFor(choice: string): string {
  const known = CHOICE_WORDS[choice];
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
