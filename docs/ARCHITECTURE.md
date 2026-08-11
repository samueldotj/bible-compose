# Architecture

The design for BibleCompose: layering, the three document models, configuration and style resolution, font pre-flight, the backend contract, build orchestration, and testing.

This is a design against [SRS v0.1](SRS-v0.1.md). Where it departs from the specification, the reason is recorded in [SRS-REVIEW](SRS-REVIEW.md).

Related: [SRS-REVIEW](SRS-REVIEW.md) · [ROADMAP](ROADMAP.md)

Decisions and their rejected alternatives live in the ADRs:
[001 shared USFM core](adr/001-usfm-core.md) ·
[002 SILE interface](adr/002-sile-interface.md) ·
[003 GUI and preview](adr/003-gui.md) ·
[004 no layout crate](adr/004-no-layout-crate.md) ·
[005 provenance](adr/005-provenance.md) ·
[006 single binary](adr/006-single-binary.md)

---

## 1. Stack

```text
Desktop shell      Tauri 2 + Rust   (dialogs, file I/O, watching, process control)
Frontend           Svelte + Vite + TypeScript  (no SvelteKit, no router, no SSR)
PDF preview        pdf.js in the webview
USFM engine        usfm-core (shared with easy-usfm) → usfm3, pinned
Configuration      toml_edit — one parse, format-preserving, span-carrying
Typesetting        SILE 0.15, pinned, as a child process
Backend input      XML, generated; never TeX-like .sil, never string-templated
```

Two of these are not free choices. The webview is what supplies complex-script text input, accessibility, and localization to the settings UI without building them ([ADR-003](adr/003-gui.md)). XML is what makes "Scripture cannot inject Lua" a property of the format rather than of an escaping function ([ADR-002](adr/002-sile-interface.md)).

## 2. Layering

```text
                    ┌────────────────────────────────────────┐
                    │  Svelte UI + pdf.js preview            │
                    └───────────────────┬────────────────────┘
                                        │  Tauri commands — typed, async
                    ┌───────────────────▼────────────────────┐
                    │  biblecompose-app                       │
                    │  build orchestration, state machine,    │
                    │  cancellation, project session          │
                    └───┬───────────┬───────────┬─────────────┘
                        │           │           │
        ┌───────────────▼──┐  ┌─────▼───────┐  ┌▼──────────────────┐
        │ biblecompose-    │  │ biblecompose│  │ biblecompose-sile │
        │ project          │  │ -config     │  │ Backend impl:     │
        │ discovery, ids,  │  │ settings +  │  │ XML emit, invoke, │
        │ asset paths,     │  │ styles,     │  │ log mapping       │
        │ caches           │  │ cascade,    │  └───────┬───────────┘
        └───────┬──────────┘  │ provenance  │          │ child process
                │             └─────────────┘          ▼
        ┌───────▼──────────┐                     ┌───────────┐
        │ biblecompose-    │                     │   SILE    │
        │ scripture        │                     │  + bc Lua │
        │ USJ → normalized │                     │   class   │
        │ ScriptureDocument│                     └─────┬─────┘
        └───────┬──────────┘                           ▼
                │                                     PDF
        ┌───────▼──────────┐
        │   usfm-core      │  shared crate, no BibleCompose concepts
        │   (→ usfm3)      │
        └──────────────────┘

    biblecompose-diagnostics is used by every box above and depends on none of them.
```

`biblecompose-config` also reaches `biblecompose-scripture`, for one reason: a style is keyed by the marker it applies to, and the markers are the model's vocabulary. Mirroring them into the style layer would be a second list of every supported marker, kept where it cannot be checked against the first.

The arrows that are *not* there are the design. `biblecompose-scripture` does not know what a page is. `biblecompose-config` does not know what SILE is. `biblecompose-sile` is the only crate that has ever heard of SILE, and it is reached only through a trait. Nothing below `biblecompose-app` knows a GUI exists.

## 3. Crates

```text
crates/
├── biblecompose-app/          Tauri commands, build state machine, cancellation,
│                              project session, background execution
├── biblecompose-project/      folder discovery, \id identification, book ordering,
│                              asset resolution, discovery and parse caches
├── biblecompose-scripture/    USJ → ScriptureDocument normalization, canon table,
│                              note and reference structure, figure resolution
├── biblecompose-config/       settings and styles: schema, defaults, cascade,
│                              provenance, toml_edit read/write, validation
├── biblecompose-sile/         Backend trait impl: XML emitter, process invocation,
│                              log mapping, version detection
├── biblecompose-diagnostics/  Diagnostic, Severity, codes, source locations
├── biblecompose-cli/          headless build — the test harness and dev tool
└── biblecompose-testkit/      fixtures, golden helpers, PDF assertions

sile/
├── classes/biblecompose.lua   the versioned rendering contract
└── packages/                  notes, references, figures, running heads

defaults/
├── biblecompose.toml          embedded via include_str!
└── styles.toml
```

Eight crates plus a test kit, against the SRS's ten. `biblecompose-tauri` is the desktop shell and arrives at M2 ([ADR-003](adr/003-gui.md)): a bridge with no domain logic, so the CLI and the GUI cannot disagree about what a build is. `biblecompose-layout` is gone ([ADR-004](adr/004-no-layout-crate.md)); `biblecompose-usfm` and `biblecompose-model` are replaced by the shared `usfm-core` plus a thin normalization crate ([ADR-001](adr/001-usfm-core.md)); `biblecompose-core`'s orchestration lives in `biblecompose-app`, which is the only thing that orchestrates.

Everything except `biblecompose-app` builds without Tauri and runs headless. That is NFR-009, satisfied structurally rather than by discipline, and `biblecompose-cli` is the proof that it holds — it is built from milestone one and never allowed to break.

---

## 4. Source-of-truth model

```text
Project folder  ← authoritative; no proprietary database, no hidden metadata
      ↓
usfm-core   → UsfmDocument (USJ, source-faithful, spans)
      ↓ normalization
ScriptureDocument   ← composition-oriented, canon-ordered
      ↓ + ResolvedSettings + ResolvedStyles   (both carrying provenance)
XML emission
      ↓
SILE + biblecompose class
      ↓
PDF
```

The arrow points one way, and **BibleCompose never writes USFM**. BLD-004 and NFR-007 are therefore properties of the architecture rather than promises: no code path in any crate opens a `.usfm` file for writing. The only files BibleCompose writes are `biblecompose.toml`, `styles.toml`, its own cache directory, and the output PDF.

## 5. The three models

Three representations, each with one job. Collapsing any two of them is the mistake that makes the others hard to change.

**`UsfmDocument`** — what `usfm-core` produces. USJ, source-faithful, spans on structural nodes, unknown markers preserved rather than dropped (USFM-004). It answers *what does the file say*. BibleCompose does not define it and does not extend it.

**`ScriptureDocument`** — the normalized model FUN-001 asks for. It answers *what is the publication*.

```rust
pub struct ScriptureDocument {
    pub books: Vec<Book>,              // canon-ordered, inclusion already applied
    pub provenance: Vec<BookSource>,   // book → file, for every diagnostic
}

pub struct Book {
    pub code: BookCode,                // GEN…REV, plus deuterocanon
    pub names: BookNames,              // \h, \toc1..3, \mt1..4
    pub blocks: Vec<Block>,
}

pub enum Block {
    Paragraph { style: ParaStyle, content: Vec<Inline> },
    Poetry    { style: PoetryStyle, level: u8, content: Vec<Inline> },
    Heading   { style: HeadingStyle, level: u8, content: Vec<Inline> },
    ListItem  { level: u8, content: Vec<Inline> },
    Table     { rows: Vec<Row> },
    Figure    (FigureRef),
    Break,                             // \b
}

pub enum Inline {
    Text(String),
    Chapter { number: u16, published: Option<String>, alternate: Option<u16> },
    Verse   { id: VerseId, published: Option<String>, alternate: Option<VerseId> },
    Char    { style: CharStyle, content: Vec<Inline> },
    Note    (Note),                    // \f, \fe — structured, not flattened
    Ref     (CrossReference),          // \x — a distinct type, per SCR-004
    Milestone(Milestone),
    Unsupported { marker: String, at: SourceLoc },   // diagnosed, never silent
}
```

Three properties of this model are load-bearing:

**Chapter and verse are inline anchors, not containers.** SCR-001 requires that hiding the numbers must not lose the reference. If verses were containers, hiding them would be a rendering flag on a structural node and every consumer would have to remember it. As anchors, the number's visibility is a style question and the anchor is always there — for running heads, for PDF destinations (SCR-008), and for diagnostics that name a reference. It also matches the reality that a paragraph in USFM legitimately spans verses and a verse legitimately spans paragraphs.

**Cross-references are their own type** (SCR-004), not notes with a flag. They have different placement rules, different styling, and a different roadmap.

**`Unsupported` is a variant, not an omission.** FUN-003 requires unknown markers to be diagnosed rather than discarded. Carrying them through with their source location means a marker can be supported later without touching the parser boundary, and means the emitter is where the decision to drop something is made and logged.

**The canon table is data, not code** — book code, canonical position, standard abbreviations, testament, deuterocanonical flag. Shipped as a table so that including the deuterocanon costs rows rather than a schema change, and so a project can override ordering (BOOK-002) without special cases.

## 6. Configuration and style resolution

Both files go through one mechanism, because they have the same problem: merge defaults with overrides, keep track of where every value came from, write back without destroying the file.

```text
embedded defaults (include_str!)
        ↓  field-by-field merge, per CFG-002
project biblecompose.toml / styles.toml   ← parsed once, by toml_edit
        ↓  style inheritance flattened, cycles detected
        ↓  units parsed and validated, values range-checked
ResolvedSettings / ResolvedStyles         ← every value carries its origin
```

**One parse, by `toml_edit`.** The format-preserving document is the only parse; the typed view is derived from it. This gives four things at once: CFG-006 upgraded from "where practical" to actual comment and ordering preservation on write; CFG-003's line and column on a syntax error; CFG-004's unknown-key detection as a walk of the document against *what resolution asked for* — not a separately written list of legal keys, which is a second thing to update when a setting is added — with a span per stray key; and a single place where a GUI edit turns into a file mutation.

**Every resolved value carries its origin** ([ADR-005](adr/005-provenance.md)):

```rust
pub struct Sourced<T> { pub value: T, pub origin: Origin }

pub enum Origin {
    Builtin,
    File { path: Utf8PathBuf, line: u32, col: u32 },
    Inherited { from: StyleSelector },
}
```

STY-008's inspector is then a read of the resolved map. CFG-007's "reset to default" is deleting the key whose origin is a file. STY-004's diagnostic has somewhere to point.

**Resolved settings reach the backend as class options, on the command line.** SILE does not read class options from an XML root — measured, not assumed — so they are passed with `-O key=value`, in a fixed order, as plain strings. The translation lives in `biblecompose-app`, the only crate that has heard of both the configuration layer and the backend, which is also where [ADR-005](adr/005-provenance.md)'s rule that provenance cannot reach the emitter becomes a signature rather than a convention. Hiding a verse number is a class option and not an emission change: the document says *what* and the class says *how* ([ADR-002](adr/002-sile-interface.md)), so turning numbers back on needs no re-emission.

**Style selectors are typed, not strings** (STY-003):

```rust
pub enum StyleSelector {
    Paragraph(ParaStyle), Poetry(PoetryStyle), Heading(HeadingStyle),
    Character(CharStyle), Chapter, Verse, Note(NoteKind),
    Reference, Figure, RunningHead, Folio, ListItem(u8), TableCell,
}
```

`[paragraph.q1]` and a hypothetical `[character.q1]` cannot collide, which is the requirement. Inheritance is a single optional `inherits` key resolved into a flat map at load time; the cycle check happens there, once, and the diagnostic names the cycle.

**Units are parsed, not passed through.** `"0.55in"`, `"11.5pt"`, `"6x9in"` become typed lengths at the configuration boundary. A string that reaches the emitter is a bug, because it means an invalid unit will be diagnosed by SILE, in SILE's words, at the wrong layer.

**The built-in defaults are a TOML file, not a table of Rust constants.** `defaults.toml` is compiled in with `include_str!` and read by the code that reads a project file, so a default that would be rejected from a project file fails the test suite rather than shipping; it is also what CFG-007's "reset to default" shows, and a readable list of every key that exists.

`schema_version = 1` is asked for from the first release, and the two ways of getting it wrong are answered differently. An **unknown** version is one clear error and the project file is closed — not read at all — because reading a file written for a schema we do not know produces a cascade of complaints about keys that are correct in their own version. A **missing** version is a warning whose help names the line to add: there is exactly one version, so assuming it is safe, and refusing every file written before the key existed would punish publishers for a problem versioning exists to prevent later.

## 7. Pre-flight

This layer exists because the backend will not complain, and the S0 spike established that as a category rather than a quirk. Five separate times SILE did the wrong thing without saying so, and **four of those were clean builds with a zero exit code**:

| What | What SILE does | Evidence |
|---|---|---|
| A font that cannot render the text | substitutes; emits a page of `.notdef` boxes, exit 0 | F-12 |
| A language it has no hyphenation patterns for | applies another language's patterns | F-11 |
| Frame geometry with negative height | warns once per page and carries on | F-7 |
| An asset path outside the project | embeds it | F-14 |
| Command syntax arriving inside Scripture | executes it *(SIL input only; not reachable via [ADR-002](adr/002-sile-interface.md))* | F-13 |

The lesson is not that SILE is careless. **A typesetter's job is to set what it is given; refusing bad input is the application's job**, and none of it can be delegated. Everything below runs in Rust, before the backend is invoked, and each check exists because the spike watched the alternative succeed.

### 7.1 Fonts and scripts

PDF-003 and PDF-004 are BibleCompose's obligations, checked before a single page is set.

```text
resolved styles → set of (font family, weight, style) requested
        ↓  resolve: project assets/fonts/ first, then system
        ↓  unresolved → Error, naming the setting that asked for it
resolved font files
        ↓  for each: read cmap
        ↓  against the codepoint set the text actually uses, per style
coverage report → Error with an example reference for each gap
```

The same machinery runs the other way for the settings form. Rather than a spelling of a family name, or the operating system's font dialog — which offers every face installed here, knows nothing about the ones the project or the backend ship, and has no opinion about whether any of them can draw the book — the window asks this crate for the list a build would resolve against, in resolution order, each already checked against the codepoint set above. A publisher setting Tamil sees which four of three hundred families can set it, and which ones travel with the book. A picker that let them choose one of the other two hundred and ninety-six would only be moving the coverage error later.

Two details make it work in practice. Project-local fonts must be usable without installing them into the operating system (FONT-003), so the emitter refers to fonts by file path for anything under `assets/fonts/`, and only by family name for system fonts — S0.5 confirmed SILE loads a face by path that fontconfig has never heard of, and subsets and embeds it correctly. And the codepoint set is computed from the normalized model, per style, so a font used only for footnotes is checked only against footnote text — otherwise a project with a Latin-only note font and Tamil body text reports a false failure.

### 7.2 Hyphenation

A Tamil Bible set through the backend carries hyphens in the middle of Tamil words throughout, and nothing reports it (FONT-004). [Spike F-11](../spike/NOTES.md) recorded the symptom and inferred the cause: that asking for a language the backend cannot hyphenate gets you *another language's* patterns.

**Measured against the pinned backend, that inference is wrong and the symptom is real.** SILE 0.15.13 ships `languages/ta/hyphens-tex.lua`; Tamil patterns exist, they are auto-generated from TeX, and they fire. On one book of Lamentations: `ta` produced 510 hyphens, `am` and a nonexistent tag produced 7 — the number in the source text — and `en` also produced 7, because English patterns do not match Tamil letters. A language with no patterns gets *no* hyphenation, not somebody else's.

So the defect is narrower and sharper than a missing-pattern table would address, and a table of "languages the backend has patterns for" would have passed `ta` straight through, which is the bug. **What decides it is the script**: hyphenation is a convention of the Latin, Greek and Cyrillic traditions and a few others, and in Tamil, Devanagari, Thai, Hebrew, Arabic and the rest a mid-word hyphen is an error however good the patterns are.

The script is read from the text rather than from the language tag, because the text is what gets set and a tag can be absent, wrong, or describe a book that is mostly in another script. Where the script does not hyphenate, the backend is told not to, and a project that asked for hyphenation is told why it is off — nothing is wrong with the project, but a setting that did nothing has to be mentioned. The language tag itself is passed through unchanged: it drives more than hyphenation, and rewriting it to encode a hyphenation decision would hide that decision in a value something else reads.

### 7.3 Geometry and assets

Resolved frame geometry is validated before emission — a frame whose computed height is zero or negative is a blocking diagnostic naming the margin settings that produced it, because user-supplied margins make that a reachable state and the backend only warns.

Asset paths are checked for containment after canonicalization, so `..` and symlinks are both covered. This is the only such check anywhere in the pipeline: SILE validates an image's *format* and never its *provenance*, and will embed a file from anywhere on disk without comment.

## 8. The backend contract

```rust
pub trait Backend {
    fn version(&self) -> Result<BackendVersion>;       // SILE-002
    fn run(&self, job: &BackendJob, cancel: &CancelToken,
           log: &mut dyn FnMut(LogLine)) -> Result<BackendOutcome>;
}
```

One trait, one implementation, no second model behind it ([ADR-004](adr/004-no-layout-crate.md)). SILE-001 is satisfied because this trait is the only route to the backend and nothing above `biblecompose-app` can reach it.

**The generated input is XML** ([ADR-002](adr/002-sile-interface.md)). SILE treats XML as a first-class input and elements as command invocations, which means the BibleCompose Lua class defines the vocabulary and Scripture never becomes syntax:

```xml
<biblecompose version="1" class="biblecompose">
  <styles>…resolved style map, emitted as data…</styles>
  <book code="MAT" name="Matthew">
    <heading style="s1">The Preaching of John</heading>
    <para style="p">
      <chapter n="3"/><verse n="1"/>
      <text>In those days came </text>
      <char style="nd"><text>John</text></char>
      <text> the Baptist</text>
      <note style="f" caller="+"><para style="ft"><text>Or …</text></para></note>
      <text>, preaching.</text>
    </para>
  </book>
</biblecompose>
```

A backslash in Scripture is a backslash. A brace is a brace. The serializer escapes `<`, `>`, and `&`; there is no other escaping to get right, and no way for a verse, a book name, or a font name typed into a settings field to become a command.

**Determinism has two different standards** ([SRS-REVIEW F4](SRS-REVIEW.md#f4--reproducible-is-two-different-claims-and-must-be-split)). The XML is byte-reproducible and asserted as such by golden files. The PDF is asserted structurally — page count, geometry, embedded fonts, per-page extracted text, image presence.

The reason the PDF cannot be byte-compared is worth recording exactly, because the obvious explanation is the wrong one. SILE already zeroes the document `/ID` and writes no creation date. What varies is the **font subset tag**, which is randomly generated per run: four builds of identical input gave four different hashes and two different file sizes, differing only in tags like `AYABNL+DejaVuSerif` versus `HQTCEM+DejaVuSerif`. So the structural comparison strips the six-letter prefix before comparing font names, and nobody wastes an afternoon looking for a `SOURCE_DATE_EPOCH` that does not exist.

One rule enforces the first: **no `HashMap` on the emission path**. Rust randomizes its iteration order per process, so a single `HashMap` in configuration resolution, style resolution, or emission makes the golden tests fail intermittently and unreproducibly. `BTreeMap` or `IndexMap`, lint-enforced.

**The Lua class is versioned with the application** (SILE-009). The `version` attribute on the root element is the contract; the class refuses a version it does not know, which turns a mismatched install into one clear message rather than a page of Lua stack traces.

**Invocation is a child process, and stays one for v1.** SILE 0.15 is a Rust binary with an embedded Lua VM, and it does publish a crate, so in-process embedding is imaginable later. It is the wrong trade now: a child process gives a hard failure boundary for a Lua error (NFR-007), and it gives cancellation, which an embedded VM does not (BLD-006). Arguments are passed as an array; nothing is ever concatenated into a shell string (§15).

## 9. Build orchestration

```text
Idle → Loading → Loaded ──blocking errors?──→ Blocked
                   │ no
                   ▼
              Validating → Emitting → Typesetting → Publishing → Succeeded
                   └──────────┴───────────┴────────────┴────────→ Failed
                                                       └────────→ Cancelled
```

These are exactly GUI-006's states. They live in `biblecompose-app`, are the single source of truth for what the UI shows and what the Build button does, and every transition is reported to the UI as an event rather than polled.

**Nothing writes to the output path.** Every build runs in `.biblecompose/build/<id>/` and the finished PDF is moved into place atomically, only after it exists and is non-empty. BLD-009 is then structural: a failure cannot replace the last good PDF because it never had access to it. The one case that still fails late is a destination locked by an open viewer, which gets its own diagnostic naming the file (BLD-011).

**Cancellation kills the process tree.** A Job Object with kill-on-close on Windows; a process group and a group signal on Unix. `Child::kill` alone orphans descendants on Windows, and an orphan holding a file handle turns the next build's atomic publish into an unexplainable failure.

**Two caches, both keyed on content, both invalidated by five things.** A discovery cache mapping path to book code, so PRJ-003's requirement to read `\id` rather than trust the filename does not mean re-reading 66 files on every open; and a parse cache, so editing one book does not reparse the rest. Both keyed on path, size, and modification time; both invalidated by a change in configuration hash, style hash, marker-table version, backend version, or application version. Missing any one of the five produces a stale-cache bug that will be diagnosed as an emitter bug.

**Draft builds** ([SRS-REVIEW F10](SRS-REVIEW.md#f10--build-time-is-the-dominant-fact-of-the-workflow-and-the-srs-does-not-confront-it)). A full Bible takes minutes; the workflow is change-a-setting-and-rebuild. A draft build restricts the run to the selected books and marks the output as a draft. The mechanism already exists in BOOK-003; what this adds is making it the default action while iterating.

## 10. Diagnostics

One type, produced by six stages, rendered by one panel.

```rust
pub struct Diagnostic {
    pub severity: Severity,          // Error | Warning | Info
    pub code: &'static str,          // stable; DIA-001
    pub stage: Stage,                // Discovery | Usfm | Config | Style
                                     // | Asset | Font | Backend | Output
    pub message: String,
    pub location: Option<SourceLoc>, // file, line, column
    pub reference: Option<ScriptureRef>,
    pub help: Option<String>,
    pub detail: Option<String>,      // raw backend text, collapsed by default
}
```

Codes carry their stage as a prefix — `USFM-`, `CFG-`, `STY-`, `FONT-`, `SILE-`, `OUT-` — so DIA-004's filtering is a field comparison and a test asserting a code cannot silently match a different stage's.

Two rules the SRS implies but does not state. **Validation runs to completion before the backend is invoked** (DIA-002): a blocked build reports every blocking issue at once, not the first. And **diagnostics quote at most one elided source line, never the full context**, and the build log carries no Scripture by default — NFR-010, which is easy to satisfy at the start and impossible to retrofit once the log format is relied upon.

Diagnostics from the codes `usfm-core` already defines are passed through unchanged rather than re-mapped, so an error means the same thing in both products.

## 11. GUI

The three-pane layout of SRS §11.1, unchanged. What the design adds is where the boundary sits.

Svelte components never call Tauri APIs directly; they call typed service interfaces, exactly as `easy-usfm` does. Long work — discovery, parsing, emission, SILE — runs on a background task and reports progress as events, so GUI-012 and NFR-003 hold because the UI thread has nothing to block on rather than because the work is fast.

The preview is pdf.js in the webview: one implementation on all three platforms, page-windowed so a 2,000-page Bible does not rasterize on open ([ADR-003](adr/003-gui.md)). GUI-009's "Open PDF" and "Open Output Folder" remain, as conveniences rather than as a platform fallback.

Unsaved settings edits are held in the session with a dirty flag per key and the origin of the pending value, which is what makes GUI-010's protection specific — the close prompt can say which settings would be lost.

## 12. Performance budgets

| Metric | Target | On |
|---|---|---|
| Project open → books listed | < 500 ms | 66 books, warm cache |
| Project open → parse and validation complete | < 5 s (NFR-002) | 66 books, cold |
| Reparse after one book changes | < 200 ms | 66-book project |
| Draft build, one book | < 10 s | typical Gospel, two columns |
| UI frame time during a build | unaffected | any project size |
| Cancel → process gone, UI operable | < 1 s | any build state |

Parsing is parallel across files; there is no incremental parsing here and none is needed. The full-Bible build time is deliberately absent: it is SILE's, it will be minutes, and the design's answer is the draft build rather than a number BibleCompose cannot control.

## 13. Testing

Six layers, running headless through `biblecompose-cli`.

**Inherited from `usfm-core`** — parser unit tests, the ~200-file vendored corpus and the fetched long tail, the three-way differential oracle, and the fuzz target. SRS §16.1's first two layers arrive with the dependency ([ADR-001](adr/001-usfm-core.md)).

**Normalization** — USJ to `ScriptureDocument` over the whole corpus, asserting that no Scripture text is lost and no content is reordered (FUN-002, USFM-005). The assertion is a concatenation comparison, which is crude and catches the only failure that matters.

**Configuration and styles** — merge, unit parsing, unknown keys, inheritance flattening, cycle detection, and round-trip through `toml_edit` proving comments and key order survive a GUI write.

**Golden XML** — the emitted backend input, byte-compared, on every platform. This is where determinism is caught, and it catches it the day the first element is emitted rather than at M5.

**PDF structure** — page count, page geometry, embedded font list, per-page extracted text, image presence, against vendored version-pinned fonts. Never byte comparison.

**Scenarios** — SRS §16.2's ten acceptance scenarios A through J, scripted end-to-end through the CLI, plus GUI smoke tests for the flows the CLI cannot reach.

## 14. Security

SRS §15 as written, with the mechanism named for each clause.

Scripture and configuration cannot inject Lua because they are XML text nodes, not command syntax (§8). SILE is invoked with an argument array through the process API; nothing is concatenated into a shell string. Asset references resolve inside the project directory, and a path escaping it is a diagnostic rather than a silent read — checked after canonicalization, so `..` and symlinks are both covered. Temporary directories come from the OS API and are removed unless `keep_intermediates` is set. Nothing leaves the machine: no telemetry, no network calls on any path, and the offline end-to-end test (NFR-004) is what keeps that true.

One addition. The preview renders a PDF that BibleCompose itself produced, but pdf.js runs in the webview, so its JavaScript execution and external-link handling are disabled in the viewer configuration — the cost of being wrong about "we generated it" is too high for the saving.
