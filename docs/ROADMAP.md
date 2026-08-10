# Roadmap

The shape of the work and why it is ordered as it is.

**This is not a status board.** What is written here should still be true after v1 ships: what each stage delivers, why the sequence is what it is, and where each guarantee first gets established.

It is a resequencing of SRS §17.3, not a replacement. Six of its seven milestones survive with their content intact; what changes is what comes before M0 and what moves earlier. Reasons in [SRS-REVIEW](SRS-REVIEW.md).

Related: [SRS-REVIEW](SRS-REVIEW.md) · [ARCHITECTURE](ARCHITECTURE.md)

---

# Part 1 — Milestones

| | Milestone | For | What it means | Items |
|---|---|---|---|---|
| **S0** | Typesetting spike | nobody | We know SILE can set a Bible page. No Rust. | 8 |
| **M0** | Skeleton and contract | nobody | The pipeline exists end to end on one book. Ugly, deterministic, tested. | 10 |
| **S1** | Packaging spike | nobody | We know what a single binary costs, and whether Windows is a wall. | 5 |
| **M1** | USFM to PDF | us | Real Scripture through the real parser, in two columns. | 9 |
| **M2** | Configuration | us | Page, typography, and output settings, from file and from the GUI. | 10 |
| **M3** | Styles | first outside testers | The visual layer, editable without TOML. First build worth showing. | 8 |
| **M4** | Publishing structures | wider testers | Footnotes, cross-references, figures, running heads. A real Bible. | 8 |
| **M5** | Hardening | wider testers | Full corpus, fonts, cancellation, caches, packaging. | 9 |
| **M6** | Version 1.0 | everyone | Installers, presets, documentation, the ten acceptance scenarios. | 6 |

### S0 — Typesetting spike

*Days, not weeks. No Rust, no parser, no GUI.*

Hand-write SILE input for one realistic Scripture page and produce a PDF: two balanced columns, footnotes at the foot, cross-references, a running head carrying a verse range, a chapter opening, in Latin and in Tamil.

**It comes first because it is the only genuinely unproven thing in the product.** Everything else — parsing, TOML merging, a three-pane desktop UI — is work whose feasibility nobody doubts. Whether SILE sets an acceptable Bible page, and how much custom Lua that takes, decides the shape of [ADR-002](adr/002-sile-interface.md)'s class and therefore the shape of the emitter. SRS §17.3 discovers this at M4. Prior art to mine: [Freely-Given-org/BibleTypesetter](https://github.com/Freely-Given-org/BibleTypesetter).

It also settles smaller unknowns at no extra cost: how SILE's XML input maps elements to commands, whether namespace prefixes survive that mapping, how running-head marks behave across a column break, and whether PDF artwork can be placed as a figure (SRS §4.2 lists `creation-map.pdf`, and vector versus raster is a print-quality decision).

**Done when** a PDF exists that a typesetter would call acceptable, and the SILE source for it is checked in as the seed of `sile/classes/biblecompose.lua`.

**Outcome: yes, with a correction.** SILE sets a Bible page in Latin and in Tamil — balanced two columns, footnotes at the foot, running heads carrying the live verse range, vector artwork. But its bundled `bible` class typesets only when passed no options at all, and its two-column mode has never run, so [`sile/classes/biblecompose.lua`](../sile/classes/biblecompose.lua) is ours from the first line. The spike also moved four requirements and widened the pre-flight layer from fonts to a category ([ARCHITECTURE §7](ARCHITECTURE.md#7-pre-flight)). Full record and evidence in [spike/NOTES.md](../spike/NOTES.md).

### M0 — Skeleton and contract

*No user-facing output.* The Rust workspace, the diagnostic model, the build state machine, the `Backend` trait, the XML emitter, and the CLI — driven by a **hand-built `ScriptureDocument`**, with no parser involved.

Hand-built is the point. It decouples M0 from `usfm-core`'s readiness (Part 2 §4), and it proves the second half of the pipeline before the first half exists. At the end of M0 a fixture document becomes a PDF through the real emitter, the real class, and the real process invocation.

**Guarantees established here, and never allowed to regress:** golden XML byte-comparison across platforms; ordered maps only on the emission path; build in a temporary directory with atomic publish; process-tree cancellation. Each is cheap now and expensive later ([SRS-REVIEW F4, F11, F12](SRS-REVIEW.md#f4--reproducible-is-two-different-claims-and-must-be-split)).

The CLI is built here, not post-MVP: it is how everything above is tested headlessly, which NFR-009 requires ([SRS-REVIEW F9](SRS-REVIEW.md#f9--the-cli-is-in-the-post-mvp-roadmap-but-is-required-by-the-mvp)).

### S1 — Packaging spike

*Days, not weeks. Runs alongside M1–M4 and must finish before M5 needs it.*

Build SILE from source on all three platforms and find out what a single distributable binary actually costs. [ADR-006](adr/006-single-binary.md) chose the shape — one executable that re-executes itself, keeping the process boundary — but the number that decides whether it is affordable has not been measured.

**It is separate from S0 because it asks a different question.** S0 asked whether SILE can set a Bible page; the answer was yes. S1 asks whether SILE can be *shipped*, and the two have almost nothing in common: one is typography, the other is four C libraries and a Lua rock tree on three operating systems.

**It comes before P5.7 rather than inside it** because a bad answer changes a requirement rather than a task. Spike F-1 established there is no prebuilt SILE for Windows or macOS, so both must be built from source. If Windows turns out to be impractical, that does not change [ADR-006](adr/006-single-binary.md) — it changes NFR-001's claim that Windows is a Tier-1 target, and that is worth knowing at M1 rather than at M5 with installers half-built.

**Done when** a single binary on at least one platform typesets a fixture with no SILE installed, the artifact size is recorded, and the Windows answer is known either way.

### M1 — USFM to PDF

**You can** point the CLI at a folder of USFM and get a two-column PDF: paragraphs, poetry `q1`–`q4`, section headings, chapter and verse numbers, common character styles.

`usfm-core` arrives ([ADR-001](adr/001-usfm-core.md)) and the normalization pass is written against it. Discovery, `\id` identification, canonical ordering. Still no GUI.

**You cannot yet** configure anything, style anything, or see a footnote.

**Established here:** no Scripture text is lost or reordered by normalization, asserted over the corpus.

### M2 — Configuration

**You can** open a project folder in a window, see its books, edit page size, margins, columns, font, body size, numbering, and output path, and build. Settings persist to `biblecompose.toml` with comments intact.

The GUI shell arrives ([ADR-003](adr/003-gui.md)) along with the whole configuration layer: `toml_edit`, the defaults merge, unit parsing, provenance ([ADR-005](adr/005-provenance.md)), `schema_version`, unknown-key diagnostics.

**Established here:** CFG-006 — a GUI write does not disturb the rest of the file.

### M3 — Styles

**You can** change how it looks — body typography, poetry indents, heading sizes, chapter and verse appearance, character styles — from the GUI or from `styles.toml`, with the inspector telling you where each value came from.

The style cascade, typed selectors, inheritance with cycle detection, and the resolved-style inspector (STY-008, which is free by now because of [ADR-005](adr/005-provenance.md)).

**This is the first milestone worth showing outside the project.** It does what the product name promises for simple editions. Suitable for a friendly typesetter with a Gospel and an opinion.

### M4 — Publishing structures

**You can** produce something that is recognizably a Bible: footnotes, cross-references in the note area, figures from project assets, running heads with book name and verse range, page numbers.

The heaviest layer of Lua class work, which S0 has already de-risked.

### M5 — Hardening

**You can** rely on it. The full 66-book corpus builds. Fonts are pre-flighted, so a missing font or an uncovered script is an error rather than a page of tofu ([SRS-REVIEW F5](SRS-REVIEW.md#f5--sile-substitutes-missing-fonts-silently-so-pdf-003-and-pdf-004-cannot-be-delegated)). Cancel works mid-build. Draft builds make the iterate-and-rebuild loop usable ([SRS-REVIEW F10](SRS-REVIEW.md#f10--build-time-is-the-dominant-fact-of-the-workflow-and-the-srs-does-not-confront-it)). Caches make reopening fast. The finished PDF opens in the platform's own viewer. SILE and its native dependencies are packaged for three platforms.

Packaging is called out as milestone content rather than release-week work, because HarfBuzz, fontconfig, ICU, and libtexpdf across Windows, macOS, and Linux is its own project.

### M6 — Version 1.0

Signed installers, default presets, documentation, performance and reliability work, and all ten of SRS §16.2's acceptance scenarios demonstrated.

---

# Part 2 — How the work is shaped

## 1. Why this order

**The unproven thing goes first.** S0 exists because the SRS's plan reaches its riskiest question at M4, three milestones deep. Everything downstream of the emitter is designed against what S0 demonstrates.

**The pipeline is proven end to end before it is proven wide.** M0 builds a PDF from a hand-written fixture through the real emitter and the real backend. A vertical slice first, then breadth — the alternative is discovering an integration problem after four layers are complete.

**Guarantees are established where they are cheap.** Determinism at M0, before there is anything to make deterministic. Atomic publish and process-tree cancellation at M0, before a build has anything to lose. Font pre-flight can wait for M5 because it needs real projects to be worth testing; determinism cannot, because retrofitting it means auditing every map in the codebase.

**The GUI arrives at M2, after the CLI.** Not because the GUI is unimportant, but because a headless build is what tests everything beneath it, and building the GUI first means the core is only ever exercised through a window.

## 2. Sizing

Sizes are relative effort, not schedule. They exist to signal where the risk sits, not to support a Gantt chart.

| | Meaning |
|---|---|
| **S** | A focused session. Well understood before it starts. |
| **M** | A few sessions. The common case. |
| **L** | Wide surface or genuinely intricate. Expect a second pass. |
| **XL** | Where the project can go wrong. A poor approach here costs weeks, not days. |
| **⏳** | Lead-time-bound. Little effort, unpredictable calendar. Orthogonal to size. |

73 items: 19 S, 46 M, 6 L, 2 XL (S1.4 dropped from L to M when [ADR-006](adr/006-single-binary.md) moved to option C). Roughly comparable in scale to `easy-usfm`, which is not a coincidence — most of the difference is that BibleCompose does not build a parser ([ADR-001](adr/001-usfm-core.md)) and does not build an editor.

**The two XL items are worth knowing by name.** S0.2, the two-column Scripture page, is the item that decides whether the product is buildable as designed; it is XL despite being days of work, because a wrong answer there invalidates the emitter. P1.5, USJ-to-`ScriptureDocument` normalization, is the semantic heart — a bad model there is felt in styles, emission, and every diagnostic for the life of the project. Both deserve a design pass before code.

## 3. Items with lead time

Five items are gated by something outside the work itself. Their calendar is not their effort, so they benefit from being started well before their phase.

| ID | Gated by | Typical wait |
|---|---|---|
| **P1.1** | `usfm-core` extraction depends on `easy-usfm` reaching its own M0 | weeks, and see §4 |
| ~~**S1.2**~~ | ~~Building SILE's C dependencies under MSYS2~~ — **resolved**, and not that way: cross-compiled from Linux ([S1-NOTES P-9, P-10](../spike/S1-NOTES.md)) | *done* |
| **P5.7** | Packaging SILE's native dependencies may need upstream fixes on at least one platform; answered in outline by S1 | unpredictable |
| **P6.1** | Code-signing certificates must be purchased and issued | 3–10 business days |
| **P6.2** | Default fonts need redistribution terms confirmed with rights holders | days to weeks |

## 4. The dependency on easy-usfm

> **Resolved, and it never bit.** By the time P1.1 started, `usfm-core` was implemented, tested and corpus-backed; the extraction was a rename plus one small API addition. What follows is kept as written because the reasoning was sound and the mitigation is still the right one if a shared dependency ever does slip.

BibleCompose's M1 needs `usfm-core`, which is `easy-usfm`'s M0 and does not exist yet ([ADR-001](adr/001-usfm-core.md)). This is the largest scheduling risk in the plan and it is managed rather than avoided.

**BibleCompose needs a subset.** Batch whole-file parse, source spans, diagnostics, verse index. It does not need the incremental chapter-chunked session, which is the larger and harder half of that milestone. So the extraction can be useful to BibleCompose well before `easy-usfm`'s own M0 is complete.

**S0 and M0 do not need it at all.** Eighteen items, between them, that touch no parser. That runway is a further reason for the ordering above.

**One straightening-out belongs to the extraction itself.** `easy-usfm-core` treats UTF-16 offsets as its boundary type because it crosses into JavaScript. BibleCompose has no such boundary and must not pay the conversion. `usfm-core` should expose byte and line/column offsets natively, with UTF-16 as the WASM layer's concern — arguably better for `easy-usfm` too, and cheapest to do at the moment the crate is extracted.

**If it slips**, the fallback is that BibleCompose depends on `usfm3` directly behind its own thin facade, and converges on `usfm-core` later. That is [ADR-001](adr/001-usfm-core.md)'s rejected option B, taken as a schedule mitigation rather than as a design; the facade is what makes the retreat cheap and it is the reason the facade exists in either project.

## 5. What each milestone deliberately leaves broken

| | Leaves broken |
|---|---|
| S0 | Everything. It is a PDF and some Lua |
| M0 | No USFM is read. Fixtures only |
| M1 | Nothing is configurable. No window |
| M2 | Everything looks like the default |
| M3 | No footnotes, no cross-references, no figures |
| M4 | Slow, unpackaged, no font checking, cancel is best-effort |
| M5 | Unsigned, undocumented, no presets |

## 6. Reading the item tables

Each item is one coherent deliverable. **Done includes tests and green CI**, not code that runs locally. The *Done when* column is the acceptance test — it is written to be checkable by someone who did not do the work, and where it restates an SRS requirement the ID is named so the trace is visible.

---

# Part 3 — Work items

## S0 · Typesetting spike

*8 items · 4 S, 3 M, 1 XL. No Rust in this phase.*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **S0.1** | S | SILE 0.15 pinned; toolchain and native dependency versions recorded | A one-line document produces a PDF; SILE, HarfBuzz, ICU, and libtexpdf versions are written down and reproducible on a second machine |
| **S0.2** | **XL** | Two-column Scripture page: frames, column balancing, body typography, break behaviour | A page of real Scripture sets in two balanced columns with acceptable justification; no verse number is stranded at the foot of a column; the SILE source is checked in |
| **S0.3** | M | Footnotes and cross-references in the note area | Notes land on the page carrying their caller; a note too long for its page splits or migrates without orphaning the caller; cross-references are visually distinguishable from footnotes |
| **S0.4** | M | Running head with a live verse range, folio, chapter opening | The head shows book name and the first and last reference actually present on the page, correct across a column break and on a page whose first verse began on the previous one |
| **S0.5** | M | The same page in Tamil, from a font file that is not installed | Renders with no missing glyphs from `assets/fonts/`; line breaking and conjunct shaping are acceptable to a reader of the script |
| **S0.6** | S | XML input path: the same page driven from XML rather than `.sil` | Element names resolve to class commands; a backslash, a brace, and a percent sign in text pass through as literal characters; namespace-prefix behaviour is recorded either way ([ADR-002](adr/002-sile-interface.md)) |
| **S0.7** | S | Figures: raster and vector artwork | A JPG places with configured sizing; PDF artwork either places or is recorded as unsupported with a decision on what BibleCompose does instead ([SRS-REVIEW F14a](SRS-REVIEW.md#f14--smaller-gaps-and-conflicts)) |
| **S0.8** | S | Findings written up; the spike source becomes the seed class | `sile/classes/biblecompose.lua` exists; [ADR-002](adr/002-sile-interface.md) is amended with what the spike proved and what it disproved |

## Phase 0 → M0 · Skeleton and contract

*10 items · 3 S, 6 M, 1 L*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P0.1** | S | Workspace scaffold, seven crates, CI matrix on three platforms | `cargo test` green on Windows, macOS, and Linux from an empty workspace; a CI assertion fails the build if any crate other than `biblecompose-app` depends on `biblecompose-sile` ([ADR-004](adr/004-no-layout-crate.md)) |
| **P0.2** | S | `biblecompose-diagnostics`: `Diagnostic`, `Severity`, `Stage`, codes, `SourceLoc` | Every code carries its stage as a prefix; a test asserts a code cannot be constructed with a mismatched stage (DIA-001) |
| **P0.3** | M | `ScriptureDocument` and its fixtures, hand-built | Fixtures covering paragraph, poetry, heading, list, table, chapter and verse anchors, note, cross-reference, figure, and `Unsupported` compile and serialize ([ARCHITECTURE §5](ARCHITECTURE.md#5-the-three-models)) |
| **P0.4** | L | The XML emitter and the `Backend` trait; close the two defects S0 left in the class | A fixture document emits XML that the class accepts and SILE turns into a PDF; the emitter's input type cannot carry provenance ([ADR-005](adr/005-provenance.md)); page 1 follows the frame chain so column B is not empty on the first page of a book ([spike F-8](../spike/NOTES.md)); every value crossing into SILE machinery is already a string, not a content node ([spike F-9](../spike/NOTES.md)) |
| **P0.5** | M | Determinism harness: golden XML, ordered-maps lint | Byte-identical output over 100 runs on all three platforms; a `HashMap` introduced anywhere on the emission path fails CI (SILE-005, DET-001); PDF comparison strips the random font-subset prefix, which is the only reason the PDF itself is not byte-stable ([spike F-15](../spike/NOTES.md)) |
| **P0.6** | M | Process invocation, version detection, output capture | Backend version appears in the build log; every line of SILE stdout and stderr reaches the log with nothing dropped (SILE-002, SILE-006) |
| **P0.7** | M | Build directory, atomic publish, locked-destination diagnostic | Killing the process mid-build leaves the previous PDF byte-identical and no partial file at the output path; a destination locked by another process produces BLD-011's actionable message (BLD-009, BLD-010) |
| **P0.8** | M | Process-tree cancellation — Job Object on Windows, process group on Unix | Cancel during typesetting leaves no SILE process on any platform, verified by process enumeration rather than by exit code; UI-equivalent state is operable within 1 s (BLD-006) |
| **P0.9** | M | `biblecompose-cli`: `build`, `emit`, `validate`, `version` | The full fixture-to-PDF pipeline runs headless with no GUI crate compiled into the binary (NFR-009) |
| **P0.10** | S | Build state machine and event stream | All eight states of GUI-006 are observable in order from the CLI event log, including the cancelled and blocked paths |

## S1 · Packaging spike

*5 items · 1 S, 3 M, 1 L. **Four done, one skipped — S1 is complete.** Findings in [S1-NOTES](../spike/S1-NOTES.md); what it leaves for P5.7 is at the end of that file.*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **S1.1** | M | Build SILE 0.15.13 from source on Linux, and record what it actually needs | `./bootstrap.sh && ./configure && make` produces a working binary; the C libraries, their versions, and the Lua rock handling are written down. **The cargo question is already answered — no** ([S1-NOTES P-1](../spike/S1-NOTES.md)): `src/embed.rs` ships only as an autotools template, and the binary links seven static libraries built from C sources cargo never sees |
| **S1.2** | L ✅ | The same on Windows | **Done.** Not under MSYS2 but cross-compiled from Linux with mingw-w64: four lines of patch to SILE, ICU taken from MSYS2's package, reproduced by [s1-windows-cross.sh](../spike/s1-windows-cross.sh). Typesets `john_1_1_5.xml` natively on Windows 11 to a 6×9in PDF whose text is identical to the Linux build's ([P-9](../spike/S1-NOTES.md), [P-10](../spike/S1-NOTES.md)). NFR-001's Tier-1 claim stands |
| **S1.3** | M ⏭️ | The same on macOS | **Skipped.** No macOS available, and S1.2 answered the question S1 existed to ask. Reopen before P5.7, which cannot ship a macOS build without it |
| **S1.4** | M ✅ | One binary that carries SILE and extracts it once | **Done** ([P-11](../spike/S1-NOTES.md)). An 80.5 MB `biblecompose.exe` builds a correct PDF on Windows with no SILE installed and nothing configured: cold run 1.27 s, warm 0.12 s, cache directory named from the bundle's contents. Two gaps only running it could find — `--version` needs the runtime environment as much as a build does, and Windows has no system fontconfig to fall back on |
| **S1.5** | S ✅ | Measure and write up; settle ADR-006 | **Done.** Sizes measured on both platforms — 15 MB Linux, 78 MB Windows, the gap almost entirely ICU. [ADR-006](adr/006-single-binary.md) is **Accepted**, having **changed from option B to option C**: B's distinguishing claim ("nothing on disk") was false, and what remained of the distinction did not survive measurement |

Not an item, but the thing to watch: **ICU data is 32 MB of the 78**, and filtering it is the only large size lever. It needs the SRS to say which scripts are supported — see [ADR-006's consequences](adr/006-single-binary.md#consequences), including the silent-failure hazard if the break dictionaries are filtered out with everything else.

## Phase 1 → M1 · USFM to PDF

*9 items · 2 S, 6 M, 1 XL*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P1.1** | M ✅ | `usfm-core` extracted from `easy-usfm`; byte and line/column offsets native; both repositories consume it | Both projects build against the shared crate; no UTF-16 conversion occurs on BibleCompose's path; no `usfm3` type appears in the crate's public API ([ADR-001](adr/001-usfm-core.md)). **Done** — renamed and merged as [easy-usfm#7](https://github.com/samueldotj/easy-usfm/pull/7); `biblecompose-scripture` depends on it at a pinned revision and passes its 43 diagnostic codes through unchanged. Two of this item's premises were already false when it started — see [ADR-001's qualifications](adr/001-usfm-core.md#consequences) |
| **P1.2** | S ✅ | Composition corpus: whole-book fixtures on top of `usfm-core`'s | At least one complete book per feature class and per script in the coverage list; pinned by checksum; verify harness passes. **Done** — 13 whole books, 1.2 MB, chosen by set cover; the harness re-derives scripts and features from the bytes rather than trusting the manifest ([corpus/README.md](../corpus/README.md)) |
| **P1.3** | M ✅ | Project discovery: recursive scan, `\id` identification, duplicates, generated-directory exclusion | A renamed `MAT` file loads as MAT; two files declaring `\id MAT` block the build; `output/` and `.biblecompose/` never produce duplicate inputs; no code path opens a `.usfm` file for writing (PRJ-002 – PRJ-006, BLD-004). **Done** — nine tests, one per requirement, against real directories. BLD-004 is asserted against the architecture: a test fails if any non-test source in the crate opens a file for writing |
| **P1.4** | M ✅ | Canon table as data, including deuterocanonical books; ordering and inclusion | GEN precedes EXO regardless of filename; configured order and configured exclusions are both reflected; adding a deuterocanonical book is a row, not a schema change (BOOK-001 – BOOK-003). **Done** — the 84-row table was already in place from P0.3; this added `BookPlan`, the selection and ordering policy over it. Twelve tests. Filling it from `biblecompose.toml` is M2's job, so the policy takes resolved settings rather than reading them |
| **P1.5** | **XL** ✅ | USJ → `ScriptureDocument` normalization | Paragraphs, poetry, headings, lists, chapter and verse anchors, character styles, and notes normalize across the corpus; unknown markers survive as `Unsupported` with a location rather than being dropped (FUN-001, FUN-003, USFM-004). **Done** — 16 unit tests plus a smoke run over `usfm-core`'s 200-file corpus, which took distinct unsupported-marker diagnostics from 18 to 0 by normalizing introduction matter and note internals. One genuine text loss remains and is upstream: a bare `|` in paragraph text discards the rest of the line, which matters because that is the danda in Indic scripts |
| **P1.6** | M | Text-loss and ordering assertion over the corpus | Concatenated Scripture text of the normalized model equals that of the source, book by book, on every corpus file; no reordering across book or chapter boundaries (FUN-002, USFM-005) |
| **P1.7** | M ✅ | Emitter coverage for the M1 construct set; two-column body from the class | A real Gospel emits and typesets end to end; golden XML for every M1 construct. **Done** — the Berean Mark, 41 pages, 6×9in, running heads tracking the verse range. The construct golden found three emitter defects that unit tests had not, including two producing invalid XML |
| **P1.8** | M | Parallel parse across files; the NFR-002 budget | A 66-book project parses and validates in under 5 s cold on reference hardware; the benchmark runs in CI and a 20 % regression fails the build |
| **P1.9** | S | Emitted-line → Scripture-reference map | A forced SILE error deep in a book reports a Scripture reference, not an XML line number (SILE-007) |

## Phase 2 → M2 · Configuration

*10 items · 3 S, 6 M, 1 L*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P2.1** | M ✅ | Tauri 2 + Svelte + Vite + TypeScript scaffold; platform service interfaces; CI matrix | An empty window builds and runs on three platforms; a lint fails the build if a Svelte component imports a Tauri API directly ([ADR-003](adr/003-gui.md)). Both halves compile, the lint is verified by breaking it, and the window has been run — which is how the Vite watcher fault that killed `tauri dev` was found |
| **P2.2** | M ✅ | `toml_edit` layer: one parse, typed view derived from it, spans retained | Syntax errors report file, line, and column; the typed view and the format-preserving document cannot disagree because there is one parse (CFG-003). The second parser is kept out of reach rather than merely unused — `toml` is not a dependency of the config crate and `toml_edit`'s `serde` feature is off, both asserted in the architecture test |
| **P2.3** | M ✅ | Settings schema, embedded defaults, field-by-field merge, `schema_version` | A USFM-only folder builds from embedded defaults; changing only `page.width` leaves unrelated defaults intact; an unknown schema version is one clear diagnostic (CFG-001, CFG-002, CFG-008). Resolved settings reach the page as SILE class options, verified against rendered PDFs; a test compares what the application sends against what the class declares, in both directions |
| **P2.4** | M ✅ | Unit and value parsing — lengths, page sizes, enums, ranges | `"0.55in"`, `"11.5pt"`, `"6x9in"` become typed values at the configuration boundary; an invalid unit is diagnosed by BibleCompose with a location, never by SILE. **Taken before P2.3**, so the schema is written in typed values rather than in strings that P2.4 would then have to replace |
| **P2.5** | S ✅ | Unknown-key detection with spans; strict mode | `page.wdith` is reported at its own line; strict mode promotes it to an error (CFG-004). The set of legal keys is what resolution asked for, so it cannot drift from the schema; an unknown *table* is one complaint at its header rather than one per key inside it |
| **P2.6** | M ✅ | Provenance — `Sourced<T>` and `Origin` — threaded through resolution | Every resolved settings value reports Builtin or File-with-location; a merge that fails to set an origin does not compile ([ADR-005](adr/005-provenance.md)). The typed side landed with P2.3, for the reason the ADR gives; this adds the string-keyed index CFG-007 and STY-008 need, and the test that it is complete |
| **P2.7** | M ✅ | Write-back preserving comments and ordering; reset to default | A GUI edit to one key leaves every comment, blank line, and key order in the file untouched — including the key's own alignment, so the only bytes that change are the value; removing an override restores the built-in value (CFG-005 – CFG-007). Reset deletes the key rather than writing the default into the file |
| **P2.8** | L ✅ | GUI: project pane with per-book status, settings forms, Build and Cancel, build log | Page size, margins, columns, font, body size, numbering, notes, and output path are all editable without touching TOML; the book list shows validation status; the log is copyable (GUI-001 – GUI-003, GUI-007). The form is generated from the schema, so a setting added to resolution gets a row without anyone adding one |
| **P2.9** | S ✅ | Background execution and the event bridge | The window stays interactive and Cancel stays usable throughout a multi-minute build; no long work runs on the UI thread (GUI-012, NFR-003). Two threads — one builds, one drains events onto the window — and `start_build` returns as soon as the work is handed over. **Not yet watched through a real multi-minute build in the window**; the mechanism is tested, the experience is not |
| **P2.10** | S ✅ | Diagnostics panel: severity and file filtering, click to book | Clicking a diagnostic selects the related book and shows its detail; filtering to errors in one book works; a blocked build lists every blocking issue at once, before any backend process starts (GUI-005, DIA-002, DIA-004). The pane's per-book badge and the panel's list count the same diagnostics, asserted in a test |

## Phase 3 → M3 · Styles

*8 items · 3 S, 4 M, 1 L*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P3.1** | M ✅ | Style schema, typed selectors, and a built-in style for every supported marker | Every marker BibleCompose claims to support renders without a project override; `paragraph.q1` and a same-named selector in another class cannot collide (STY-001, STY-003). The selector list is generated from the model's marker enums, so a marker added to the model gets a selector rather than silently going unstyled |
| **P3.2** | M ✅ | Cascade, single-parent inheritance flattening, cycle detection | A `styles.toml` override changes only what it names; `q2` inherits poetry properties from `q1`; a cycle is one diagnostic naming the cycle, not a stack overflow (STY-002, STY-007). One diagnostic per *cycle* rather than per selector that can reach one, and every resolved property carries `Builtin`, `File` or `Inherited` |
| **P3.3** | S ✅ | Unsupported selector and property diagnostics with a location | A misspelled property is reported at its line rather than silently ignored (STY-004), with the nearest legal name suggested; a misspelled *class* is reported as itself rather than through each property inside it; `strict` promotes both to errors, as it does for settings keys. The project's `styles.toml` is read by `project::open`, so these reach the panel |
| **P3.4** | M ✅ | Resolved style map emitted as data; the class applies it | Style values reach SILE as data, never as command fragments; golden XML covers the full property set ([ADR-002](adr/002-sile-interface.md)). Verified on paper: overriding three sizes in `styles.toml` moves exactly those three in the PDF |
| **P3.5** | L ✅ | Style editor GUI | Body font and size, paragraph spacing, heading size, poetry indent, chapter and verse appearance, footnote style, and common character styles are all editable and persist (GUI-004, STY-005). Every row says whether its value is built-in, from the project's file, or inherited from a named style — so [P3.6](#) is a read of what is already on screen |
| **P3.6** | S | Resolved-style inspector | For any element, the inspector shows each property's value and whether it came from the built-in set, a project file with a location, or inheritance from a named selector (STY-008) |
| **P3.7** | M | External edit and reload; changed-file detection | An external `styles.toml` edit is reflected after reload; an externally edited USFM file raises a changed-file indication (STY-006, FUN-006, FUN-007) |
| **P3.8** | S | Golden XML across the style matrix | Every selector class and every supported property has a golden case; a style change that should affect one selector is proven not to affect others |

## Phase 4 → M4 · Publishing structures

*8 items · 2 S, 6 M*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P4.1** | M | Footnotes: structured model, callers, class rendering, splitting, numbering policy | A `\f` note renders at its intended location with its caller intact, including a note that must split across pages; notes never overlap the last lines of a long column ([spike F-10](../spike/NOTES.md)); numbering restarts per the configured policy rather than running continuously through the book; submarkers `fr`, `ft`, `fq`, `fqa`, `fk`, `fl`, `fv` are structural, not flattened (SCR-003, USFM-002) |
| **P4.2** | M | Cross-references as a distinct type, in the note area | An `\x` reference renders in the configured placement and is styled independently of footnotes; `xo`, `xt`, `xk`, `xq` are represented separately (SCR-004, SCR-005) |
| **P4.3** | M | Figures: asset resolution, containment, sizing, missing-image policy | A figure renders from a relative project path with `src`, `alt`, `size`, `loc`, `copy`, and `ref` preserved; raster and vector artwork both place; a path escaping the project directory after canonicalization is a diagnostic — the backend enforces nothing here ([spike F-14](../spike/NOTES.md)); a supplied PDF's page box and inherited font subsets are accounted for; a missing image follows the configured policy (SCR-006, USFM-003) |
| **P4.4** | M | Running heads and folios with live verse ranges | Book name, reference range, and page number appear per the settings; the range is correct on a page whose first verse started earlier and across a column break |
| **P4.5** | S | Independent visibility switches | Chapter numbers, verse numbers, section headings, footnotes, and cross-references can each be hidden without altering USFM, and hiding a number does not remove its anchor (SCR-001, SCR-007) |
| **P4.6** | M | Section headings `s1`–`s4`, `\d`, `\sp`, `\r`; chapter openings | Each renders with its own resolved style; a heading keeps with the text that follows it rather than sitting alone at a column foot |
| **P4.7** | M | Lists and tables | `li1`–`li4`, `lim1`–`lim4`, and `tr`/`th#`/`tc#` render with correct indentation and column alignment |
| **P4.8** | S | PDF metadata and reference anchors | Title, language, author, and subject appear in PDF properties; verse anchors are carried far enough that destinations and bookmarks are a later minor release rather than a model redesign (PDF-005, SCR-008) |

## Phase 5 → M5 · Hardening

*9 items · 1 S, 7 M, 1 L*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P5.1** | M | Font resolution: project fonts first, system second, unresolved is blocking | A font absent from the system and from `assets/fonts/` blocks the build with a diagnostic naming the setting that requested it; a project-local `.ttf` renders on a machine where it is not installed (FONT-001, FONT-003) |
| **P5.2** | M | Coverage pre-flight from the font's character map; hyphenation-pattern table | A Latin-only font configured for Tamil Scripture produces a coverage error with an example reference, before SILE runs; a font used only for footnotes is checked only against footnote text; a language the backend has no patterns for disables hyphenation and says so, rather than borrowing another language's (FONT-002, FONT-004, PDF-004) |
| **P5.3** | M | Full 66-book corpus build; PDF structural assertions; vendored pinned test fonts | Page count, page geometry, embedded font list, per-page extracted text, and image presence asserted; the suite does not fail when a system font is updated (PDF-001 – PDF-003, DET-002) |
| **P5.4** | M | Draft builds and selected-book UI | A one-book draft after a single style change completes in a small fraction of a full-Bible build and is visibly marked as a draft (BLD-012, BOOK-004) |
| **P5.5** | M | Discovery and parse caches with the five-part invalidation key | Reopening a 66-book project lists books in under 500 ms warm; a change to configuration, styles, marker table, backend version, or application version invalidates the cache ([SRS-REVIEW F14d](SRS-REVIEW.md#f14--smaller-gaps-and-conflicts)) |
| **P5.6** | S | Open PDF and Open Output Folder; output-path pre-flight | The finished PDF opens in the platform's own viewer and the containing folder can be revealed (GUI-009). **There is no integrated preview** — GUI-008 is a SHOULD and [ADR-003](adr/003-gui.md#revision-the-preview-is-gone-and-most-of-this-argument-with-it) declines it. The pre-flight is the part that matters: an external viewer holds the output open, so a locked destination is now the ordinary case rather than an edge one, and it must be reported **before** a full-Bible build rather than after it (OUT-001, [SRS-REVIEW F10](SRS-REVIEW.md#f10--build-time-is-the-dominant-fact-of-the-workflow-and-the-srs-does-not-confront-it)) |
| **P5.7** | L ⏳ | Packaging SILE, HarfBuzz, fontconfig, ICU, libtexpdf, and the Lua rock tree for three platforms, on the shape S1 proved | A fresh machine with no SILE installed builds a PDF from the installer artefact; the application ships as one executable that re-executes itself to typeset ([ADR-006](adr/006-single-binary.md)), so cancellation and crash isolation are unchanged; `luasec` and `luasocket` are omitted, so the shipped runtime contains no TLS or socket code at all ([spike F-16](../spike/NOTES.md)); the advanced executable override still works for development (SILE-003, SILE-004, SILE-009) |
| **P5.8** | M | SILE error mapping table; raw log collapsed behind it | Each known backend failure class becomes a BibleCompose diagnostic with the raw text available but collapsed; an unmapped failure still surfaces rather than being swallowed (SILE-007, DIA-005) |
| **P5.9** | S | Offline end-to-end test; log hygiene review | A full open-edit-build session succeeds with the network disabled and issues zero requests; the build log contains no Scripture by default (NFR-004, NFR-010) |

## Phase 6 → M6 · Version 1.0

*6 items · 5 M, 1 L*

| ID | | Deliverable | Done when |
|---|---|---|---|
| **P6.1** | M ⏳ | Installers and code signing, three platforms | Signed artefacts install cleanly on a fresh machine each, with no security warning on first launch |
| **P6.2** | M ⏳ | Default presets and default fonts | Reader, standard two-column, and large-print presets each produce an acceptable PDF from a bare USFM folder; every bundled font's redistribution terms are confirmed |
| **P6.3** | L | Accessibility audit and fixes | The whole application is operable by keyboard; the audit passes at the agreed level; complex-script text entry works in every settings field (NFR-011) |
| **P6.4** | M | Localization scaffolding; English strings extracted | No user-facing string is hard-coded in business logic; a second locale can be added without touching Rust (NFR-012) |
| **P6.5** | M | The ten acceptance scenarios, scripted end to end | Scenarios A through J of SRS §16.2 all pass in CI, including the forced-backend-failure and cancel cases; source USFM checksums are unchanged after every one |
| **P6.6** | M | Documentation, release notes, v1.0 sign-off | A new user gets from a USFM folder to a PDF using the documentation alone; every MUST in the SRS is traced to a passing test or a recorded exception |

---

# Part 4 — Where each guarantee is established

This table records the mechanism behind each promise and the point in the build where it first holds — useful long after the work is done, because it says which test protects which guarantee.

| Guarantee | Mechanism | Established at |
|---|---|---|
| SILE can set a Bible page at all | a PDF a typesetter accepts | S0.2 |
| Scripture cannot inject Lua or SILE commands | XML text nodes; fixture carrying `\`, `{`, `%` | S0.6, held by P0.4 |
| No crate but the app reaches the backend | CI dependency assertion | P0.1 |
| Backend input is byte-identical everywhere | golden XML, three platforms, 100 runs | P0.5 |
| A failed build never replaces the last good PDF | kill mid-build, byte-compare the previous output | P0.7 |
| Cancel leaves no backend process behind | process enumeration after cancel, three platforms | P0.8 |
| The core is testable without a GUI | full pipeline through the CLI, no GUI crate linked | P0.9 |
| BibleCompose never writes USFM | no write path in any crate; checksums before and after | P1.3, held by P6.5 |
| No Scripture text is lost or reordered | corpus-wide concatenation comparison | P1.6 |
| A 66-book project parses within budget | CI benchmark, 20 % regression fails the build | P1.8 |
| A backend error names a Scripture reference | emitted-line → reference map | P1.9 |
| A GUI write does not disturb the rest of the TOML | round-trip test over commented fixtures | P2.7 |
| Why a value looks the way it does is answerable | origin on every resolved value | P2.6, surfaced at P3.6 |
| Style inheritance terminates | cycle detection with a naming diagnostic | P3.2 |
| Hiding a number does not lose its anchor | anchors asserted present with numbering disabled | P4.5 |
| No silent font substitution | resolution failure is blocking, before SILE runs | P5.1 |
| No missing-glyph boxes in output | cmap coverage pre-flight per style | P5.2 |
| PDF geometry and fonts match the settings | structural assertions against vendored pinned fonts | P5.3 |
| Nothing leaves the machine | full session captured with the network disabled | P5.9 |
| Logs carry no Scripture by default | log review against the corpus | P5.9 |
| Keyboard-only operation | accessibility audit | P6.3 |
| Every MVP acceptance scenario | SRS §16.2 A–J scripted in CI | P6.5 |

---

# Part 5 — Decisions deferred

Left open because answering them well needs either a rights holder or a thing that is not built yet.

1. **Default fonts.** A licensing question, not an architecture one. Gates P6.2, and S0.5 should use a candidate so the choice is informed by how it sets.
2. **USFM 2.x tolerance.** Belongs to `usfm-core` and its marker table; shapes P1.5's diagnostics but not its model.
3. **PDF artwork as a figure source.** S0.7 answers it; if unsupported, P4.3 needs a documented conversion or rejection path.
4. **Whether the deuterocanon appears in shipped presets.** The canon table carries those books from P1.4 regardless; whether a preset selects them is a product choice for P6.2.
5. **Cross-reference placement beyond the note area.** Centre-column is explicitly post-MVP; whether an inline or end-of-paragraph mode joins the footnote-area mode in v1 is a P4.2 decision once the note area is working.
6. ~~**Whether Windows stays a Tier-1 target.**~~ **Closed by S1.2: yes.** SILE cross-compiles from Linux and typesets correctly on Windows ([S1-NOTES P-9, P-10](../spike/S1-NOTES.md)). Two things replace this question rather than settling with it: upstream tests none of it, so every SILE upgrade is a Windows risk we absorb; and the Windows artifact is 78 MB against Linux's 15, of which 32 MB is ICU data — which turns "which scripts do we support?" into a live question the SRS has to answer.

---

# Part 6 — Deliberately excluded

Recorded so the boundary is defensible when the request arrives, and so a future reader can see these were considered rather than overlooked.

| Request | Answer |
|---|---|
| Centre-column or gutter references, diglot, interlinear | SRS §17.2. Post-MVP; the verse anchors in P4.8 are what keep them possible |
| Float and wraparound images, thumb indexing, cover generation | SRS §17.2 |
| PDF/X, CMYK, prepress profiles | SRS §17.2. The base PDF workflow has to be stable first |
| Page-level micro-adjustment, a visual page editor | SRS §2.3. Not what this product is |
| Study-Bible sidebars, multiple synchronized note streams | SRS §17.2 |
| Generated TOC, glossary, indexes | SRS §17.2, beyond basic peripheral content |
| A plugin system or project-provided Lua | SRS §17.2, and it would undo [ADR-002](adr/002-sile-interface.md)'s security property |
| Translation, editing, or Paratext-style collaboration | SRS §2.3. `easy-usfm` edits files; this composes them |
| A second typesetting backend | Not planned. [ADR-004](adr/004-no-layout-crate.md) accepts that adding one later costs a second emitter |
| In-process SILE instead of a child process | Rejected on evidence, not assumption — [ADR-006](adr/006-single-binary.md). The `sile` crate is linkable, but a Lua VM cannot be stopped mid-typeset (BLD-006) and a segfault in HarfBuzz would take the application with it (NFR-007). A single binary is had instead by re-executing it, which keeps both |
| Telemetry on by default | Never. SRS §15 |

---

## Keeping this document true

The item tables will drift as the work reveals itself — items split, merge, and get reordered, and that is expected. The parts meant to outlast them are **Part 1** (what each milestone delivers), **Part 2's ordering rationale**, and **Part 4** (which test protects which guarantee). If an item changes, update the table. If one of those three stops being true, something more significant has changed and it deserves a note in the relevant [ADR](adr/).
