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

import { EN_HELP } from "./help";

/** Words that belong to no settings key: the chrome of the window itself. */
export interface Chrome {
  readonly appName: string;

  // The welcome screen.
  readonly welcomeKicker: string;
  readonly openProjectEllipsis: string;
  readonly startIntro: string;
  readonly startExistingTitle: string;
  readonly startStepDownloadBefore: string;
  readonly startStepDownloadAfter: string;
  readonly startStepExtract: string;
  readonly startStepSelect: string;
  readonly openBible: string;
  readonly newProject: string;
  readonly newProjectEllipsis: string;
  readonly browse: string;
  readonly choose: string;
  readonly recentProjects: string;
  readonly recentNote: string;
  readonly noLongerThere: string;
  readonly open: string;
  readonly contract: string;
  readonly publicationName: string;
  readonly where: string;
  readonly creates: string;
  readonly create: string;
  readonly creating: string;
  readonly languageTagHint: string;
  readonly language: string;
  readonly noUsfmHere: string;
  readonly noProjectOpen: string;
  readonly loading: string;
  readonly copyUsfmBefore: string;
  readonly copyUsfmAfter: string;

  // The top bar.
  readonly findSetting: string;
  readonly findShortcut: string;
  readonly generatePdf: string;
  readonly cancelBuild: string;
  readonly cancel: string;
  readonly theme: string;
  readonly themeSystem: string;
  readonly themeLight: string;
  readonly themeDark: string;
  readonly reload: string;

  // The rail.
  readonly navPublication: string;
  readonly navText: string;
  readonly navPage: string;
  readonly navType: string;
  readonly navOutput: string;
  readonly railHint: string;
  readonly closeProject: string;

  // The proof spread and its toolbar.
  readonly spreadView: string;
  readonly pageView: string;
  readonly fit: string;
  readonly actualSize: string;
  readonly guides: string;
  readonly showing: string;
  readonly onThePage: string;
  readonly leftPage: string;
  readonly rightPage: string;

  // The inspector's shared words.
  readonly reset: string;
  readonly builtInDefault: string;
  readonly setInProject: string;
  readonly notSet: string;
  readonly dontShow: string;
  readonly notInThisBuild: string;
  readonly settingsRegion: string;

  // Books.
  readonly booksRegion: string;
  readonly filterBooks: string;
  readonly includeAll: string;
  readonly clear: string;
  readonly oldTestament: string;
  readonly newTestament: string;
  readonly deuterocanon: string;
  readonly selectionTitle: string;
  readonly selectionDesc: string;
  readonly inPublication: string;
  readonly nothingIncluded: string;
  readonly orderTitle: string;
  readonly booksFollow: string;
  readonly canonicalOrder: string;
  readonly customOrder: string;
  readonly orderHint: string;
  readonly restoreCanonical: string;
  readonly filesTitle: string;
  readonly openProjectFolder: string;
  readonly notIncluded: string;

  // Template.
  readonly startFrom: string;
  readonly presetNote: string;
  readonly editionsRegion: string;
  readonly templateKicker: string;
  readonly chooseTemplate: string;
  readonly chooseTemplateDesc: string;
  readonly whatItDoes: string;
  readonly applyTemplate: string;
  readonly overwriteSettings: string;
  readonly overwriteWarning: string;

  // Trim & margins.
  readonly trimTitle: string;
  readonly sizeRow: string;
  readonly customSize: string;
  readonly columnsRow: string;
  readonly gutterRow: string;
  readonly gutterHint: string;
  readonly marginsTitle: string;
  readonly top: string;
  readonly bottom: string;
  readonly inner: string;
  readonly outer: string;
  readonly marginsHint: string;
  readonly furnitureTitle: string;
  readonly headRow: string;
  readonly headHint: string;
  readonly footRow: string;
  readonly footHint: string;
  readonly unitIn: string;
  readonly unitMm: string;
  readonly unitPt: string;

  // Headers & footers.
  readonly header: string;
  readonly footer: string;
  readonly leftHeader: string;
  readonly leftFooter: string;
  readonly rightHeader: string;
  readonly rightFooter: string;
  readonly rightPageTitle: string;
  readonly mirrorRow: string;
  readonly mirrorHint: string;
  readonly mirrorButton: string;
  readonly outerSlot: string;
  readonly centreSlot: string;
  readonly innerSlot: string;
  readonly emptySlot: string;
  readonly customSlot: string;
  readonly templateHint: string;
  readonly headFieldsTitle: string;
  readonly headFieldsNote: string;
  readonly fieldsHelp: string;
  readonly fieldColumn: string;
  readonly meaningColumn: string;
  readonly exampleColumn: string;
  readonly close: string;

  // Styles and fonts.
  readonly stylesRegion: string;
  readonly stylesDesc: string;
  readonly inspectDesc: string;
  readonly theBodyFont: string;
  readonly unset: string;
  readonly property: string;
  readonly value: string;
  readonly from: string;
  readonly styleInspector: string;
  readonly filterSelectors: string;
  readonly noElementSelected: string;
  readonly nothingMatches: string;
  readonly chooseFont: string;
  readonly useThisFont: string;
  readonly readingFonts: string;
  readonly searchFonts: string;
  readonly coveringOnly: string;
  readonly noProjectToCheckAgainst: string;
  readonly fontsInProject: string;
  readonly fontsInProjectNote: string;
  readonly fontsBundled: string;
  readonly fontsBundledNote: string;
  readonly fontsInstalled: string;
  readonly fontsInstalledNote: string;
  readonly setsThisScripture: string;
  readonly nothingMatchesCovering: string;

  // Build.
  readonly buildNote: string;
  readonly severity: string;
  readonly whereColumn: string;
  readonly message: string;
  readonly fixIn: string;
  readonly nothingToReport: string;
  readonly nothingMatchesFilter: string;
  readonly allSeverities: string;
  readonly options: string;
  readonly keepHint: string;
  readonly strictHint: string;
  readonly outputFolder: string;
  readonly lastBuild: string;
  readonly openFolder: string;
  readonly openPdf: string;
  readonly stageRead: string;
  readonly stageCheck: string;
  readonly stageEmit: string;
  readonly stageLayout: string;
  readonly stageWrite: string;
  readonly buildIdleTitle: string;
  readonly buildIdleDesc: string;
  readonly building: string;
  readonly completed: string;
  readonly failed: string;
  readonly blocked: string;
  readonly cancelled: string;
  readonly starting: string;
  readonly typesettingProgress: string;
  readonly backendLog: string;

  // The palette.
  readonly paletteTitle: string;
  readonly moveHint: string;
  readonly openHint: string;
  readonly closeHint: string;
  readonly tabWord: string;
  readonly sectionWord: string;

  // The status bar.
  readonly statusIdle: string;
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
  /** A head or foot slot's accessible name: which side, which line, which slot. */
  readonly headerSlot: (side: string, slot: string) => string;
  readonly footerSlot: (side: string, slot: string) => string;
  readonly startFromTemplate: (title: string) => string;
  /** The status bar over a style's row. */
  readonly styleRow: (name: string) => string;
  readonly searchHits: (count: number) => string;
  readonly colourSwatch: (property: string) => string;
  /** `2 of 66 books`, in the top bar. */
  readonly booksOf: (included: number, all: number) => string;
  /** `2 of 66 in the publication`. */
  readonly inPublicationOf: (included: number, all: number) => string;
  readonly chapters: (count: number) => string;
  /** `44 chapters in 2 books`. */
  readonly chaptersInBooks: (chapters: number, books: number) => string;
  /** `10 problems`. */
  readonly problems: (count: number) => string;
  readonly errors: (count: number) => string;
  readonly warnings: (count: number) => string;
  readonly notes: (count: number) => string;
  /** `pages 412–413`, over the spread, after the book's name. */
  readonly pages: (first: string, last: string) => string;
  /** `7 × 10 in · text block 5.45 × 8.05 in`. */
  readonly textBlock: (page: string, block: string) => string;
  /** `page 41 of about 88`. */
  readonly pagesSoFar: (done: number, expected: number | null) => string;
  readonly errorsMustBeFixed: (count: number) => string;
  readonly inheritedFrom: (name: string) => string;
  /** `3 files have changed on disk`. */
  readonly changedOnDisk: (count: number) => string;
  readonly settingsFound: (count: number) => string;
  /** `Theme: dark`, on the toggle. */
  readonly themeIs: (name: string) => string;
  /** `Fix in Contents`, the link on a problem row. */
  readonly booksInFolder: (count: number) => string;
  readonly wrote: (path: string) => string;
  readonly languageWithTag: (name: string, tag: string) => string;
  readonly setInThisProject: (tag: string) => string;
  /** `cannot draw 12 characters`, beside a font. */
  readonly cannotDraw: (count: number) => string;
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
  /** What each control does, by its search key, for the status bar. */
  readonly help: Readonly<Record<string, string>>;
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
  "page.column_gap": "Gutter",
  "page.header_gap": "Head",
  "page.footer_gap": "Foot",
  "typography.font_family": "Font",
  "typography.font_size": "Body size",
  "typography.leading": "Leading",
  "typography.hyphenation": "Hyphenate",
  "numbering.show_chapter_numbers": "Chapter numbers",
  "numbering.show_verse_numbers": "Verse numbers",
  "numbering.hide_first_verse_number": "Hide first verse number",
  "numbering.show_chapter_labels": "Chapter labels",
  "numbering.chapter_number_placement": "Chapter numbers go",
  "numbering.verse_number_placement": "Verse numbers go",
  "numbering.margin_gap": "Gap to the margin numbers",
  "quotes.start": "A quotation begins",
  "quotes.line_indent": "Its line's indent",
  "quotes.hang": "Lines after a quotation begins",
  "quotes.indent_gap": "Extra indent per quotation",
  "contents.show_book_introductions": "Book introductions",
  "contents.show_introductory_outlines": "Introductory outlines",
  "contents.show_section_headings": "Section headings",
  "contents.drop_caps": "Drop caps",
  "contents.drop_cap_of": "What drops",
  "contents.drop_cap_lines": "Lines a drop cap spans",
  "contents.book_starts": "Book",
  "contents.chapter_starts": "Chapter",
  "contents.verse_starts": "Verse",
  "contents.start_verses": "Verses that must fit",
  "typography.justify": "Justify paragraphs",
  "typography.keep_poetry_indentation": "Keep poetry indentation",
  "notes.show_footnotes": "Footnotes",
  "notes.show_cross_references": "Cross-references",
  "notes.footnote_callers": "Footnote marks",
  "notes.cross_reference_callers": "Reference marks",
  "notes.restart_numbering": "Marks start again",
  "notes.cross_reference_placement": "References go",
  "headers.left_page.header_left": "Outer",
  "headers.left_page.header_center": "Centre",
  "headers.left_page.header_right": "Inner",
  "headers.left_page.footer_left": "Outer",
  "headers.left_page.footer_center": "Centre",
  "headers.left_page.footer_right": "Inner",
  "headers.right_page.header_left": "Inner",
  "headers.right_page.header_center": "Centre",
  "headers.right_page.header_right": "Outer",
  "headers.right_page.footer_left": "Inner",
  "headers.right_page.footer_center": "Centre",
  "headers.right_page.footer_right": "Outer",
  "assets.missing_figure": "A figure with no file",
  "output.name": "PDF file name",
  "output.anchors": "PDF bookmarks reach",
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
  first_letter: "The chapter's first letter",
  chapter_number: "The chapter number",
  blank_left_page: "Blank page, then a left page",
  blank_right_page: "Blank page, then a right page",
  blank_next_page: "Blank page, then the next page",
  in_line: "In the line",
  new_line: "On a line of its own",
  at_quote: "Line up with the quotation mark",
  after_quote: "Line up with the word after it",
  in_text: "In the text",
  left_margin: "In the left margin",
  right_margin: "In the right margin",
  next_column: "Next column",
  next_page: "Next page",
  left_page: "Left page",
  right_page: "Right page",
};

/** The English catalogue. */

export const EN: Catalogue = {
  chrome: {
    appName: "BibleCompose",

    welcomeKicker: "Print Bibles from USFM",
    openProjectEllipsis: "Open a project…",
    startIntro:
      "A Bible project is a folder containing Scripture text in USFM format. Open one, or create " +
      "an empty folder to start a new translation.",
    startExistingTitle: "Starting with an existing Bible",
    startStepDownloadBefore: "Download an open-licensed Bible in USFM or Paratext format from",
    startStepDownloadAfter: ".",
    startStepExtract: "Extract the downloaded files to a folder.",
    startStepSelect: "Select that folder as your Bible project directory.",
    openBible: "Open.Bible",
    newProject: "New project",
    newProjectEllipsis: "New project…",
    browse: "Browse…",
    choose: "Choose…",
    recentProjects: "Recent projects",
    recentNote: "Projects you open appear here. Removing one leaves the folder untouched.",
    noLongerThere: "no longer there",
    open: "Open",
    contract: "contract",
    publicationName: "Publication name",
    where: "Where it goes",
    creates: "Creates ",
    create: "Create",
    creating: "Creating…",
    languageTagHint: "a BCP-47 tag, such as ta",
    language: "Language",
    noUsfmHere: "This folder has no USFM in it.",
    noProjectOpen: "No project open.",
    loading: "Loading…",
    copyUsfmBefore: "Copy your USFM files into ",
    copyUsfmAfter: ", then reload.",

    findSetting: "Find a setting…",
    findShortcut: "Ctrl K",
    generatePdf: "Generate PDF",
    cancelBuild: "Cancel build",
    cancel: "Cancel",
    theme: "Theme",
    themeSystem: "system",
    themeLight: "light",
    themeDark: "dark",
    reload: "reload",

    navPublication: "Publication",
    navText: "Text",
    navPage: "Page",
    navType: "Type",
    navOutput: "Output",
    railHint: "Click anything on the page to open its setting.",
    closeProject: "Close project",

    spreadView: "Spread",
    pageView: "Page",
    fit: "Fit",
    actualSize: "100%",
    guides: "Guides",
    showing: "Showing",
    onThePage: "on the page",
    leftPage: "Left page",
    rightPage: "Right page",

    reset: "Reset",
    builtInDefault: "built-in default",
    setInProject: "set in this project",
    notSet: "not set",
    dontShow: "Don't show",
    notInThisBuild: "not in this build",
    settingsRegion: "Settings",

    booksRegion: "Books",
    filterBooks: "Filter books",
    includeAll: "Include all",
    clear: "Clear",
    oldTestament: "Old Testament",
    newTestament: "New Testament",
    deuterocanon: "Deuterocanonical",
    selectionTitle: "Selection",
    selectionDesc: "Turn a book on to include it. Drag to change the order.",
    inPublication: "In the publication",
    nothingIncluded: "No book is in the publication yet.",
    orderTitle: "Order",
    booksFollow: "Books follow",
    canonicalOrder: "Canonical order",
    customOrder: "A custom order",
    orderHint: "Drag a book, or use its arrows, to switch to a custom order.",
    restoreCanonical: "Restore canonical order",
    filesTitle: "Files",
    openProjectFolder: "Open project folder",
    notIncluded: "not included",

    startFrom: "Start from a template",
    presetNote: "Each one writes its settings into the project, where you can change them one at a time.",
    editionsRegion: "Templates",
    templateKicker: "Template",
    chooseTemplate: "Choose a template",
    chooseTemplateDesc:
      "Click one to read what it sets. Applying it writes those settings into this project, " +
      "where each can be changed afterwards.",
    whatItDoes: "What it does",
    applyTemplate: "Apply template",
    overwriteSettings: "Overwrite settings",
    overwriteWarning:
      "This writes the template's settings into your project, replacing any of them you have " +
      "already set. There is no undo.",

    trimTitle: "Trim",
    sizeRow: "Size",
    customSize: "Custom…",
    columnsRow: "Columns",
    gutterRow: "Gutter",
    gutterHint: "Between columns — unused with one",
    marginsTitle: "Margins",
    top: "Top",
    bottom: "Bottom",
    inner: "Inner",
    outer: "Outer",
    marginsHint:
      "Inner is the binding side. A wider bottom than top keeps the block from looking low on the page.",
    furnitureTitle: "Furniture",
    headRow: "Head",
    headHint: "Running head to text block",
    footRow: "Foot",
    footHint: "Text block to page number",
    unitIn: "in",
    unitMm: "mm",
    unitPt: "pt",

    header: "Header",
    footer: "Footer",
    leftHeader: "Left page header",
    leftFooter: "Left page footer",
    rightHeader: "Right page header",
    rightFooter: "Right page footer",
    rightPageTitle: "Right page",
    mirrorRow: "Follows the left page, mirrored",
    mirrorHint: "Turn off to set the right page's six slots separately.",
    mirrorButton: "Mirror left → right",
    outerSlot: "Outer",
    centreSlot: "Centre",
    innerSlot: "Inner",
    emptySlot: "Empty",
    customSlot: "Custom…",
    templateHint: "{Book} {Range}",
    headFieldsTitle: "What a slot can say",
    headFieldsNote:
      "A slot is text with fields in braces: {Book} {Range} reads “1 John 1:1–2:6”, and " +
      "{Book}:{FirstChapter}-{FirstVerse} reads “1 John:1-1”. Names read without regard to case " +
      "or underscores. A slot whose fields all have nothing on a page prints nothing there; a " +
      "field with nothing among fields with something is left out. Write {{ or }} for a brace " +
      "of your own.",
    fieldsHelp: "?",
    fieldColumn: "Field",
    meaningColumn: "What it reads",
    exampleColumn: "For example",
    close: "Close",

    stylesRegion: "Styles",
    stylesDesc: "Unset values fall back to the body font and the style this one inherits from.",
    inspectDesc:
      "Every element the typesetter knows, and where each of its values came from — the " +
      "built-in set, this project, or inheritance.",
    theBodyFont: "the body font",
    unset: "unset",
    property: "Property",
    value: "Value",
    from: "From",
    styleInspector: "Style inspector",
    filterSelectors: "Filter elements",
    noElementSelected: "No element selected.",
    nothingMatches: "Nothing matches.",
    chooseFont: "Choose a font",
    useThisFont: "Use this font",
    readingFonts: "Reading the fonts on this machine…",
    searchFonts: "Search fonts",
    coveringOnly: "Only fonts that can set this Scripture",
    noProjectToCheckAgainst: "No project is open, so nothing has been checked against Scripture.",
    fontsInProject: "In this project",
    fontsInProjectNote: "ships with the book",
    fontsBundled: "Bundled",
    fontsBundledNote: "ships with BibleCompose",
    fontsInstalled: "Installed here",
    fontsInstalledNote: "on this machine only",
    setsThisScripture: "sets this Scripture",
    nothingMatchesCovering: "Nothing matches that can set this Scripture.",

    buildNote: "An error stops the build. Warnings print anyway unless Strict settings is on.",
    severity: "Severity",
    whereColumn: "Where",
    message: "Message",
    fixIn: "Fix in",
    nothingToReport: "Nothing to report.",
    nothingMatchesFilter: "Nothing matches the filter.",
    allSeverities: "All",
    options: "Options",
    keepHint: "Leave the typesetter's files in the output folder",
    strictHint: "Treat a warning in the settings as an error and stop",
    outputFolder: "Output",
    lastBuild: "Last build",
    openFolder: "Open folder",
    openPdf: "Open PDF",
    stageRead: "Read USFM",
    stageCheck: "Check the settings",
    stageEmit: "Prepare the typesetting",
    stageLayout: "Page layout",
    stageWrite: "Write the PDF",
    buildIdleTitle: "Ready",
    buildIdleDesc: "Generate PDF sets the whole publication afresh and writes it to the output folder.",
    building: "Building…",
    completed: "Completed",
    failed: "Failed",
    blocked: "Blocked",
    cancelled: "Cancelled",
    starting: "starting…",
    typesettingProgress: "Typesetting progress",
    backendLog: "Typesetter log",

    paletteTitle: "Find a setting",
    moveHint: "↑↓ move",
    openHint: "↵ open setting",
    closeHint: "Esc close",
    tabWord: "Section",
    sectionWord: "Styles › section",

    statusIdle: "Point at a control to see what it does.",
  },
  phrases: {
    includeBook: (code) => `Include ${code}`,
    moveEarlier: (code) => `Move ${code} earlier`,
    moveLater: (code) => `Move ${code} later`,
    resetSetting: (label) => `Reset ${label}`,
    forgetProject: (name) => `Forget ${name}`,
    headerSlot: (side, slot) => `${side} page, header ${slot}`,
    footerSlot: (side, slot) => `${side} page, footer ${slot}`,
    startFromTemplate: (title) => `Start from “${title}”?`,
    styleRow: (name) => `${name}: how it is set — the properties below.`,
    searchHits: (count) => (count === 1 ? "1 match" : `${count} matches`),
    colourSwatch: (property) => `${property} swatch`,
    booksOf: (included, all) => `${included} of ${all} books`,
    inPublicationOf: (included, all) =>
      included === all ? `All ${all} in the publication` : `${included} of ${all} in the publication`,
    chapters: (count) => (count === 1 ? "1 chapter" : `${count} chapters`),
    chaptersInBooks: (chapters, books) =>
      `${chapters} chapter${chapters === 1 ? "" : "s"} in ${books} book${books === 1 ? "" : "s"}`,
    problems: (count) => (count === 1 ? "1 problem" : `${count} problems`),
    errors: (count) => (count === 1 ? "1 error" : `${count} errors`),
    warnings: (count) => (count === 1 ? "1 warning" : `${count} warnings`),
    notes: (count) => (count === 1 ? "1 note" : `${count} notes`),
    pages: (first, last) => `pages ${first}–${last}`,
    textBlock: (page, block) => `${page} · text block ${block}`,
    pagesSoFar: (done, expected) =>
      expected ? `page ${done} of about ${expected}` : `page ${done}`,
    errorsMustBeFixed: (count) =>
      `${count} error${count === 1 ? "" : "s"} must be fixed before a build can run.`,
    inheritedFrom: (name) => `inherited from ${name}`,
    changedOnDisk: (count) =>
      count === 1 ? "1 file has changed on disk" : `${count} files have changed on disk`,
    settingsFound: (count) => (count === 1 ? "1 setting" : `${count} settings`),
    themeIs: (name) => `Theme: ${name}`,
    booksInFolder: (count) => (count === 1 ? "1 book in the folder" : `${count} books in the folder`),
    wrote: (path) => `wrote ${path}`,
    languageWithTag: (name, tag) => `${name} (${tag})`,
    setInThisProject: (tag) => `${tag} — set in this project`,
    cannotDraw: (count) => `cannot draw ${count} character${count === 1 ? "" : "s"}`,
  },
  states: {
    idle: "Idle",
    loading: "Loading",
    loaded: "Loaded",
    blocked: "Blocked",
    validating: "Checking settings",
    emitting: "Preparing",
    typesetting: "Typesetting",
    publishing: "Publishing",
    succeeded: "Completed",
    failed: "Failed",
    cancelled: "Cancelled",
  },
  labels: EN_LABELS,
  placeholders: EN_PLACEHOLDERS,
  choices: EN_CHOICES,
  help: EN_HELP,
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
