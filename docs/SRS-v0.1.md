<!--
  Converted from BibleCompose_Software_Requirements_Specification_v0.1.docx,
  which was the original deliverable and remains in git history.

  This is the source document, not a design document: it states what the
  product must do, and every requirement ID the design cites (PRJ-001,
  SILE-005, NFR-009 and the rest) is defined here. The analysis of it is in
  SRS-REVIEW.md, and nothing in this file has been edited to agree with
  that analysis -- where the two differ, the review says so and this stands
  as written.
-->

# BibleCompose

### Software Requirements Specification

Rust desktop application for USFM → SILE → PDF composition

| Document version | 0.1 — Initial requirements draft |
|---|---|
| Product name | BibleCompose |
| Status | Proposed / working specification |
| Date | 7 August 2026 |
| Primary output | Publication-quality PDF |

Purpose

Define the product behavior, file contracts, user experience, architecture boundaries, quality attributes, and acceptance criteria for the first production-capable release of BibleCompose.

## Document Control

| Version | Date | Status | Summary |
|---|---|---|---|
| 0.1 | 7 Aug 2026 | Draft | Initial requirements based on the defined BibleCompose product concept. |

### Reading Guide

Requirement keywords use the following meanings:

- MUST — required for the specified release and acceptance.
- SHOULD — strongly desired; may be deferred only with an explicit release decision.
- MAY — optional or future-facing capability.

### Contents

1. Executive Summary

2. Product Vision and Scope

3. Definitions and Assumptions

4. Project Folder and File Contract

5. Users and Primary Workflows

6. Functional Requirements

7. Configuration Requirements

8. Style System Requirements

9. USFM Support Requirements

10. SILE Integration and PDF Generation

11. GUI Requirements

12. Software Architecture

13. Non-Functional Requirements

14. Diagnostics and Error Handling

15. Security and Data Handling

16. Testing and Acceptance

17. MVP Scope

18. Post-MVP Roadmap

19. Open Decisions

20. Technical References

## 1. Executive Summary

BibleCompose is a cross-platform desktop application written primarily in Rust for composing Bible publications from USFM source files. A user opens a project folder containing one or more Bible books in USFM. BibleCompose discovers and validates the books, merges built-in defaults with optional project settings and style overrides, converts the Scripture content into an internal document model, generates SILE-compatible composition input, invokes SILE as the typesetting backend, and produces a PDF.

The product goal is to provide a simpler, modern, Scripture-aware composition workflow while keeping the typesetting engine separate from the Bible-specific application logic. SILE is treated as a rendering backend, not as the project data model or user-facing configuration language.

### 1.1 Product at a Glance

```text
Project folder
  ├── *.usfm / *.sfm
  ├── biblecompose.toml   (optional)
  ├── styles.toml         (optional)
  └── assets/             (optional)
          │
          ▼
     BibleCompose
  Rust project loader
  USFM parser + validator
  Scripture document model
  settings/style resolver
          │
          ▼
     SILE adapter
          │
          ▼
        SILE
          │
          ▼
   publication PDF
```

### 1.2 Core Design Principles

- Project folder is the source of truth. A BibleCompose project does not require a proprietary project database.
- Sensible defaults first. A folder containing valid USFM should be enough to generate a usable PDF.
- Configuration is declarative. Project behavior and visual styles are stored as readable TOML rather than application-internal state.
- Bible semantics are independent of SILE. USFM is parsed into a structured Rust model before any SILE source is produced.
- Reproducible builds. The same source, settings, styles, assets, BibleCompose version, and SILE backend should produce materially equivalent output.
- No Scripture text is modified merely to achieve a visual layout change.

## 2. Product Vision and Scope

### 2.1 Vision

BibleCompose should make professional Bible composition approachable through a native desktop GUI while retaining a programmable typesetting backend. It should be suitable for Reader Bibles, standard single-column or double-column Bibles, and progressively more complex Bible editions as the product matures.

### 2.2 Goals

- Generate a PDF directly from a folder of USFM books with no mandatory configuration files.
- Allow project-level page, typography, book-order, header/footer, numbering, and output settings.
- Allow USFM marker styles to be overridden without editing Scripture source files.
- Provide clear validation and composition diagnostics with book/chapter/verse or file/line context.
- Hide SILE implementation details from ordinary users while still allowing advanced diagnostics and intermediate-file inspection.
- Support Unicode Scripture and complex-script shaping through the SILE backend.
- Create a maintainable Rust codebase in which parsing, semantics, styling, GUI, and rendering are independently testable.

### 2.3 Non-Goals for the First Release

- Bible translation or collaborative text editing comparable to Paratext.
- Full PTXprint feature parity.
- General-purpose desktop publishing comparable to Adobe InDesign.
- Automatic language translation, spell checking, or theological content generation.
- DBL project management, cloud synchronization, or team workflow.
- Interactive drag-and-drop placement of arbitrary text boxes on individual pages.

## 3. Definitions and Assumptions

| Term | Meaning |
|---|---|
| Project | A folder opened by BibleCompose containing Scripture sources and optional BibleCompose configuration/assets. |
| USFM | Unified Scripture Format Markup; the primary Scripture input format. |
| Book ID | Canonical three-character Scripture identifier such as GEN, PSA, MAT, JHN, REV. |
| Settings file | Optional biblecompose.toml containing publication-wide behavior and layout settings. |
| Style file | Optional styles.toml containing visual and layout rules keyed primarily by USFM marker/style. |
| Resolved configuration | Built-in defaults merged with project settings and style overrides. |
| Document model | BibleCompose’s normalized Rust representation of books, paragraphs, verses, notes, references, figures, and inline spans. |
| SILE adapter | The component that transforms the internal document model and resolved styles into input consumable by SILE and invokes the SILE backend. |
| Build | One attempt to transform the currently loaded project into a PDF. |
| Diagnostic | A structured error, warning, or informational message associated with a project file or semantic location. |

### 3.1 Assumptions

- UTF-8 is the default and required encoding for newly supported project files. Files with a byte-order mark may be tolerated.
- Projects may contain all 66 Protestant-canon books or any subset; BibleCompose must not require a complete Bible.
- Book order defaults to canonical order but may be overridden.
- SILE is bundled with the desktop application where licensing and packaging permit; otherwise BibleCompose must provide a guided backend-location configuration.
- Project files remain editable in external tools; BibleCompose must not introduce hidden lock-in metadata into USFM files.
- The application is designed to operate fully offline.

## 4. Project Folder and File Contract

### 4.1 Minimal Project

```text
MyBible/
  GEN.usfm
  EXO.usfm
  ...
  MAT.usfm
  ...
  REV.usfm
```

With this minimal structure, BibleCompose uses built-in settings and built-in USFM styles and must be capable of producing a PDF without requiring any additional files.

### 4.2 Full Project Example

```text
MyBible/
  biblecompose.toml
  styles.toml
  books/
    01GEN.usfm
    02EXO.usfm
    ...
  assets/
    images/
      creation-map.pdf
      ark.jpg
    fonts/
      ProjectTamil-Regular.ttf
  output/                 # optional; may be generated
  .biblecompose/          # generated cache/intermediate data; optional
```

### 4.3 Discovery Rules

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| PRJ-001 | The application shall allow the user to open a folder as a BibleCompose project. | MUST | Open Folder selects a directory and initiates discovery. |
| PRJ-002 | The application shall recursively discover .usfm and .sfm files in the selected project folder unless excluded by configuration. | MUST | Nested books/ folder is detected without manual file registration. |
| PRJ-003 | Each Scripture file shall be identified primarily from its USFM \id marker, not from filename alone. | MUST | A renamed MAT file still loads as MAT when \id MAT is present. |
| PRJ-004 | The application shall detect duplicate canonical book IDs and block PDF generation until the ambiguity is resolved. | MUST | Two files declaring \id MAT produce a blocking diagnostic. |
| PRJ-005 | The application shall support projects containing a subset of Bible books. | MUST | A project containing only JHN builds successfully. |
| PRJ-006 | The application should ignore generated output/cache directories during USFM discovery. | SHOULD | output/ and .biblecompose/ do not create duplicate inputs. |
| PRJ-007 | The project shall remain portable as a normal filesystem directory. | MUST | Moving the folder does not require database migration when relative asset paths are preserved. |

## 5. Users and Primary Workflows

### 5.1 Primary User

The primary user is a Bible publisher, translator, typesetter, ministry worker, or technically comfortable editor who already has Scripture in USFM and wants a reproducible print/PDF composition workflow without manually authoring SILE code.

### 5.2 Primary Workflow

1. Launch BibleCompose.
1. Choose Open Project Folder.
1. BibleCompose discovers books, loads optional configuration, parses and validates the project.
1. User reviews detected books and any diagnostics.
1. User optionally changes settings or styles through the GUI; changes are saved to project TOML files.
1. User selects Build PDF.
1. BibleCompose generates SILE input and invokes SILE.
1. Build progress and warnings appear in the GUI.
1. On success, BibleCompose opens the generated PDF in the integrated preview or the user’s system viewer.
1. User iterates on settings/styles and rebuilds.

## 6. Functional Requirements

### 6.1 Project Loading and Validation

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| FUN-001 | The application shall parse all discovered Scripture files into a normalized in-memory document model. | MUST | Project model exposes books, chapters, paragraphs, verses, character spans, notes, references, and figures found in source. |
| FUN-002 | The application shall preserve source order and content unless a documented normalization is required. | MUST | Round-trip model tests show no silent Scripture text deletion. |
| FUN-003 | Unknown or unsupported USFM markers shall generate structured diagnostics and shall not be silently discarded. | MUST | Unsupported marker is reported with file and location. |
| FUN-004 | The application shall distinguish blocking errors from non-blocking warnings. | MUST | Build button/state reflects whether blocking errors exist. |
| FUN-005 | The application shall expose detected metadata including book ID, file path, chapter count, and validation status in the project UI. | MUST | Project explorer shows per-book status. |
| FUN-006 | The user shall be able to reload the project after external file changes without closing the application. | MUST | Reload reparses and refreshes diagnostics. |
| FUN-007 | The application should detect external changes to project files and offer or perform safe reload. | SHOULD | Editing MAT.usfm externally triggers a changed-file indication. |

### 6.2 Build and Output

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| BLD-001 | The user shall be able to start PDF generation from the GUI. | MUST | Build PDF initiates the pipeline. |
| BLD-002 | A successful build shall create exactly one primary PDF at the resolved output path. | MUST | Output file exists and is non-empty. |
| BLD-003 | The default output filename shall be derived from project name, with a configurable override. | MUST | Project MyBible defaults to MyBible.pdf unless overridden. |
| BLD-004 | The application shall not overwrite source USFM during a build. | MUST | Source checksums are unchanged after build. |
| BLD-005 | The application shall capture SILE standard output, warnings, and errors and translate them into the build log. | MUST | Backend failures are visible without opening a terminal. |
| BLD-006 | The application shall support canceling an in-progress build and terminating the SILE child process safely. | MUST | Cancel stops composition and restores usable UI state. |
| BLD-007 | The application should support a clean build that discards caches/intermediate generated files. | SHOULD | Clean Build regenerates all derived artifacts. |
| BLD-008 | The application should provide an option to retain generated SILE source and intermediate files for debugging. | SHOULD | Debug setting exposes generated files under .biblecompose/build/. |
| BLD-009 | A failed build shall not replace the last known good PDF unless the new PDF was completed successfully. | MUST | Existing PDF remains intact after a forced backend error. |

### 6.3 Book Selection and Ordering

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| BOOK-001 | Detected books shall default to canonical Bible order rather than filesystem order. | MUST | GEN precedes EXO even if filenames sort differently. |
| BOOK-002 | Project settings shall allow explicit book ordering. | MUST | Configured order is reflected in the PDF. |
| BOOK-003 | Project settings shall allow books to be included or excluded without deleting source files. | MUST | Excluded book does not appear in output. |
| BOOK-004 | The GUI should provide checkboxes or equivalent controls for included books. | SHOULD | User can toggle a book and rebuild. |
| BOOK-005 | The application may later support named publication profiles such as NT, Reader Edition, Large Print, or Custom. | MAY | Deferred roadmap capability. |

### 6.4 Scripture Features

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| SCR-001 | Chapter and verse markers shall be retained semantically even when configured to be visually hidden. | MUST | Reader-style output can hide numbers without losing reference anchors in the document model. |
| SCR-002 | Paragraph, poetry, section heading, list, and character-level styles shall map to configurable BibleCompose styles. | MUST | Different USFM markers render with distinct resolved styles. |
| SCR-003 | USFM footnotes shall be parsed as structured note content and rendered through a SILE footnote strategy. | MUST | A source \f note appears at the intended note location in PDF. |
| SCR-004 | USFM cross-references shall be parsed independently from footnotes. | MUST | A source \x reference is represented as a cross-reference object. |
| SCR-005 | MVP shall support rendering cross-references as footnote-area references or inline/end-of-paragraph references; advanced center-column reference layout may be deferred. | MUST | At least one stable configurable cross-reference presentation exists. |
| SCR-006 | USFM figure markers shall be parsed with source, alt/caption/reference metadata where present. | MUST | A supported figure renders from a relative project asset path. |
| SCR-007 | The application shall support hiding chapter numbers, verse numbers, section headings, footnotes, and cross-references independently through settings. | MUST | Each feature can be disabled without altering USFM. |
| SCR-008 | The application should preserve logical reference anchors in generated PDF where technically feasible. | SHOULD | Internal destinations/bookmarks can be added in a later minor release without model redesign. |

## 7. Configuration Requirements

### 7.1 Configuration Files

BibleCompose uses two optional TOML files. If either file is absent, embedded defaults are used. If a file is present, only specified values override defaults; users are not required to copy the complete default configuration into each project.

```text
project/
  biblecompose.toml   # publication and behavior settings
  styles.toml         # marker/style overrides
  *.usfm
```

### 7.2 Settings Merge Rules

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| CFG-001 | BibleCompose shall contain an embedded default settings configuration. | MUST | A USFM-only folder builds. |
| CFG-002 | If biblecompose.toml exists, project values shall override embedded defaults field-by-field. | MUST | Changing only page.width does not erase unrelated defaults. |
| CFG-003 | Invalid TOML syntax shall produce a blocking diagnostic containing filename and line/column where available. | MUST | Malformed TOML cannot silently fall back to defaults. |
| CFG-004 | Unknown settings keys shall produce a warning by default and may be treated as errors in strict mode. | MUST | Typo such as page.wdith is visible. |
| CFG-005 | The GUI shall be able to save supported setting changes back to biblecompose.toml. | MUST | GUI change persists after reopening project. |
| CFG-006 | The application shall avoid rewriting unrelated formatting/comments in TOML where reasonably practical; if full preservation is not feasible, it must save deterministic valid TOML. | SHOULD | Config save behavior is predictable. |
| CFG-007 | A user shall be able to reset a setting to inherited/default behavior. | MUST | Removing override restores built-in value. |

### 7.3 Proposed biblecompose.toml

```toml
[project]
name = "My Bible"
language = "ta"
books = ["GEN", "EXO", "MAT", "MRK", "LUK", "JHN"]

[page]
size = "6x9in"
columns = 2
margin_top = "0.55in"
margin_bottom = "0.55in"
margin_inner = "0.70in"
margin_outer = "0.50in"
column_gap = "0.18in"

[typography]
font_family = "Noto Serif Tamil"
font_size = "11.5pt"
line_spacing = 1.15
language = "ta"
hyphenation = true

[numbering]
show_chapter_numbers = true
show_verse_numbers = true

[notes]
show_footnotes = true
show_cross_references = true
cross_reference_placement = "footnote-area"

[headers]
enabled = true
show_book_name = true
show_reference_range = true

[output]
file = "output/MyBible.pdf"
keep_intermediates = false
```

### 7.4 Settings Categories Required for v1

| Category | Examples / responsibilities |
|---|---|
| Project | name, language/locale, book selection/order, optional metadata |
| Page | paper/trim size, orientation, margins, columns, column gap |
| Typography | primary font, optional fallback fonts, body font size, line spacing, hyphenation/language |
| Numbering | show/hide chapter and verse numbers; basic chapter/verse styling delegated to styles.toml |
| Notes | footnote visibility, cross-reference visibility and supported placement mode |
| Headers/Footers | enabled state, book name/reference/page-number content, basic alignment |
| Output | output file/path, overwrite policy, keep intermediates/debug mode |
| Backend | bundled SILE selection or advanced custom executable path; timeout/debug behavior |
| Images | base asset path, default max width, missing-image policy |

## 8. Style System Requirements

### 8.1 Purpose

styles.toml defines the visual treatment of semantic elements. The style layer must allow a publisher to change typography and spacing without editing the underlying USFM. BibleCompose shall ship with a complete default style set for the USFM markers supported by the release.

### 8.2 Style Cascade

```text
Built-in BibleCompose style defaults
              ↓
Project styles.toml overrides
              ↓
Resolved style map
              ↓
SILE emission
```

### 8.3 Proposed styles.toml

```toml
[paragraph.p]
font_size = "11.5pt"
alignment = "justify"
first_line_indent = "0.16in"
space_before = "0pt"
space_after = "0pt"

[paragraph.q1]
left_indent = "0.18in"
first_line_indent = "0in"
space_before = "2pt"

[paragraph.q2]
left_indent = "0.34in"
first_line_indent = "0in"

[paragraph.s1]
font_size = "13pt"
font_weight = 600
space_before = "10pt"
space_after = "4pt"
keep_with_next = true

[chapter]
font_size = "20pt"
font_weight = 700

[verse]
font_size = "7.5pt"
position = "superscript"

[character.bd]
font_weight = 700

[character.it]
font_style = "italic"

[footnote]
font_size = "8.5pt"
line_spacing = 1.05
```

### 8.4 Style Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| STY-001 | BibleCompose shall contain built-in styles for every USFM marker it claims to support. | MUST | Supported marker never requires a project override to render. |
| STY-002 | Project styles.toml shall override built-in styles by semantic selector. | MUST | paragraph.q1 override changes only q1 unless inheritance is defined. |
| STY-003 | The style engine shall distinguish paragraph-level, character-level, chapter, verse, note, reference, figure, header/footer, and peripheral styles. | MUST | Selectors cannot collide merely because marker names are similar. |
| STY-004 | Unsupported style properties shall generate diagnostics rather than being silently ignored. | MUST | Misspelled style property is reported. |
| STY-005 | The GUI shall expose common style properties without requiring TOML editing. | MUST | User can edit body font, size, paragraph spacing, heading size, poetry indent, and chapter/verse appearance. |
| STY-006 | Advanced users shall be able to edit styles.toml externally and reload. | MUST | External change is reflected after reload. |
| STY-007 | Styles should support inheritance to minimize duplication, provided inheritance remains deterministic and inspectable. | SHOULD | A q2 style may inherit common poetry properties. |
| STY-008 | The application should provide an inspector showing the resolved style and source of each property (default vs project override). | SHOULD | User can diagnose why an element looks a certain way. |

## 9. USFM Support Requirements

### 9.1 Compatibility Target

The internal semantic model should align with the USFM 3.1 content model while remaining tolerant of common legacy USFM 2.x project files. Validation may distinguish between strict conformance and compatibility warnings. BibleCompose is not required to implement every USFM marker in its first release, but supported and unsupported markers must be explicit.

### 9.2 MVP Marker Support

| Area | Markers / scope | Priority |
|---|---|---|
| Identification / metadata | id, ide, h, toc1, toc2, toc3 | MUST |
| Titles | mt1–mt4, mte1–mte2 | MUST |
| Introductions | is1–is4, ip, im, iq1–iq4, ili1–ili2 | SHOULD |
| Chapters / verses | c, ca, cp, v, va, vp | MUST |
| Paragraphs | p, m, po, pr, cls, pmo, pm, pmc, pmr, pi1–pi3, mi, nb, pc, ph1–ph3 | MUST for common p/m/pi/nb/pc; others SHOULD |
| Poetry | q1–q4, qr, qc, qa, qm1–qm4, qd, b | MUST for q1–q4 and b; others SHOULD |
| Sections | s1–s4, sr, r, d, sp | MUST for s1–s4; others SHOULD |
| Lists | li1–li4, lim1–lim4 | SHOULD |
| Character styles | add, bd, bdit, em, it, nd, no, sc, sup, wj, qt, sig, tl, k, dc, pn, ord, w | MUST for common emphasis/name-of-Deity/words-of-Jesus styles; rest SHOULD |
| Footnotes | f, fe and standard footnote submarkers such as fr, ft, fq, fqa, fk, fl, fw, fp, fv, fd | MUST |
| Cross references | x and common submarkers such as xo, xt, xk, xq, xot, xnt, xdc | MUST |
| Figures | fig and defined attributes such as src, alt, size, loc, copy, ref | MUST |
| Tables | tr, th#, thr#, tc#, tcr# | SHOULD |
| Milestones / advanced attributes | USFM 3 milestones and user-defined attributes | MAY for v1; must be preserved/diagnosed if unsupported |

### 9.3 Parsing and Preservation

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| USFM-001 | The parser shall produce source spans or equivalent location metadata sufficient to associate diagnostics with files and approximate line/column positions. | MUST | Malformed marker diagnostic navigates to the relevant source location. |
| USFM-002 | Nested character markers and note submarkers shall be represented structurally rather than flattened into plain text. | MUST | Formatting and note semantics survive parsing. |
| USFM-003 | Character attributes defined by supported USFM versions shall be preserved in the internal model. | MUST | Figure/word/reference attributes remain available to renderer. |
| USFM-004 | Unknown markers shall be preserved in a generic node or explicit unsupported-node representation whenever feasible. | SHOULD | Future support can be added without changing parser architecture. |
| USFM-005 | The parser shall never silently merge verse text across book boundaries or reorder Scripture content. | MUST | Corpus-level ordering tests pass. |
| USFM-006 | Validation shall detect missing/invalid \id, malformed chapter/verse numbers, unclosed character spans, malformed notes, duplicate book IDs, and invalid asset references at minimum. | MUST | Fixture suite emits expected diagnostics. |

## 10. SILE Integration and PDF Generation

### 10.1 Integration Strategy

The first implementation should treat SILE as a versioned rendering backend invoked through a dedicated adapter. BibleCompose should generate a controlled intermediate SILE document/package rather than emitting SILE commands throughout the application. This boundary allows the backend implementation to evolve without coupling GUI or USFM parsing code to SILE syntax.

```text
Rust core
  Project → USFM AST → Scripture Document Model
                         ↓
                  Resolved Layout Model
                         ↓
                   SileEmitter trait
                         ↓
             generated .sil + resources
                         ↓
                SILE child process
                         ↓
                       PDF
```

### 10.2 Backend Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| SILE-001 | BibleCompose shall invoke SILE only through a dedicated backend adapter interface. | MUST | No UI module shells out to SILE directly. |
| SILE-002 | The application shall detect and report the SILE backend version used for each build. | MUST | Build log records backend version. |
| SILE-003 | Desktop distributions should bundle a tested SILE runtime and required BibleCompose SILE packages. | SHOULD | Fresh install can build without separately installing SILE. |
| SILE-004 | An advanced setting may allow an alternate SILE executable path for development/testing. | MAY | Developer can test newer backend without replacing bundled runtime. |
| SILE-005 | The generated SILE input shall be deterministic for identical normalized input and resolved configuration, excluding intentionally variable metadata. | MUST | Golden-file tests remain stable. |
| SILE-006 | Backend stderr/stdout shall be captured and associated with the active build. | MUST | No backend error is lost in a hidden console. |
| SILE-007 | SILE-specific failures shall be converted to understandable BibleCompose diagnostics where mapping is possible, while retaining raw backend details in an expandable log. | MUST | User sees concise error plus technical details. |
| SILE-008 | The application shall clean temporary intermediate files after a successful build unless keep_intermediates is enabled. | MUST | Temporary build directory is removed or retained according to setting. |
| SILE-009 | SILE custom packages used by BibleCompose shall be versioned together with BibleCompose releases. | MUST | A release has a known application/backend package compatibility set. |

### 10.3 PDF Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| PDF-001 | Generated output shall be a standards-compliant PDF readable by current mainstream PDF viewers. | MUST | PDF opens successfully in at least two independent viewers during release testing. |
| PDF-002 | Page size, margins, columns, fonts, text direction, and supported images shall reflect resolved settings/styles. | MUST | Golden PDF geometry checks pass. |
| PDF-003 | Fonts required for correct text display shall be embedded or otherwise handled in a print-safe manner when licensing permits. | MUST | Preflight confirms expected font embedding. |
| PDF-004 | Unicode Scripture text shall render with no missing-glyph boxes when the configured font set supports the characters. | MUST | Complex-script fixture renders correctly. |
| PDF-005 | The system should support PDF metadata including title, language, author/publisher, and subject where provided. | SHOULD | Metadata is visible in PDF properties. |
| PDF-006 | Print-specific standards such as PDF/X may be added after the base PDF workflow is stable. | MAY | Roadmap item, not MVP blocker. |

## 11. GUI Requirements

### 11.1 Main Window

The GUI should expose a conventional project-oriented desktop workflow. The exact Rust GUI framework is an implementation decision, but the application UI itself should be implemented in or tightly controlled by the Rust application and should not require a browser-based server process.

```text
┌──────────────────────────────────────────────────────────────────┐
│ BibleCompose   [Open Project] [Reload] [Build PDF] [Cancel]      │
├───────────────┬───────────────────────────────┬──────────────────┤
│ Project       │ PDF Preview / Build Summary   │ Settings / Style │
│               │                               │ Inspector         │
│ ✓ GEN         │                               │                  │
│ ✓ EXO         │                               │ Page             │
│ ! LEV         │                               │ Typography       │
│ ...           │                               │ Notes            │
│ ✓ MAT         │                               │ Headers          │
│               │                               │ Output           │
├───────────────┴───────────────────────────────┴──────────────────┤
│ Diagnostics / Build Log                                          │
└──────────────────────────────────────────────────────────────────┘
```

### 11.2 GUI Functional Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| GUI-001 | The user shall be able to open a project folder from the main window. | MUST | Folder picker loads the selected project. |
| GUI-002 | The project pane shall list detected books in resolved order with included/excluded and validation status. | MUST | Book list updates after load/reload. |
| GUI-003 | The GUI shall provide access to core settings without requiring manual TOML editing. | MUST | Page size, margins, columns, font, body size, numbering, notes, and output path are editable. |
| GUI-004 | The GUI shall provide a style editor for at least body paragraphs, poetry, section headings, chapter numbers, verse numbers, footnotes, and common character styles. | MUST | Common style values can be changed and persisted. |
| GUI-005 | The GUI shall show diagnostics with severity, message, book/file, and source location when available. | MUST | Clicking a diagnostic selects the related book and shows detail. |
| GUI-006 | The GUI shall show build state: idle, validating, generating, running SILE, completed, failed, canceled. | MUST | Status is visible throughout build. |
| GUI-007 | The GUI shall provide a build log with copyable technical output. | MUST | User can copy error details for support. |
| GUI-008 | After a successful build, the application should show an integrated PDF preview. | SHOULD | First pages can be inspected without leaving BibleCompose. |
| GUI-009 | If integrated preview is unavailable on a platform, the application shall provide Open PDF and Open Output Folder actions. | MUST | Generated PDF is easy to access. |
| GUI-010 | Unsaved settings/style edits shall be visibly indicated and protected from accidental project close. | MUST | Close/reopen flow does not silently discard changes. |
| GUI-011 | The application should support light and dark appearance following the operating-system preference. | SHOULD | Theme changes preserve readability. |
| GUI-012 | The UI should remain responsive while parsing or typesetting by running long work off the UI thread. | MUST | Window remains interactive during build and cancel is usable. |

## 12. Software Architecture

### 12.1 Proposed Rust Workspace

```text
biblecompose/
  crates/
    biblecompose-app/          # desktop entry point + GUI composition
    biblecompose-core/         # orchestration and domain services
    biblecompose-project/      # folder discovery and project model
    biblecompose-usfm/         # parser, AST, validation, source spans
    biblecompose-model/        # normalized Scripture document model
    biblecompose-config/       # TOML settings, defaults, merge, schema
    biblecompose-style/        # style selectors, cascade, validation
    biblecompose-layout/       # backend-neutral layout model
    biblecompose-sile/         # SILE emitter, runtime invocation, diagnostics
    biblecompose-diagnostics/  # shared error/warning model
    biblecompose-testkit/      # fixtures and golden test helpers
  sile/
    classes/
    packages/
  defaults/
    biblecompose.toml
    styles.toml
```

### 12.2 Architectural Boundaries

| Component | Responsibility / boundary |
|---|---|
| Project Layer | Discovers files, resolves relative paths, tracks project metadata. Must not know SILE syntax. |
| USFM Layer | Parses and validates Scripture markup. Must not know GUI widgets or PDF details. |
| Document Model | Normalized semantic representation. Stable boundary between source parsing and composition. |
| Configuration Layer | Loads defaults + TOML overrides, validates values/units, exposes typed settings. |
| Style Layer | Resolves marker/semantic styles into typed style objects; no raw SILE strings in project files. |
| Layout Layer | Converts semantic content plus style/config into backend-neutral layout intentions where practical. |
| SILE Backend | Maps layout/document structures to SILE and manages runtime process/resources. |
| GUI Layer | Presents project/config/diagnostics/build functions and calls core services asynchronously. |
| Diagnostics Layer | Common typed errors/warnings shared by parser, config, style, asset, and SILE stages. |

### 12.3 Recommended Internal Data Flow

```text
ProjectSource
   ↓
UsfmDocument (source-aware AST)
   ↓ normalization
ScriptureDocument
   ├─ Book
   │   ├─ Chapter/verse anchors
   │   ├─ Paragraphs / poetry / lists
   │   ├─ Inline spans
   │   ├─ Footnotes
   │   ├─ CrossReferences
   │   └─ Figures
   ↓ + ResolvedConfig + ResolvedStyles
CompositionDocument
   ↓
SileDocument
   ↓
PDF
```

### 12.4 Key Implementation Decisions

- Use typed Rust structures for settings and styles; do not pass arbitrary TOML values deep into rendering code.
- Use a stable diagnostic type containing severity, code, message, source path, optional line/column, optional Scripture reference, and causal detail.
- Use async/background task execution for project parsing and SILE invocation; long-running work must not block the GUI event loop.
- Prefer a child-process integration with SILE for v1 because it creates a strong failure boundary and simplifies backend replacement/testing.
- Bundle BibleCompose-specific SILE classes/packages in the application release rather than generating every low-level layout primitive ad hoc.
- Treat generated SILE as an intermediate build artifact, not as a user-maintained source format.

## 13. Non-Functional Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| NFR-001 | The application shall support Windows, macOS, and Linux where SILE runtime packaging is feasible; Windows and macOS are Tier-1 release targets unless the project decides otherwise. | SHOULD | CI builds and smoke tests exist for supported platforms. |
| NFR-002 | Opening a standard 66-book USFM project should complete initial parse/validation within 5 seconds on a contemporary desktop for typical Scripture-only projects. | SHOULD | Performance fixture meets target on reference hardware. |
| NFR-003 | The GUI shall remain responsive during builds lasting minutes. | MUST | UI interaction and cancel remain usable. |
| NFR-004 | BibleCompose shall not require an internet connection for project load, validation, composition, or PDF generation. | MUST | Offline end-to-end test succeeds. |
| NFR-005 | The application shall use UTF-8 internally and preserve Unicode text without lossy conversions. | MUST | Round-trip Unicode fixtures pass. |
| NFR-006 | Builds shall be reproducible enough for regression testing; generated intermediate text should be deterministic. | MUST | Golden SILE outputs are stable. |
| NFR-007 | A crash or failed build shall not corrupt source USFM or existing configuration files. | MUST | Fault-injection tests show source integrity. |
| NFR-008 | Configuration and style schemas shall be versionable to permit controlled future evolution. | MUST | Version field or migration strategy is documented before schema-breaking changes. |
| NFR-009 | The core parser, configuration resolver, style resolver, and SILE emitter shall be testable without launching the GUI. | MUST | Automated unit/integration tests run headless. |
| NFR-010 | Application logs shall avoid recording Scripture content unnecessarily; diagnostics should include only the context required to identify a problem. | SHOULD | Review of logs confirms no full-corpus dumping by default. |
| NFR-011 | User-facing controls shall be keyboard navigable and use readable labels; accessibility semantics should be supported by the chosen GUI toolkit. | SHOULD | Basic keyboard-only workflow succeeds. |
| NFR-012 | The application should support localized GUI strings in the architecture even if the initial release ships in English only. | SHOULD | UI strings are not hard-coded throughout business logic. |

## 14. Diagnostics and Error Handling

### 14.1 Diagnostic Model

```text
Diagnostic {
  severity: Error | Warning | Info
  code: "USFM-UNTERMINATED-CHAR"
  message: "Character style \bd is not closed"
  file: "books/MAT.usfm"
  line: 153
  column: 18
  reference: "Matthew 5:3"   # optional
  help: "Add \bd* before ..." # optional
}
```

### 14.2 Required Error Classes

- Project discovery errors: inaccessible directory, unreadable file, duplicate book ID.
- USFM syntax/semantic errors: missing ID, malformed markers, invalid chapter/verse values, unclosed spans/notes.
- Configuration errors: invalid TOML, invalid enum/value/unit, unknown key.
- Style errors: unknown selector/property, invalid font size/unit/value, cyclic inheritance if supported.
- Asset errors: missing image/font, unsupported image format, path escaping project policy if restricted.
- Backend errors: SILE executable missing, backend version incompatible, non-zero exit, timeout/cancel, output PDF absent.
- Output errors: destination unwritable, file locked, disk full, unsafe overwrite.

### 14.3 Diagnostic UX Requirements

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| DIA-001 | Errors shall use stable machine-readable codes in addition to human-readable text. | MUST | Tests assert diagnostic codes. |
| DIA-002 | A build blocked by validation shall explain the blocking issues before SILE is invoked. | MUST | No backend process starts when required input is invalid. |
| DIA-003 | Where safe, warnings shall allow the build to proceed. | MUST | Unknown non-critical metadata may warn without blocking. |
| DIA-004 | The diagnostics panel shall support filtering by severity and book/file. | SHOULD | User can focus on errors in one book. |
| DIA-005 | Raw SILE logs shall be available for debugging but collapsed by default behind user-friendly diagnostics. | MUST | Technical details are accessible without overwhelming normal workflow. |

## 15. Security and Data Handling

- BibleCompose operates on local files selected by the user and shall not upload Scripture or project metadata by default.
- Paths passed to SILE shall be properly escaped/quoted and never constructed through unsafe shell concatenation. Prefer direct process APIs with argument arrays.
- Generated SILE input must treat Scripture and configuration values as data. User-controlled content shall not be allowed to inject arbitrary Lua/SILE execution unless an explicit advanced extension mode is designed later.
- Relative asset references should resolve inside the project directory by default. Access outside the project may be allowed only through explicit configuration and must be visible to the user.
- Temporary build directories should use operating-system safe temporary-file APIs and be cleaned according to configuration.
- No telemetry is required for core functionality. If telemetry is ever added, it must be opt-in or clearly disclosed and must not transmit Scripture text.

## 16. Testing and Acceptance

### 16.1 Automated Test Layers

- Parser unit tests for valid and invalid USFM snippets.
- Corpus tests across all 66 canonical books and representative peripheral files.
- Configuration merge and validation tests.
- Style cascade/inheritance tests.
- Golden intermediate SILE generation tests.
- Backend integration tests that invoke the pinned SILE runtime on small fixtures.
- PDF smoke tests checking page count, dimensions, embedded fonts where feasible, and text/image presence.
- GUI smoke tests for open-project, edit-setting, build, cancel, diagnostics, and open-PDF flows.
- Cross-platform packaging smoke tests.

### 16.2 MVP Acceptance Scenarios

| Scenario | Pass condition |
|---|---|
| A — Defaults only | Given a folder containing valid GEN.usfm and JHN.usfm and no BibleCompose config files, opening the folder and choosing Build PDF produces a readable PDF using embedded defaults. |
| B — Settings override | Given biblecompose.toml specifying 6×9 page size, one column, custom margins and hidden verse numbers, the generated PDF reflects those settings. |
| C — Style override | Given styles.toml changing body font size and q1 indent, only the intended styles change compared with the default build. |
| D — Footnote | Given valid \f content in USFM, the note is rendered in the PDF and remains associated with its caller. |
| E — Cross-reference | Given valid \x content, the reference appears using the configured supported placement mode. |
| F — Figure | Given a valid figure marker and existing project image, the image renders with configured sizing behavior. |
| G — Invalid USFM | Given an unclosed character style, BibleCompose shows a blocking diagnostic with source file/location and does not invoke SILE. |
| H — Invalid config | Given malformed biblecompose.toml, the project reports a blocking TOML error rather than silently using defaults. |
| I — Backend failure | Given a forced SILE error, the GUI reports failure, retains the previous good PDF, and exposes the backend log. |
| J — Cancel | Given a long-running build, Cancel terminates the active backend and returns the UI to an operable state. |

## 17. MVP Scope

The first shippable milestone should prove the complete architecture rather than attempt every advanced Bible layout feature. The following is the recommended MVP boundary.

### 17.1 MVP Must Include

- Rust desktop GUI and project-folder opening.
- Recursive USFM discovery and book identification.
- Core USFM parser with source locations and diagnostics.
- Built-in settings and styles.
- Optional biblecompose.toml and styles.toml overrides.
- Canonical book ordering and project book selection.
- One-column and two-column body composition.
- Body paragraphs, section headings, chapter/verse numbering, poetry q1–q4.
- Common inline character styles.
- Footnotes.
- Cross-references using one stable placement mode.
- Basic images/figures.
- Headers/footers and page numbers at a basic level.
- SILE backend invocation and error capture.
- PDF output and Open PDF action.
- Build cancellation, logs, and last-good-output protection.
- Automated golden/integration tests.

### 17.2 Explicitly Defer from MVP

- Center-column or verse-aligned gutter cross-reference system.
- Diglot synchronization.
- Interlinear layout.
- Advanced float/wraparound image placement.
- Thumb indexing.
- Automatic cover generation.
- PDF/X and CMYK prepress workflow.
- Interactive page-specific micro-adjustments.
- Complex study-Bible sidebars and multiple synchronized note streams.
- Automatic table of contents, glossary/index generation beyond basic peripheral content.
- Plugin ecosystem or arbitrary project-provided Lua execution.
- Full visual page editor.

### 17.3 Suggested Milestone Sequence

| Milestone | Deliverable |
|---|---|
| M0 — Skeleton | Rust workspace, GUI shell, project open, diagnostics model, build orchestration interfaces. |
| M1 — USFM to simple PDF | Parse basic id/c/v/p/q/s markers → document model → generated SILE → PDF. |
| M2 — Configuration | Embedded defaults, biblecompose.toml, page/typography/output settings, GUI editing. |
| M3 — Styles | styles.toml, paragraph/character styles, chapter/verse appearance, style GUI. |
| M4 — Publishing structures | Footnotes, cross-references, figures, headers/footers. |
| M5 — Hardening | Full-book corpus tests, diagnostics, cancellation, packaging, integrated preview if ready. |
| M6 — v1 release | Documentation, default presets, performance/reliability fixes, platform installers. |

## 18. Post-MVP Roadmap

| Area | Potential capability |
|---|---|
| Advanced reference layout | Center-column references, side/gutter references aligned to verse anchors, external reference databases. |
| Reader Bible | Hidden chapter/verse numbers, chapter-opening treatments, running reference ranges, paragraph optimization. |
| Study Bible | Multiple note streams, sidebars, profiles, questions, charts, structured study content. |
| Diglot | Two translations with chapter/verse synchronization, independent typography and directionality. |
| Image composition | Anchored floats, column spanning, full-page figures, caption/credit management, wraparound shapes. |
| Front/back matter | TOC, introductions, glossary, indexes, map index, copyright/credit pages. |
| Prepress | PDF/X, spot/color workflows, printer profiles, preflight reporting, bleed/crop marks if needed. |
| Profiles/presets | Reader, standard Bible, large print, study Bible, booklet, custom reusable presets. |
| Extensibility | Safe plugin or extension API for custom transformations and layout behaviors. |
| CLI/headless | Optional command-line build mode reusing the same Rust core for CI and automated publishing. |

## 19. Open Decisions

These decisions do not block the SRS architecture but should be resolved before implementation reaches the affected milestone.

| Decision | What must be decided |
|---|---|
| GUI framework | Choose a Rust-capable desktop GUI framework based on cross-platform maturity, text controls, accessibility, packaging, and PDF-preview integration. Candidates can be evaluated separately; the requirements intentionally do not bind BibleCompose to a specific toolkit. |
| USFM parser strategy | Build a dedicated Rust parser vs. adapt an existing compatible parser. The selected approach must provide source spans and preserve unknown content safely. |
| PDF preview implementation | Embedded viewer vs. OS viewer for v1. The backend PDF generation must not depend on the preview choice. |
| Bundled SILE distribution | Confirm licensing, binary size, platform packaging, fonts, and BibleCompose-specific SILE package delivery. |
| Default fonts | Select legally redistributable defaults with broad Unicode support; allow project fonts where licensing permits. |
| Cross-reference MVP placement | Choose the simplest robust v1 location: footnote area is recommended before center-column alignment. |
| Configuration schema versioning | Decide whether explicit schema_version = 1 is required from first release or introduced before the first breaking change. |
| Legacy USFM tolerance | Define exact compatibility guarantees for USFM 2.x fixtures while keeping the semantic model aligned with USFM 3.1. |
| Canonical scope | Decide whether deuterocanonical books are included in built-in canonical ordering and presets; parser architecture should permit them regardless. |

## 20. Technical References

*These references inform the technical baseline but do not replace BibleCompose’s own versioned requirements and tests.*

| Reference | Location | Relevance |
|---|---|---|
| USFM/USX/USJ Documentation — USFM 3.1 introduction | https://docs.usfm.bible/usfm/3.1.1/introduction.html | USFM 3.1 describes a common Scripture content model expressed as USFM, USX, or USJ. |
| USFM/USX/USJ Documentation — peripheral books/divisions | https://docs.usfm.bible/usfm/3.1.2/periph/books-divs.html | Reference for front matter, introductions, back matter, glossary/index and other peripheral-book identifiers. |
| USFM/USX/USJ Documentation — character attributes | https://docs.usfm.bible/usfm/3.1.2/char/attributes.html | Reference for attributes such as figure src/alt/size/loc/copy/ref and other character-level attributes. |
| SILE Typesetter — project home / releases | https://sile-typesetter.org/ | SILE release and documentation entry point. |
| SILE v0.15.0 release notes | https://sile-typesetter.org/blog/release-v0.15.0/ | Documents SILE’s compiled Rust application architecture while retaining Lua-based typesetting internals and extensibility. |

## Appendix A — Product Definition in One Sentence

BibleCompose is a Rust desktop application that turns a folder of USFM Scripture files, plus optional publication settings and style overrides, into a validated, reproducible PDF using SILE as the typesetting backend.

## Appendix B — Default Build Contract

```text
INPUT
  folder containing ≥ 1 valid USFM book
  biblecompose.toml  optional
  styles.toml        optional
  assets/            optional

PROCESS
  discover → parse → validate → resolve defaults/overrides
  → build Scripture model → emit SILE → run SILE

OUTPUT
  one PDF + diagnostics/build log

GUARANTEE
  no source USFM modification during composition
```
