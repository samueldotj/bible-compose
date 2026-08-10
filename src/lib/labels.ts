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

export interface Group {
  readonly id: string;
  readonly title: string;
  readonly keys: readonly string[];
}

export const LABELS: Readonly<Record<string, string>> = {
  "project.name": "Publication name",
  "project.language": "Language tag",
  "books.order": "Order",
  "books.include": "Include only",
  "books.exclude": "Exclude",
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
  "output.file": "Output file",
  "output.keep_intermediates": "Keep intermediates",
  strict: "Strict settings",
};

/** Placeholder text where an empty field means something specific. */
export const PLACEHOLDERS: Readonly<Record<string, string>> = {
  "project.name": "the folder's name",
  "books.order": "canonical",
  "books.include": "every book found",
  "books.exclude": "none",
};

export const GROUPS: readonly Group[] = [
  { id: "project", title: "Project", keys: ["project.name", "project.language"] },
  { id: "books", title: "Books", keys: ["books.order", "books.include", "books.exclude"] },
  {
    id: "page",
    title: "Page",
    keys: [
      "page.size",
      "page.columns",
      "page.margin_top",
      "page.margin_bottom",
      "page.margin_inner",
      "page.margin_outer",
      "page.column_gap",
      "page.header_gap",
      "page.footer_gap",
    ],
  },
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
  {
    id: "content",
    title: "What appears",
    keys: [
      "numbering.show_chapter_numbers",
      "numbering.show_verse_numbers",
      "notes.show_footnotes",
      "notes.show_cross_references",
      "headers.enabled",
      "headers.show_book_name",
      "headers.show_reference_range",
      "headers.show_page_number",
    ],
  },
  {
    id: "output",
    title: "Output",
    keys: ["output.file", "output.keep_intermediates", "strict"],
  },
];

export function labelFor(key: string): string {
  return LABELS[key] ?? key;
}

/**
 * How the configuration is split across tabs.
 *
 * Four rather than two, because "Settings" had become a scroll: the page
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
   * Where a setting belonging to no group ends up. Exactly one tab claims
   * them, so a key added to the schema is visible somewhere rather than
   * nowhere.
   */
  readonly orphans?: boolean;
}

export const TABS: readonly Tab[] = [
  {
    id: "settings",
    title: "Settings",
    settingGroups: ["project", "books", "output"],
    orphans: true,
  },
  { id: "page", title: "Page", settingGroups: ["page"] },
  { id: "appears", title: "What appears", settingGroups: ["content"] },
  { id: "styles", title: "Styles", settingGroups: ["typography"], styles: true },
];
