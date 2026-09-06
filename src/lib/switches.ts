/**
 * The switches the example page carries, grouped by the question they
 * answer, and the tab each group is on.
 *
 * One list for three tabs — Contents, Paragraph and Numbering all show the
 * same page of 1 John with a different set of switches beside it — so a
 * switch is added in one place, and the search can say which tab a
 * setting is on by asking this list rather than keeping a second one.
 */

/** Which example tabs there are, besides Headers & Footers. */
export type ExampleTab = "contents" | "paragraph" | "notes";

export interface SwitchRow {
  readonly key: string;
  readonly label: string;
  /** Idle unless this setting is on: there is nothing for it to act on. */
  readonly under?: string;
  /** Decided, and shown on, while this setting is on. */
  readonly implied?: string;
  /** Idle unless one of these settings holds a value that passes. */
  readonly unless?: { readonly keys: readonly string[]; readonly test: (value: string) => boolean };
  readonly note?: string;
  /** For a number: the range the resolver accepts. */
  readonly range?: readonly [number, number];
  /**
   * One dropdown over two settings: a switch that says whether, and a
   * choice that says where. "Don't show" turns the switch off; any other
   * entry turns it on and writes the choice. Two settings that cannot
   * disagree — a place is dormant while the switch is off — shown as the
   * one decision they are.
   */
  readonly combined?: {
    /** The choice setting, whose spellings the entries are. */
    readonly where: string;
    /** The entry for the switch being off. */
    readonly off: string;
    /** What to call each spelling of the choice. */
    readonly labels: Readonly<Record<string, string>>;
  };
}

export interface SwitchGroup {
  readonly title: string;
  readonly tab: ExampleTab;
  /**
   * Groups that share a stack sit one under the other in a column, and the
   * next group without it sits beside that column. Front matter and
   * Numbering are both about what appears; Start is about where.
   */
  readonly stack?: string;
  readonly switches: readonly SwitchRow[];
}

export const SWITCH_GROUPS: readonly SwitchGroup[] = [
  {
    title: "Front matter",
    tab: "contents",
    stack: "appears",
    switches: [
      { key: "contents.show_book_introductions", label: "Book introductions" },
      { key: "contents.show_introductory_outlines", label: "Introductory outlines" },
      { key: "contents.show_section_headings", label: "Section headings" },
    ],
  },
  {
    title: "Paragraph",
    tab: "paragraph",
    switches: [
      { key: "typography.justify", label: "Justify paragraphs" },
      {
        key: "typography.keep_poetry_indentation",
        label: "Keep poetry indentation",
        // 1 John is prose throughout. Saying so beats a switch that looks
        // broken because the passage gives it nothing to do.
        note: "no poetry in this passage",
      },
    ],
  },
  {
    title: "Quotations",
    tab: "paragraph",
    switches: [
      {
        key: "quotes.start",
        label: "A quotation begins",
        note: "Set in the PDF, not on this page. On a line of its own: the line before it ends, and the quotation starts the next, indented — once per level.",
      },
      { key: "quotes.line_indent", label: "Its line's indent" },
      {
        key: "quotes.hang",
        label: "Lines after a quotation begins",
        note: "Set in the PDF, not on this page: the lines after a quotation mark line up with it, and a quotation inside one stands further in.",
      },
      {
        key: "quotes.indent_gap",
        label: "Extra indent per quotation",
      },
    ],
  },
  {
    title: "Drop caps",
    tab: "paragraph",
    switches: [
      {
        key: "contents.drop_caps",
        label: "Drop caps",
        note: "The first verse goes unnumbered; with the first letter dropped, the chapter number takes a line of its own.",
      },
      {
        key: "contents.drop_cap_of",
        label: "What drops",
        under: "contents.drop_caps",
      },
      {
        key: "contents.drop_cap_lines",
        label: "Lines a drop cap spans",
        // Meaningless without an initial to span them.
        under: "contents.drop_caps",
        // The resolver's own bounds, so the field cannot offer a number
        // the file would refuse.
        range: [2, 6],
      },
    ],
  },
  {
    title: "Numbering",
    tab: "contents",
    stack: "appears",
    switches: [
      {
        key: "numbering.show_chapter_numbers",
        label: "Chapter numbers",
        combined: {
          where: "numbering.chapter_number_placement",
          off: "Don't show",
          labels: {
            in_text: "Show in the text",
            left_margin: "In the left margin",
            right_margin: "In the right margin",
          },
        },
      },
      {
        key: "numbering.show_chapter_labels",
        label: "Chapter labels",
        // A translation either carries `\cl` or it does not, and most do
        // not — so say that this switch may have nothing to act on.
        note: "USFM's \\cl, where a translation has it",
      },
      {
        key: "numbering.show_verse_numbers",
        label: "Verse numbers",
        combined: {
          where: "numbering.verse_number_placement",
          off: "Don't show",
          labels: {
            in_text: "Show in the text",
            left_margin: "In the left margin",
            right_margin: "In the right margin",
          },
        },
      },
      {
        key: "numbering.hide_first_verse_number",
        label: "Hide first verse number",
        // Nothing to hide when no verse number is shown at all.
        under: "numbering.show_verse_numbers",
        // And nothing to decide under a dropped initial, which is the
        // first verse's marker.
        implied: "contents.drop_caps",
      },
    ],
  },
  {
    title: "Start",
    tab: "contents",
    switches: [
      {
        key: "contents.book_starts",
        label: "Book",
        note: "Where the next book begins. One book here, so nothing to show.",
      },
      {
        key: "contents.chapter_starts",
        label: "Chapter",
        note: "Where every chapter but a book's first begins. Chapter 2 stays put here: the page is a page.",
      },
      { key: "contents.verse_starts", label: "Verse" },
      {
        key: "contents.start_verses",
        label: "Verses that must fit",
        note: "Whatever the start above: a book or chapter with fewer than this of its verses fitting in the column moves to the next column. 0 turns it off.",
        range: [0, 30],
      },
    ],
  },
  {
    title: "Notes",
    tab: "notes",
    switches: [
      { key: "notes.show_footnotes", label: "Footnotes" },
      { key: "notes.footnote_callers", label: "Footnote marks", under: "notes.show_footnotes" },
      { key: "notes.show_cross_references", label: "Cross-references" },
      {
        key: "notes.cross_reference_callers",
        label: "Reference marks",
        under: "notes.show_cross_references",
      },
      {
        key: "notes.cross_reference_placement",
        label: "References go",
        under: "notes.show_cross_references",
      },
      {
        key: "notes.restart_numbering",
        label: "Marks start again",
        // Both sequences, and the only boundary this passage has is the
        // chapter — so say which one the example can actually show.
        note: "at chapter 2, in this passage",
      },
    ],
  },
];
