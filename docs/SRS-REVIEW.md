# SRS Review — BibleCompose v0.1

An analysis of [BibleCompose_Software_Requirements_Specification_v0.1.docx](../BibleCompose_Software_Requirements_Specification_v0.1.docx) before designing against it: what the specification gets right, where it contradicts itself, what it leaves out, and which of its open decisions can be closed now.

The design that follows from this review is in [ARCHITECTURE](ARCHITECTURE.md); the sequencing consequences are in [ROADMAP](ROADMAP.md); the five load-bearing choices are in the [ADRs](adr/).

---

## 1. Verdict

The specification is sound and unusually complete for a v0.1. Requirement IDs with per-row acceptance criteria, explicit non-goals, an honest open-decisions list, and a stated architectural boundary between Bible semantics and the typesetting engine — the shape is right and this review does not propose changing it.

Fourteen findings follow. Two of them change the plan materially:

- **F1** — the component the SRS treats as its largest build (`biblecompose-usfm` + `biblecompose-model`) already exists as a designed component in the sibling `easy-usfm` project, and should be shared rather than rewritten.
- **F2** — the riskiest unknown in the product (whether SILE produces acceptable two-column Scripture with notes) is not touched until M4 of a seven-milestone plan, and must move to the front.

The rest are corrections, gap-fills, and decisions taken.

---

## 2. Findings

### F1 — The largest specified component already exists next door

SRS §12.1 proposes two crates:

```text
biblecompose-usfm/    # parser, AST, validation, source spans
biblecompose-model/   # normalized Scripture document model
```

`easy-usfm-core` is that component, already designed to a level of detail this SRS does not reach: a facade over the `usfm3` crate, a USJ document tree carrying source spans, a marker table with version metadata driving diagnostic severity, stable diagnostic codes, a verse index handling ranges and `\va`/`\vp`, a vendored ~200-file corpus plus a fetched long tail, a three-way differential oracle, and a fuzz target gating releases. `easy-usfm`'s own architecture note already calls it *"a candidate for its own repository."*

BibleCompose is the second consumer that makes the extraction pay for itself. This closes SRS Open Decision *"USFM parser strategy"* and removes the single largest work item and the single largest technical risk from the plan.

It is not free, and three qualifications matter:

- **It is not built yet.** `easy-usfm` is design-complete with no implementation; its M0 is the parser layer. BibleCompose's first real milestone therefore depends on another project's first milestone. That is a scheduling dependency to manage deliberately, not a detail — see [ROADMAP §3](ROADMAP.md#3-the-dependency-on-easy-usfm).
- **It is tuned for editing, not batch composition.** Incremental chapter-chunked reparse and UTF-16 offsets exist for a keystroke loop that BibleCompose does not have. The chunking is simply unused here; the offset space is not — BibleCompose wants byte and line/column offsets natively and must not pay a UTF-16 conversion it has no use for.
- **USJ is source-faithful, and SRS FUN-001 wants normalized.** These are different models and both are wanted. The seam between them is exactly where BibleCompose's own work starts.

→ [ADR-001](adr/001-usfm-core.md)

### F2 — The riskiest unknown is scheduled last

SRS §17.3 sequences M1 as *"USFM to simple PDF"* and defers footnotes, cross-references, figures, and headers to M4. But the question on which the entire product depends is not whether SILE can set a paragraph. It is whether SILE can set **two balanced columns of Scripture with footnotes and cross-references at the foot, running heads carrying a verse range, and column breaking that does not strand a verse number at the bottom of a column** — in Latin and in a complex script — at a quality a publisher will accept.

If the answer needs custom Lua frame work, that is fine and knowable in days. If the answer is that some part of it is impractical, three milestones of parser, config, and style work will have been built on a false premise.

**Recommendation: a Spike 0 before M0.** Hand-write SILE input for one realistic two-column page and produce a PDF. No Rust, no parser, no GUI. Prior art to mine: [Freely-Given-org/BibleTypesetter](https://github.com/Freely-Given-org/BibleTypesetter). Everything downstream of the emitter is then designed against a demonstrated capability rather than an assumption.

The spike also settles several smaller unknowns at no extra cost: whether SILE places PDF artwork as a figure (SRS §4.2 lists `creation-map.pdf` as an asset, and vector-versus-raster is a print-quality decision, not a detail), how running-head marks behave across a column break, and how the XML input path of [ADR-002](adr/002-sile-interface.md) maps elements to commands.

### F3 — §15's injection rule is won or lost by choosing the input format

SRS §15 requires that generated SILE input *"treat Scripture and configuration values as data"* and that user content *"not be allowed to inject arbitrary Lua/SILE execution."*

If BibleCompose emits TeX-like `.sil` by string templating, then every `\`, `{`, `}`, and `%` in Scripture text, in a book name, in a footnote, or in a font name typed into a settings field is a potential command, and the guarantee rests forever on an escaping function being perfect. That is a security property maintained by vigilance, which is the kind that eventually fails.

SILE accepts XML as a first-class input, detected by a leading angle bracket. Emitting XML makes Scripture a text node: escaping is total, standard, and performed by the serializer rather than by us. The guarantee becomes structural.

→ [ADR-002](adr/002-sile-interface.md)

### F4 — "Reproducible" is two different claims and must be split

SILE-005 and NFR-006 are stated as one idea. They are not.

- **The generated backend input is byte-reproducible.** Same normalized model plus same resolved configuration must give the identical file, every time, on every machine. This is achievable and is what golden tests should assert.
- **The PDF is not byte-reproducible.** PDFs carry a creation timestamp and a document ID, and line breaking depends on the exact font binary and on the HarfBuzz and ICU versions underneath SILE. The SRS's own wording — *"materially equivalent"* — is the right standard, but the tests must match it: assert page count, page geometry, the embedded font list, extracted text per page, and image presence. Not bytes.

Two concrete consequences the SRS does not state:

- **Test fonts must be vendored and pinned.** A system font update otherwise breaks golden PDF assertions with no code change, and the failure looks like a regression.
- **No `HashMap` iteration in configuration resolution, style resolution, or emission.** Rust randomizes `HashMap` order per process, so a `HashMap` anywhere on the emission path makes SILE-005 fail intermittently and unreproducibly — the worst available failure mode. `BTreeMap` or `IndexMap` throughout, enforced by a lint.

### F5 — SILE substitutes missing fonts silently, so PDF-003 and PDF-004 cannot be delegated

PDF-003 requires embedded fonts; PDF-004 requires no missing-glyph boxes. Both are written as though the backend will report a problem. It will not: SILE's own issue tracker records that the console is silent when a requested font is not found, and that *only knowing what the document should look like* reveals the substitution ([sile#95](https://github.com/sile-typesetter/sile/issues/95)).

For a Bible in Tamil, that failure mode is a print run of tofu.

BibleCompose must pre-flight, in Rust, before SILE is invoked:

1. Resolve every configured font to an actual file, and fail with a diagnostic if any does not resolve. Never let a fallback happen unnoticed.
2. Check the resolved font's character map against the codepoint set the text actually uses, per style and per script, and report the gap with an example reference.

This is cheap to do and is the difference between catching the problem in the application and catching it at the printer. It is a **missing requirement**; see §4.

### F6 — The layout crate is speculative abstraction

Ten crates for an MVP is a lot, and `biblecompose-layout` — *"backend-neutral layout intentions where practical"* — is the one that does not earn its place. There is no second backend, none is planned, and the SRS's own hedge (*"where practical"*) concedes the model would be partial. Its cost is real: every element modelled twice, and every new feature threaded through an extra translation.

Keep the *boundary*, drop the *model*. A `Backend` trait satisfies SILE-001 (*"no UI module shells out to SILE directly"*) exactly as well, and the emitter consumes the Scripture model plus resolved configuration and styles directly.

→ [ADR-004](adr/004-no-layout-crate.md)

### F7 — STY-008 is an architectural requirement wearing a SHOULD

*"An inspector showing the resolved style and source of each property (default vs project override)"* is listed as a SHOULD, alongside genuinely optional items. It is not optional in the same sense: provenance cannot be added later without touching every resolved type and every merge site in the configuration and style layers.

Decide it at the start. Resolution yields values that carry where they came from. The same mechanism then supplies CFG-007 (reset to inherited) and CFG-004 (unknown-key diagnostics with a line number) almost for free, and it is the only way STY-004's *"misspelled style property is reported"* can point at a location.

→ [ADR-005](adr/005-provenance.md)

### F8 — CFG-006 can be upgraded from SHOULD to MUST for free

The SRS hedges: preserve TOML comments and formatting *"where reasonably practical; if full preservation is not feasible, it must save deterministic valid TOML."*

Full preservation is feasible. `toml_edit` parses to a format-preserving document; mutate only the keys the user touched and comments, ordering, and whitespace survive untouched. The same document supplies CFG-003's line and column for syntax errors, and makes CFG-004's unknown-key detection a walk of the document against the schema with a span for each stray key.

One decision follows the choice: the typed view is derived from the `toml_edit` document rather than deserialized separately, so there is one parse and one source of spans.

### F9 — The CLI is in the post-MVP roadmap but is required by the MVP

SRS §18 lists *"CLI/headless — optional command-line build mode"* as post-MVP. But NFR-009 requires the parser, configuration resolver, style resolver, and emitter to be testable without launching the GUI, and §16.1 wants golden intermediate-generation tests and PDF smoke tests. The mechanism for all of that is a headless build.

The CLI therefore exists from the first milestone, as the test harness and the primary development tool. What is post-MVP is *shipping and documenting it as a product surface*. This is a labelling correction, but it changes what gets built first.

### F10 — Build time is the dominant fact of the workflow, and the SRS does not confront it

NFR-002 budgets five seconds to parse a 66-book project. That is comfortable: parsing is embarrassingly parallel across files.

Typesetting a whole Bible is minutes. And the workflow in §5.2 is *"user iterates on settings/styles and rebuilds."* A minutes-long loop for a one-line style change is the difference between a tool people use and a tool people abandon.

**Recommendation: a first-class draft build** — the currently selected book only, watermarked as a draft. BOOK-003 already provides the mechanism (per-book inclusion), so this is an affordance and a roadmap item rather than new architecture. It is also what makes the GUI-008 preview worth having.

Paired with it: a parse cache keyed on path, size, and modification time, so reopening a project and rebuilding after a one-file edit does not reparse 65 unchanged books.

### F11 — Cancellation needs a job object on Windows

BLD-006 requires terminating the SILE child process *safely*. `std::process::Child::kill` on Windows terminates the named process only; any descendant is orphaned and keeps its file handles, which then blocks the atomic publish in F12 with an error that looks unrelated.

Assign the child to a Job Object with kill-on-close; on Unix, spawn it into its own process group and signal the group. Both are cheap now and unpleasant to retrofit under a deadline.

### F12 — BLD-009 implies the output path is never written directly

*"A failed build shall not replace the last known good PDF."* The way to guarantee that is not to check afterwards but to never let SILE write to the destination: build into a temporary directory, then atomically rename into place only after the PDF exists and is non-empty.

One case needs an explicit diagnostic. On Windows, renaming over a PDF that the user has open in a viewer that holds a lock will fail after a successful typeset. §14.2 lists *"file locked"* among output errors; the message must name the file and say to close the viewer, because the user's mental model at that moment is that the build failed.

### F13 — PRJ-003 makes discovery an I/O problem

Identifying books from the `\id` marker rather than the filename means opening every candidate file, which is correct and costs a directory of file opens on every project open. Read the first 4 KB, find `\id`, and cache the result against path, size, and modification time. Combines with F10's parse cache.

### F14 — Smaller gaps and conflicts

| | Issue | Resolution |
|---|---|---|
| a | §4.2 lists `creation-map.pdf` as a figure asset; no requirement says whether PDF artwork is supported | Confirm in Spike 0. Vector versus raster artwork is a print-quality decision |
| b | GUI-008 has no behaviour for a 2,000-page preview | Preview renders on demand, page-windowed; never rasterizes the whole document |
| c | §14.2 lists cyclic style inheritance as an error class, but STY-007 does not define the inheritance mechanism | Single-parent `inherits` key, flattened at resolution; cycle detected there |
| d | No requirement covers what the build cache invalidates on | Config hash, style hash, marker-table version, backend version, application version — all five, or a stale-cache bug will be blamed on the emitter |
| e | NFR-010 asks logs not to record Scripture unnecessarily, but diagnostics quote source | Quote at most one line, elided, and never in the build log by default |

---

## 3. Open decisions, closed

SRS §19 lists nine. Seven can be closed now; two genuinely wait.

| Decision | Resolution | Where |
|---|---|---|
| GUI framework | **Tauri 2 + Svelte.** No local HTTP server, so §11.1's constraint holds. Matches `easy-usfm`, so the shell, dialogs, watching, and atomic-save work is shared. The decisive argument is not familiarity: the settings UI must accept complex-script text — a project name or font name typed in Tamil — and must be keyboard-navigable and localizable (NFR-011, NFR-012). A webview gets input methods, accessibility, and i18n from the platform; an immediate-mode Rust toolkit owes all three | [ADR-003](adr/003-gui.md) |
| USFM parser strategy | **Share `usfm-core`, extracted from `easy-usfm`, over `usfm3`** | [ADR-001](adr/001-usfm-core.md) |
| PDF preview | **pdf.js in the webview.** One implementation on all three platforms, no native viewer dependency. GUI-009's fallback becomes a convenience action rather than a platform contingency | [ADR-003](adr/003-gui.md) |
| Cross-reference MVP placement | **Footnote area**, as the SRS itself recommends | — |
| Configuration schema versioning | **Require `schema_version = 1` from the first release.** One line now against a migration system later; the asymmetry is not close | [ARCHITECTURE §6](ARCHITECTURE.md#6-configuration-and-style-resolution) |
| Canonical scope | **Include deuterocanonical books in the ordering table from day one.** The table is data; the cost is rows. Excluding them makes the canon a schema property, which is the expensive kind of mistake | [ARCHITECTURE §5](ARCHITECTURE.md#5-the-three-models) |
| Bundled SILE distribution | **Ship a pinned SILE binary plus the BibleCompose Lua class per platform**, version recorded in every build log (SILE-002), advanced override retained (SILE-004). SILE 0.15 is a Rust binary with an embedded Lua VM and can embed its own Lua resources, which makes a per-platform bundle tractable; HarfBuzz, fontconfig, ICU, and libtexpdf remain native dependencies to package | [ADR-002](adr/002-sile-interface.md) |
| Default fonts | **Open.** Needs a licensing review, not an architecture decision. Constrains Spike 0 only in that the spike should use a candidate | — |
| Legacy USFM 2.x tolerance | **Open, and inherited.** This is `usfm-core`'s question to answer, driven by its marker table and corpus | — |

---

## 4. Requirements the SRS is missing

Proposed for v0.2 of the specification, in its own format.

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| FONT-001 | Every font named by resolved configuration or styles shall be resolved to a specific font file before the backend is invoked; an unresolved font shall be a blocking diagnostic naming the font and the setting that requested it. | MUST | A project naming a font absent from the system and from `assets/fonts/` fails to build, with a diagnostic; no PDF is produced with a substituted font. |
| FONT-002 | Before invoking the backend, the application shall verify that each resolved font covers the codepoints used by the text it will set, and report gaps with severity Error and at least one example Scripture reference. | MUST | A Latin-only font configured for Tamil Scripture produces a coverage diagnostic rather than a PDF of missing-glyph boxes. |
| FONT-003 | Fonts placed under the project's asset font directory shall be usable without installing them into the operating system. | MUST | A project-local `.ttf` renders on a machine where that font is not installed. |
| BLD-010 | A build shall never write to the resolved output path; output shall be produced in a temporary location and moved into place atomically after success. | MUST | Killing the process mid-build leaves the previous PDF byte-identical, and leaves no partial file at the output path. |
| BLD-011 | If the output path cannot be replaced because it is locked by another process, the application shall report which file is locked and what to do. | MUST | Building while the previous PDF is open in a locking viewer produces an actionable diagnostic, not a generic I/O error. |
| BLD-012 | The application shall support a draft build restricted to a chosen subset of books, visibly marked as a draft. | SHOULD | Changing one style value and rebuilding a single book completes in a small fraction of a full-Bible build. |
| DET-001 | Generated backend input shall be byte-identical across runs, machines, and operating systems for identical normalized input and resolved configuration. | MUST | Golden-file tests over the corpus pass on all supported platforms in CI. |
| DET-002 | PDF equivalence shall be asserted structurally — page count, page geometry, embedded fonts, per-page extracted text, image presence — not by byte comparison; fonts used in these tests shall be vendored and version-pinned. | MUST | Golden PDF tests do not fail when a system font is updated. |
| CFG-008 | Configuration and style files shall carry an explicit schema version from the first release. | MUST | A file with an unknown schema version produces a clear, actionable diagnostic rather than a field-level parse failure. |

---

## 5. Risk register

| | Risk | Severity | Control |
|---|---|---|---|
| R1 | SILE cannot produce acceptable two-column Scripture with notes without substantial custom Lua | High | Spike 0, before any Rust is written (F2) |
| R2 | `easy-usfm-core` is not ready when BibleCompose needs it | High | Shared crate extracted early; BibleCompose's needs shape its M0; a thin subset unblocks M1 ([ROADMAP §3](ROADMAP.md#3-the-dependency-on-easy-usfm)) |
| R3 | `usfm3` is young, single-maintainer, pre-1.0 | Medium | Inherited unchanged from `easy-usfm` ADR-001: facade, exact pin, cheap fork, upstream engagement |
| R4 | Silent font substitution ships unreadable output | High | FONT-001–003 pre-flight in Rust; never trust the backend to complain (F5) |
| R5 | Full-Bible build times make the iterate-and-rebuild workflow unusable | Medium | Draft builds and a parse cache, from the milestone where builds first get long (F10) |
| R6 | Non-determinism creeps in through map ordering and is diagnosed as an emitter bug | Medium | Ordered maps only, lint-enforced; determinism asserted in CI from the first emitted file (F4) |
| R7 | Two projects, one shared crate, diverging needs | Medium | The shared crate stays free of both products' concerns; composition-specific normalization lives in BibleCompose ([ADR-001](adr/001-usfm-core.md)) |
| R8 | Packaging SILE with HarfBuzz, fontconfig, ICU, and libtexpdf across three platforms | Medium | Treated as its own milestone item, not as a release-week task |

---

## 6. What this review does not change

Recorded so that the design is not read as a rewrite.

The project-folder-as-source-of-truth model; TOML for settings and styles with the two-file split; the defaults-then-overrides cascade; the requirement that no build modifies USFM; the layering intent that Bible semantics never know SILE syntax; the diagnostic model with stable codes; the marker support table in §9.2; the acceptance scenarios in §16.2; the MVP boundary in §17.1 and §17.2; and every non-goal in §2.3. All adopted as written.
