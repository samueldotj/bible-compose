/**
 * Trim sizes a Bible is likely to be.
 *
 * A suggestion list, not a schema. `page.size` takes any two dimensions —
 * `6x9in`, `210x297mm`, `6in x 9in` — as well as the names the config layer
 * knows, so the control is a text field with these behind it. A publisher
 * setting a trim their press quoted them is the ordinary case in this field,
 * and a closed dropdown would be the application refusing a page it can
 * perfectly well lay out.
 *
 * The names here are the ones a printer, a Bible society or a publisher uses
 * for these sizes; the sizes themselves are what goes in the file. Where a
 * name is BibleCompose's own — `trade`, `compact` — it is written as the
 * dimensions instead, because a dimension is checkable against a printer's
 * quote and a name is not.
 *
 * Two groups, Bibles first and then the standard book trims, because a
 * Scripture publication is a book before it is anything else: a New Testament
 * for a reading programme, a Gospel for a class, a diglot for a college are
 * all ordinary books that happen to hold Scripture, and the trim their printer
 * quotes will be a trade paperback's rather than a pew Bible's.
 */
export interface Trim {
  /** What goes in `page.size`. */
  readonly value: string;
  /** What to call it in the list. */
  readonly name: string;
}

export const TRIMS: readonly Trim[] = [
  { value: "4.25x6.5in", name: "Pocket" },
  { value: "4.75x6.75in", name: "Compact" },
  { value: "5.25x7.75in", name: "Slimline" },
  { value: "5.5x8.5in", name: "Personal" },
  { value: "6x9in", name: "Standard reference — the common Bible trim" },
  { value: "6.5x9.25in", name: "Large print" },
  { value: "7x10in", name: "Study" },
  { value: "8.5x11in", name: "Pulpit / desk" },
  { value: "a6", name: "A6 — 105×148mm" },
  { value: "a5", name: "A5 — 148×210mm" },
  { value: "b5", name: "B5 — 176×250mm" },
  { value: "a4", name: "A4 — 210×297mm" },

  // Standard book trims. Nothing Scripture-specific about them; they are what
  // a printer's price list is written in.
  { value: "4.25x6.87in", name: "Mass-market paperback" },
  { value: "5x8in", name: "Digest" },
  { value: "5.25x8in", name: "Trade paperback" },
  { value: "7.5x9.25in", name: "Textbook" },
  { value: "8x10in", name: "Workbook" },
  { value: "110x178mm", name: "A format — UK mass market" },
  { value: "129x198mm", name: "B format — UK trade paperback" },
  { value: "135x216mm", name: "C format — UK hardback" },
  { value: "156x234mm", name: "Royal octavo" },
  { value: "189x246mm", name: "Crown quarto" },
];
