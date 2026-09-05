/**
 * Which styles the editor offers, and what to call them.
 *
 * The schema has around a hundred and forty selectors and nine properties
 * each. Showing all of that is a spreadsheet, not an editor — so this is the
 * curated set GUI-004 names: paragraph spacing, heading size, poetry indent,
 * chapter and verse appearance, footnote style, and the common character
 * styles. Body font and size are settings rather than styles and live in the
 * settings form.
 *
 * Curation belongs here and not in `biblecompose-config` for the same reason
 * labels do: it is a decision about what to put in front of a person. The
 * schema still holds everything, the inspector can still show all of it, and a
 * publisher who wants a selector this list omits can write it in `styles.toml`
 * by hand and the cascade will honour it.
 */

export type PropertyKind =
  | "length"
  | "integer"
  | "boolean"
  | "align"
  | "font"
  | "color"
  | "choice";

export interface PropertyRow {
  readonly name: string;
  readonly label: string;
  readonly kind: PropertyKind;
  /** For a `choice`: the spellings the file takes, and what to call each. */
  readonly choices?: readonly { readonly value: string; readonly label: string }[];
}

export interface StyleRow {
  readonly selector: string;
  readonly label: string;
  readonly properties: readonly PropertyRow[];
}

export interface StyleGroup {
  readonly id: string;
  readonly title: string;
  readonly rows: readonly StyleRow[];
}

const FACE: PropertyRow = { name: "font_family", label: "Font", kind: "font" };
const SIZE: PropertyRow = { name: "font_size", label: "Size", kind: "length" };
const WEIGHT: PropertyRow = { name: "weight", label: "Weight", kind: "integer" };
const ITALIC: PropertyRow = { name: "italic", label: "Italic", kind: "boolean" };
const SMALLCAPS: PropertyRow = { name: "smallcaps", label: "Small caps", kind: "boolean" };
const ABOVE: PropertyRow = { name: "space_above", label: "Space above", kind: "length" };
const BELOW: PropertyRow = { name: "space_below", label: "Space below", kind: "length" };
const INDENT: PropertyRow = { name: "indent", label: "Indent", kind: "length" };
const RAISE: PropertyRow = { name: "raise", label: "Raise", kind: "length" };
const ALIGN: PropertyRow = { name: "align", label: "Alignment", kind: "align" };
const COLOR: PropertyRow = { name: "color", label: "Colour", kind: "color" };

// The chapter number's own decisions. Where it sits, whether it takes a line
// or drops into the text, whether a chapter opens a column or a page.
const BORDER: PropertyRow = { name: "border", label: "Border", kind: "boolean" };
const BORDER_WIDTH: PropertyRow = { name: "border_width", label: "Border thickness", kind: "length" };
const OWN_LINE: PropertyRow = { name: "own_line", label: "On its own line", kind: "boolean" };
const GAP_BEFORE: PropertyRow = { name: "gap_before", label: "Gap before", kind: "length" };
const GAP_AFTER: PropertyRow = { name: "gap_after", label: "Gap after", kind: "length" };
const NEW_COLUMN: PropertyRow = { name: "new_column", label: "Starts a new column", kind: "boolean" };
const NEW_PAGE: PropertyRow = {
  name: "new_page",
  label: "Starts a new page",
  kind: "choice",
  choices: [
    { value: "continue", label: "No — continue" },
    { value: "next", label: "Next page" },
    { value: "left", label: "Left page" },
    { value: "right", label: "Right page" },
  ],
};

// Alignment was missing here while the schema, the cascade and the class all
// supported it, so a centred section heading — one of the most ordinary
// decisions in Bible design — could only be made by editing TOML.
const HEADING = [FACE, SIZE, WEIGHT, ITALIC, ALIGN, ABOVE, BELOW, COLOR];
const CHARACTER = [WEIGHT, ITALIC, SMALLCAPS, COLOR];

export const ALIGNMENTS = ["start", "center", "end", "justify"] as const;

/**
 * Every property, for the inspector.
 *
 * STY-008 asks what each property of an element is *and where it came from*,
 * which includes the ones nothing decides — "not set" is an answer, and one a
 * publisher wondering why a heading has no space above it needs.
 *
 * The editor's groups are a subset of these chosen per selector; this is the
 * whole set, in the order the schema lists them.
 */
export const ALL_PROPERTIES: readonly PropertyRow[] = [
  FACE,
  SIZE,
  WEIGHT,
  ITALIC,
  SMALLCAPS,
  ABOVE,
  BELOW,
  INDENT,
  RAISE,
  ALIGN,
  COLOR,
  BORDER,
  BORDER_WIDTH,
  OWN_LINE,
  GAP_BEFORE,
  GAP_AFTER,
  NEW_COLUMN,
  NEW_PAGE,
];

export const STYLE_GROUPS: readonly StyleGroup[] = [
  {
    id: "headings",
    title: "Headings",
    rows: [
      { selector: "heading.s1", label: "Section", properties: HEADING },
      { selector: "heading.s2", label: "Subsection", properties: HEADING },
      { selector: "heading.r1", label: "Parallel references", properties: HEADING },
      { selector: "heading.d1", label: "Psalm superscription", properties: HEADING },
      { selector: "heading.sp1", label: "Speaker", properties: HEADING },
    ],
  },
  {
    id: "paragraphs",
    title: "Paragraphs",
    rows: [
      { selector: "paragraph.p", label: "Body", properties: [ABOVE, BELOW, INDENT, ALIGN] },
      { selector: "paragraph.pc", label: "Centred", properties: [ABOVE, BELOW, ALIGN] },
      { selector: "paragraph.ip", label: "Introduction", properties: [SIZE, ITALIC, ABOVE, BELOW] },
    ],
  },
  {
    id: "poetry",
    title: "Poetry",
    rows: [
      { selector: "poetry.q1", label: "Level 1", properties: [INDENT, ABOVE, BELOW] },
      { selector: "poetry.q2", label: "Level 2", properties: [INDENT] },
      { selector: "poetry.q3", label: "Level 3", properties: [INDENT] },
      { selector: "poetry.q4", label: "Level 4", properties: [INDENT] },
      { selector: "poetry.qc1", label: "Centred line", properties: [ALIGN] },
      { selector: "poetry.qr1", label: "Right-aligned line", properties: [ALIGN] },
    ],
  },
  {
    id: "numbers",
    title: "Chapter and verse",
    rows: [
      {
        selector: "chapter",
        label: "Chapter number",
        properties: [
          FACE,
          SIZE,
          WEIGHT,
          ITALIC,
          COLOR,
          ALIGN,
          OWN_LINE,
          ABOVE,
          BELOW,
          GAP_BEFORE,
          GAP_AFTER,
          BORDER,
          BORDER_WIDTH,
          NEW_COLUMN,
          NEW_PAGE,
        ],
      },
      { selector: "verse", label: "Verse number", properties: [SIZE, WEIGHT, RAISE, COLOR] },
    ],
  },
  {
    id: "notes",
    title: "Notes",
    rows: [
      { selector: "note.f", label: "Footnote", properties: [FACE, SIZE, ITALIC] },
      { selector: "reference", label: "Cross-reference", properties: [FACE, SIZE, ITALIC] },
    ],
  },
  {
    id: "characters",
    title: "Character styles",
    rows: [
      { selector: "character.bd", label: "Bold", properties: CHARACTER },
      { selector: "character.it", label: "Italic", properties: CHARACTER },
      { selector: "character.em", label: "Emphasis", properties: CHARACTER },
      { selector: "character.add", label: "Added words", properties: CHARACTER },
      { selector: "character.nd", label: "Divine name", properties: CHARACTER },
      { selector: "character.wj", label: "Words of Jesus", properties: CHARACTER },
      { selector: "character.qt", label: "Quoted text", properties: CHARACTER },
    ],
  },
  {
    id: "furniture",
    title: "Page furniture",
    rows: [
      { selector: "head", label: "Running head", properties: [SIZE, ITALIC] },
      { selector: "folio", label: "Page number", properties: [SIZE, ITALIC] },
      { selector: "caption", label: "Figure caption", properties: [SIZE, ITALIC] },
    ],
  },
];

/** What the editor calls a selector, where it has a name for it. */
export function labelForSelector(selector: string): string | undefined {
  for (const group of STYLE_GROUPS) {
    const row = group.rows.find((r) => r.selector === selector);
    if (row) return `${group.title} · ${row.label}`;
  }
  return undefined;
}
