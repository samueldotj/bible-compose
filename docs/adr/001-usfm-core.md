# ADR-001 — Share `usfm-core` with easy-usfm rather than build a parser

**Status:** **Accepted** — done and consumed. `usfm-core` is the crate's name as of [easy-usfm#7](https://github.com/samueldotj/easy-usfm/pull/7), and `biblecompose-scripture` depends on it at a pinned revision
**Closes:** SRS §19 open decision *"USFM parser strategy"*

## Context

SRS §12.1 specifies two crates — `biblecompose-usfm` (parser, AST, validation, source spans) and `biblecompose-model` (normalized Scripture document model) — and §19 leaves open whether to *"build a dedicated Rust parser vs. adapt an existing compatible parser."* Together these are the largest work item in the plan and the one everything else depends on.

The requirement is a parser that is error-tolerant, produces accurate source spans, never crashes on hostile input, preserves unknown markers rather than dropping them, and reports diagnostics a publisher can act on (USFM-001 through USFM-006).

**The sibling project has already answered this question.** `easy-usfm` is a USFM editor by the same author, in the same language, with a completed design and an ADR of its own that evaluated three options and chose to wrap the [`usfm3`](https://crates.io/crates/usfm3) crate behind a facade. `easy-usfm-core` provides:

- a facade over `usfm3`, pinned exactly, with the parser named nowhere in its public API
- a USJ document tree with source spans, and an honest `Option` where `usfm3` has no span to give
- a marker table generated from the specification, carrying `since` and `deprecated_in`, from which diagnostic severity is derived rather than hardcoded
- stable diagnostic codes
- a verse index covering ranges, segments, `\va`, and `\vp`
- a ~200-file vendored corpus chosen for script and feature coverage, plus a fetched long tail
- a three-way differential oracle against `usfm3` directly and `usfm-grammar`
- a `cargo-fuzz` target with a 24-hour clean run gating releases

`easy-usfm`'s own architecture note says `easy-usfm-core` *"is a candidate for its own repository."* BibleCompose is the second consumer that makes that extraction worth doing.

## Options

### A — Write a BibleCompose parser (rejected)

Rebuilds, in a second codebase, a marker table, a diagnostic taxonomy, a corpus, a differential oracle, and a fuzz target that already have a design. The two would then drift: an error reported one way in the editor and another way in the compositor, for the same file. For a user who edits in `easy-usfm` and composes in BibleCompose — the intended user of both — that is not an inconvenience, it is the products contradicting each other.

### B — Depend on `usfm3` directly (rejected)

Cheaper than A, and it skips the facade. But `easy-usfm`'s ADR-001 documents why the facade exists, and every reason applies here identically: `usfm3` is five months old at 0.2.1, one maintainer, pre-1.0, with breaking changes reserved at the author's discretion. It also documents four concrete pieces of friction — spans held in a parallel tree, text leaves with no location at all, unrecognized marker names leaked per call via `Box::leak`, and `parse()` copying the source. Depending on it directly means BibleCompose meets all four independently and works around each in its own way.

### C — Extract `usfm-core` as a shared crate (chosen)

One crate, consumed by both products, with `usfm3` an implementation detail of it.

## Decision

**Extract `easy-usfm-core` into a standalone `usfm-core` crate and depend on it from both projects. BibleCompose adds normalization on top; it does not add a parser.**

The seam is precise, and getting it right is what keeps this from becoming a shared crate that serves neither product:

| | `usfm-core` | `biblecompose-scripture` |
|---|---|---|
| Answers | what does the file say | what is the publication |
| Model | USJ, source-faithful, spans | `ScriptureDocument`, canon-ordered |
| Scope | one file | the project |
| Knows about | markers, USFM versions | canon order, book inclusion, figures as assets |
| Diagnostics | `USFM-*` | `SCR-*`, `CFG-*`, `FONT-*` |

**Nothing composition-specific goes into `usfm-core`.** No canon table, no book ordering, no style resolution, no notion of a project. If a change to BibleCompose needs a change in `usfm-core`, that is the signal to check whether the change belongs above the seam instead.

Diagnostics from `usfm-core` pass through to the BibleCompose panel unchanged rather than being re-coded, so a message means the same thing in the editor and in the compositor.

**How that is actually held**, since "unchanged" is easy to write and easy to erode. `biblecompose-diagnostics` mirrors upstream's 43 codes as declarations — it cannot depend on `usfm-core` ([ARCHITECTURE §2](../ARCHITECTURE.md#2-layering) has it depending on nothing) and `Deserialize` resolves against `ALL`, so an unlisted code would make a build log fail to round-trip. `biblecompose_scripture::usfm::code_for` then maps every `usfm_core::DiagnosticCode` through an **exhaustive match**: a code added upstream stops our build rather than reaching a user as an unrecognised string. A catch-all arm would have been the natural thing to write and would have turned every future upstream diagnostic into a silently mislabelled one.

Both projects mint `USFM-*` codes, which is a collision waiting to happen. They stay disjoint because every upstream code carries a severity letter (`USFM-W001`) and every code of ours is bare digits (`USFM-003`) — asserted in a test rather than trusted, because on the day they collide two different conditions share one identifier and suppression settings start hiding the wrong thing.

## Consequences

**What BibleCompose no longer builds.** A lexer, a CST and AST, a marker table with version metadata, span plumbing, verse-range parsing, a diagnostic severity model, a corpus with licensing cleared, a differential oracle, and a fuzz harness. SRS §16.1's first two test layers arrive with the dependency.

**What BibleCompose still builds.** USJ-to-`ScriptureDocument` normalization, the canon table, note and reference structure, figure and asset resolution, and every diagnostic above the parser. This is real work and it is BibleCompose's own.

**Three qualifications.**

- ~~**`usfm-core` does not exist yet.**~~ **It did, and this was the ADR's most consequential error.** Written when `easy-usfm` was design-complete with no implementation, it planned BibleCompose's M1 around another project's unfinished M0. By the time P1.1 started the crate was 6,257 lines with ~200 tests, a 218-file corpus, and a fuzz target. The scheduling risk [ROADMAP §4](../ROADMAP.md#4-the-dependency-on-easy-usfm) is built around never materialised — worth recording, because the plan spent real caution on it.
- ~~**The UTF-16 offset space is not harmless.**~~ **Already the better design, before we asked.** This ADR wanted byte offsets native with UTF-16 confined to the WASM boundary. That is what the crate does, and more strictly than proposed: `ByteSpan` has no `Serialize` impl at all, so a byte offset *cannot* reach JavaScript, and `Char16` — the only serializable offset — can be produced solely by `Utf16Mapper`. The conversion is one narrow path rather than a convention. **Nothing had to be straightened out.** One gap was real and small: line numbers existed only as a method on `Utf16Mapper`, so asking "which line?" meant going through the type built to cross into JavaScript. `LineIndex` and `LineCol` are now the byte-native surface, and `Utf16Mapper` holds one rather than keeping a second line table.
- **The crate is still shaped for editing**, and that part stands. Chapter-chunked incremental reparse exists for a keystroke loop BibleCompose does not have. Unused, and harmless.
- **Two consumers make the API harder to change.** A shared crate is a commitment. The control is that `usfm-core` stays inside one repository owner's hands and both consumers are in-tree or path-linked during development, so a breaking change is one atomic commit rather than a release dance.

**The `usfm3` maturity risk is inherited, not added.** All four of `easy-usfm` ADR-001's controls carry over unchanged: the facade, the exact pin, the cheapness of forking 9,492 lines of MIT Rust, and becoming a visible downstream user. BibleCompose being a second consumer strengthens the last one.

**A product consequence worth naming.** With one engine underneath both, editing a file in `easy-usfm` and composing it in BibleCompose become the same pipeline. Whether that becomes an integration — the editor embedded as a panel, click a diagnostic in the compositor and land in the editor at the right verse — is a product question for later. Sharing the crate is what leaves the door open.

## References

[`easy-usfm` ADR-001](https://github.com/samueldotj/easy-usfm/blob/main/docs/adr/001-parser.md) · [`easy-usfm` ARCHITECTURE](https://github.com/samueldotj/easy-usfm/blob/main/docs/ARCHITECTURE.md) · [`usfm3` crate](https://crates.io/crates/usfm3) · [USFM 3.1](https://docs.usfm.bible/usfm/3.1.1/introduction.html)
