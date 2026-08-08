# Roadmap

The shape of the work and why it is ordered as it is.

**This is not a status board.** What is written here should still be true after v1 ships: what each stage delivers, why the sequence is what it is, and where each guarantee first gets established.

It is a resequencing of SRS §17.3, not a replacement. Six of its seven milestones survive with their content intact; what changes is what comes before M0 and what moves earlier. Reasons in [SRS-REVIEW](SRS-REVIEW.md).

Related: [SRS-REVIEW](SRS-REVIEW.md) · [ARCHITECTURE](ARCHITECTURE.md)

---

# Part 1 — Milestones

| | Milestone | For | What it means |
|---|---|---|---|
| **S0** | Typesetting spike | nobody | We know SILE can set a Bible page. No Rust. |
| **M0** | Skeleton and contract | nobody | The pipeline exists end to end on one book. Ugly, deterministic, tested. |
| **M1** | USFM to PDF | us | Real Scripture through the real parser, in two columns. |
| **M2** | Configuration | us | Page, typography, and output settings, from file and from the GUI. |
| **M3** | Styles | first outside testers | The visual layer, editable without TOML. First build worth showing. |
| **M4** | Publishing structures | wider testers | Footnotes, cross-references, figures, running heads. A real Bible. |
| **M5** | Hardening | wider testers | Full corpus, fonts, cancellation, caches, packaging. |
| **M6** | Version 1.0 | everyone | Installers, presets, documentation, the ten acceptance scenarios. |

### S0 — Typesetting spike

*Days, not weeks. No Rust, no parser, no GUI.*

Hand-write SILE input for one realistic Scripture page and produce a PDF: two balanced columns, footnotes at the foot, cross-references, a running head carrying a verse range, a chapter opening, in Latin and in Tamil.

**It comes first because it is the only genuinely unproven thing in the product.** Everything else — parsing, TOML merging, a three-pane desktop UI — is work whose feasibility nobody doubts. Whether SILE sets an acceptable Bible page, and how much custom Lua that takes, decides the shape of [ADR-002](adr/002-sile-interface.md)'s class and therefore the shape of the emitter. SRS §17.3 discovers this at M4. Prior art to mine: [Freely-Given-org/BibleTypesetter](https://github.com/Freely-Given-org/BibleTypesetter).

It also settles smaller unknowns at no extra cost: how SILE's XML input maps elements to commands, whether namespace prefixes survive that mapping, how running-head marks behave across a column break, and whether PDF artwork can be placed as a figure (SRS §4.2 lists `creation-map.pdf`, and vector versus raster is a print-quality decision).

**Done when** a PDF exists that a typesetter would call acceptable, and the SILE source for it is checked in as the seed of `sile/classes/biblecompose.lua`.

### M0 — Skeleton and contract

*No user-facing output.* The Rust workspace, the diagnostic model, the build state machine, the `Backend` trait, the XML emitter, and the CLI — driven by a **hand-built `ScriptureDocument`**, with no parser involved.

Hand-built is the point. It decouples M0 from `usfm-core`'s readiness (§3 below), and it proves the second half of the pipeline before the first half exists. At the end of M0 a fixture document becomes a PDF through the real emitter, the real class, and the real process invocation.

**Guarantees established here, and never allowed to regress:** golden XML byte-comparison across platforms; ordered maps only on the emission path; build in a temporary directory with atomic publish; process-tree cancellation. Each is cheap now and expensive later ([SRS-REVIEW F4, F11, F12](SRS-REVIEW.md#f4--reproducible-is-two-different-claims-and-must-be-split)).

The CLI is built here, not post-MVP: it is how everything above is tested headlessly, which NFR-009 requires ([SRS-REVIEW F9](SRS-REVIEW.md#f9--the-cli-is-in-the-post-mvp-roadmap-but-is-required-by-the-mvp)).

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

**You can** rely on it. The full 66-book corpus builds. Fonts are pre-flighted, so a missing font or an uncovered script is an error rather than a page of tofu ([SRS-REVIEW F5](SRS-REVIEW.md#f5--sile-substitutes-missing-fonts-silently-so-pdf-003-and-pdf-004-cannot-be-delegated)). Cancel works mid-build. Draft builds make the iterate-and-rebuild loop usable ([SRS-REVIEW F10](SRS-REVIEW.md#f10--build-time-is-the-dominant-fact-of-the-workflow-and-the-srs-does-not-confront-it)). Caches make reopening fast. The integrated preview lands. SILE and its native dependencies are packaged for three platforms.

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

## 2. What each milestone deliberately leaves broken

| | Leaves broken |
|---|---|
| S0 | Everything. It is a PDF and some Lua |
| M0 | No USFM is read. Fixtures only |
| M1 | Nothing is configurable. No window |
| M2 | Everything looks like the default |
| M3 | No footnotes, no cross-references, no figures |
| M4 | Slow, unpackaged, no font checking, cancel is best-effort |
| M5 | Unsigned, undocumented, no presets |

## 3. The dependency on easy-usfm

BibleCompose's M1 needs `usfm-core`, which is `easy-usfm`'s M0 and does not exist yet ([ADR-001](adr/001-usfm-core.md)). This is the largest scheduling risk in the plan and it is managed rather than avoided.

**BibleCompose needs a subset.** Batch whole-file parse, source spans, diagnostics, verse index. It does not need the incremental chapter-chunked session, which is the larger and harder half of that milestone. So the extraction can be useful to BibleCompose well before `easy-usfm`'s own M0 is complete.

**S0 and M0 do not need it at all.** Between them that is a meaningful runway during which the parser layer can land, which is a further reason for the ordering above.

**One straightening-out belongs to the extraction itself.** `easy-usfm-core` treats UTF-16 offsets as its boundary type because it crosses into JavaScript. BibleCompose has no such boundary and must not pay the conversion. `usfm-core` should expose byte and line/column offsets natively, with UTF-16 as the WASM layer's concern — arguably better for `easy-usfm` too, and cheapest to do at the moment the crate is extracted.

**If it slips**, the fallback is that BibleCompose depends on `usfm3` directly behind its own thin facade, and converges on `usfm-core` later. That is [ADR-001](adr/001-usfm-core.md)'s rejected option B, taken as a schedule mitigation rather than as a design; the facade is what makes the retreat cheap and it is the reason the facade exists in either project.

## 4. What is not in the plan at all

SRS §17.2's deferrals, unchanged: centre-column references, diglot, interlinear, float and wraparound images, thumb indexing, cover generation, PDF/X and CMYK, page-level micro-adjustment, study-Bible sidebars, generated TOC and indexes, a plugin system, a visual page editor.

Two additions to that list from this design. **A second typesetting backend** is not planned, and [ADR-004](adr/004-no-layout-crate.md) accepts that adding one later costs a second emitter. **In-process SILE** is not planned; the child process is what gives BLD-006 cancellation and NFR-007 its failure boundary ([ADR-002](adr/002-sile-interface.md)).
