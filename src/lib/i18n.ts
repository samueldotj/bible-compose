/**
 * Every word the window shows, in one place (NFR-012).
 *
 * The requirement is that the initial release ships in English while the
 * *architecture* supports another locale — so what matters is not that a
 * translation exists but that adding one touches nothing else. Hence one
 * catalogue, one shape, and a rule the linter enforces: **no user-facing
 * literal in a component**.
 *
 * # How a second locale is added
 *
 * Write a `Catalogue` — the type below makes every key mandatory, so a partial
 * translation is a compile error rather than a window with three English words
 * left in it — and pass it to {@link setLocale} before the first render. No
 * Rust changes, which is NFR-012's acceptance criterion.
 *
 * # What is deliberately not here
 *
 * **Diagnostic messages.** They come from Rust, are produced by the same code
 * on both front ends, and interpolate values a template would have to be given
 * — a book name, a character count, a path. Translating them needs the message
 * *and* its arguments to cross the wire separately, which is a change to the
 * diagnostic model rather than to this file. What crosses today is a stable
 * `code`, which is the identifier such a catalogue would key on when it comes;
 * until then a diagnostic reads in English in every locale, and says so in the
 * roadmap rather than pretending otherwise.
 *
 * **Marker names.** `\q1` is `\q1` in every language, and a settings key is a
 * schema identifier rather than a word.
 */

/** Words that belong to no settings key: the chrome of the window itself. */
export interface Chrome {
  readonly appName: string;

  // The start screen.
  readonly newProject: string;
  readonly newProjectEllipsis: string;
  readonly browse: string;
  readonly choose: string;
  readonly recent: string;
  readonly publicationName: string;
  readonly where: string;
  readonly creates: string;
  readonly noUsfmHere: string;
  readonly noProjectOpen: string;

  // The build bar.
  readonly generatePdf: string;
  readonly generateDraft: string;
  readonly cancel: string;
  readonly draft: string;
  readonly clean: string;
  readonly cleanHint: string;
  readonly problems: string;
  readonly openFolder: string;
  readonly closeProject: string;
  readonly starting: string;
  readonly typesettingProgress: string;

  // Books.
  readonly selectAll: string;
  readonly clearAll: string;
  readonly canonicalOrder: string;
  readonly oldTestament: string;
  readonly newTestament: string;

  // Styles and fonts.
  readonly property: string;
  readonly value: string;
  readonly chooseFont: string;
  readonly useThisFont: string;
  readonly readingFonts: string;
  readonly noElementSelected: string;
  readonly nothingMatches: string;
  readonly loading: string;
  readonly from: string;
  readonly header: string;
  readonly footer: string;
  readonly booksRegion: string;
  readonly chooseFolder: string;
  readonly exampleName: string;
  readonly language: string;
  readonly canonicalOrderHint: string;
  readonly forgetHint: string;
  readonly restoreHint: string;
  readonly styleInspector: string;
  readonly filterSelectors: string;
  readonly searchFonts: string;
  readonly configurationRegion: string;
  readonly stylesSectionsRegion: string;
  readonly settingsRegion: string;
  readonly stylesRegion: string;
  readonly pageToScale: string;
  readonly commonTrimSizes: string;
  readonly editionsRegion: string;
  readonly startFrom: string;
  readonly presetNote: string;
  readonly use: string;
  readonly overwriteSettings: string;
  readonly coveringOnly: string;
  readonly noProjectToCheckAgainst: string;
}

/**
 * The labels a template builds, as functions.
 *
 * `Include GEN` is two words and a book code, and a locale needs both the
 * words *and* where the code goes — Tamil does not put it where English does.
 * A function is the only shape that carries that, which is why these are not
 * in the flat table above.
 */
export interface Phrases {
  readonly includeBook: (code: string) => string;
  readonly moveEarlier: (code: string) => string;
  readonly moveLater: (code: string) => string;
  readonly resetSetting: (label: string) => string;
  readonly forgetProject: (name: string) => string;
  readonly headerSlot: (slot: string) => string;
  readonly footerSlot: (slot: string) => string;
  readonly colourSwatch: (property: string) => string;
}

/** What the build is doing, in words (GUI-006). */
export interface StateWords {
  readonly idle: string;
  readonly loading: string;
  readonly loaded: string;
  readonly blocked: string;
  readonly validating: string;
  readonly emitting: string;
  readonly typesetting: string;
  readonly publishing: string;
  readonly succeeded: string;
  readonly failed: string;
  readonly cancelled: string;
}

export interface Catalogue {
  readonly chrome: Chrome;
  readonly phrases: Phrases;
  readonly states: StateWords;
  /** Settings keys to their words. A key with no entry renders as itself. */
  readonly labels: Readonly<Record<string, string>>;
  /** What an empty field means, where empty means something specific. */
  readonly placeholders: Readonly<Record<string, string>>;
  /** The spellings a `choice` setting takes, as words. */
  readonly choices: Readonly<Record<string, string>>;
  /**
   * Everything else a person reads, by a stable id.
   *
   * An **override** rather than a source. Tab titles, group titles and the
   * name of each style property are written inline beside the structures they
   * belong to, where a reader of the settings form can see them — moving
   * fifty-odd words into a flat list would make the form unreadable to gain
   * nothing English needs. What a locale needs is an address for each of them,
   * and that is what the id is: an entry here wins, and an id with no entry
   * falls back to the English written beside it.
   *
   * Empty for English, necessarily: there is nothing to override.
   */
  readonly words: Readonly<Record<string, string>>;
}

// ---------------------------------------------------------- the English words
//
// Here rather than beside the structures they title, and the reason is
// mechanical rather than aesthetic: `labels.ts` reads `locale()` from this
// module, so this module importing its maps back is a cycle — and a cycle
// whose symptom is a blank window and `Cannot access 'EN_LABELS' before
// initialization`, because the catalogue below is built at module scope.
// One direction: the words are here, the structures are there.

export const EN_LABELS: Readonly<Record<string, string>> = {
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
  "output.name": "PDF file name",
  "output.keep_intermediates": "Keep intermediates",
  strict: "Strict settings",
};

/** Placeholder text where an empty field means something specific. */
export const EN_PLACEHOLDERS: Readonly<Record<string, string>> = {
  "project.name": "the folder's name",
  "output.name": "named after the publication",
  // Both of these end up in the PDF's properties and nowhere else, so the
  // placeholder says what leaving them empty costs, which is nothing.
  "project.author": "left out of the PDF",
  "project.subject": "left out of the PDF",
};

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
export const EN_CHOICES: Readonly<Record<string, string>> = {
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

/** The English catalogue. */

export const EN: Catalogue = {
  chrome: {
    appName: "BibleCompose",

    newProject: "New project",
    newProjectEllipsis: "New project…",
    browse: "Browse…",
    choose: "Choose…",
    recent: "Recent",
    publicationName: "Publication name",
    where: "Where it goes",
    creates: "Creates ",
    noUsfmHere: "This folder has no USFM in it.",
    noProjectOpen: "No project open.",

    generatePdf: "Generate PDF",
    generateDraft: "Generate draft",
    cancel: "Cancel",
    draft: "Draft",
    clean: "Clean",
    cleanHint: "Run the typesetter even if nothing has changed",
    problems: "Problems",
    openFolder: "Open folder",
    closeProject: "Close project",
    starting: "starting…",
    typesettingProgress: "Typesetting progress",

    selectAll: "Select all",
    clearAll: "Clear all",
    canonicalOrder: "Canonical order",
    oldTestament: "Old Testament",
    newTestament: "New Testament",

    property: "Property",
    value: "Value",
    chooseFont: "Choose a font",
    useThisFont: "Use this font",
    readingFonts: "Reading the fonts on this machine…",
    noElementSelected: "No element selected.",
    nothingMatches: "Nothing matches.",
    loading: "Loading…",
    from: "From",
    header: "Header",
    footer: "Footer",
    booksRegion: "Books",
    chooseFolder: "Choose a folder…",
    exampleName: "My Bible",
    language: "Language",
    canonicalOrderHint: "Put the books back in the order the canon gives them",
    forgetHint: "Remove from this list — the folder is not touched",
    restoreHint: "Restore the built-in value",
    styleInspector: "Style inspector",
    filterSelectors: "Filter selectors",
    searchFonts: "Search fonts",
    configurationRegion: "Configuration",
    stylesSectionsRegion: "Styles sections",
    settingsRegion: "Settings",
    stylesRegion: "Styles",
    pageToScale: "The page, to scale",
    commonTrimSizes: "Common trim sizes",
    editionsRegion: "Editions",
    startFrom: "Start from an edition",
    presetNote:
      "Each one writes its settings into this project, where you can change them one at a time.",
    use: "Use",
    overwriteSettings: "Overwrite settings",
    coveringOnly: "Only fonts that can set this Scripture",
    noProjectToCheckAgainst:
      "No project is open, so nothing has been checked against Scripture.",
  },
  phrases: {
    includeBook: (code) => `Include ${code}`,
    moveEarlier: (code) => `Move ${code} earlier`,
    moveLater: (code) => `Move ${code} later`,
    resetSetting: (label) => `Reset ${label}`,
    forgetProject: (name) => `Forget ${name}`,
    headerSlot: (slot) => `Header ${slot}`,
    footerSlot: (slot) => `Footer ${slot}`,
    colourSwatch: (property) => `${property} swatch`,
  },
  states: {
    idle: "idle",
    loading: "loading",
    loaded: "loaded",
    blocked: "blocked",
    validating: "validating",
    emitting: "generating",
    typesetting: "running SILE",
    publishing: "publishing",
    succeeded: "completed",
    failed: "failed",
    cancelled: "canceled",
  },
  labels: EN_LABELS,
  placeholders: EN_PLACEHOLDERS,
  choices: EN_CHOICES,
  words: {},
};

let active: Catalogue = EN;

/**
 * Use this catalogue from now on.
 *
 * Called before the first render. There is no reactive re-render on a locale
 * change and deliberately so: switching language while a form is half filled
 * in is a feature nobody asked for, and the window is cheap to reopen.
 */
export function setLocale(catalogue: Catalogue): void {
  active = catalogue;
}

/** The catalogue in force. */
export function locale(): Catalogue {
  return active;
}

/** The phrases a template builds. */
export function phrases(): Phrases {
  return active.phrases;
}

/** One word of chrome. The common case, so it gets the short name. */
export function t<K extends keyof Chrome>(key: K): string {
  return active.chrome[key];
}

/**
 * A word written beside its structure, translated if this locale has an
 * opinion about it.
 *
 * `id` is the address a locale keys on and `english` is what is written in the
 * source. Passing both is what lets the structures stay readable while
 * remaining addressable, and means a locale that has not got to a word yet
 * shows the English one rather than a blank or a key.
 */
export function word(id: string, english: string): string {
  return active.words[id] ?? english;
}
