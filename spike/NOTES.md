# S0 — Typesetting spike, working notes

Running log of what the spike establishes. Findings that survive get promoted into [ADR-002](../docs/adr/002-sile-interface.md) and [SRS-REVIEW](../docs/SRS-REVIEW.md) at S0.8; this file is the raw record, kept so a decision can be traced to what was actually observed.

**Environment:** WSL2, Ubuntu 22.04.5 LTS, x86_64. The host is Windows 11; SILE publishes no Windows binary (F-1), so the spike runs in WSL and the Windows packaging question is deferred to P5.7.

| Item | Status |
|---|---|
| S0.1 Toolchain pinned, versions recorded | **Done** |
| S0.2 Two-column Scripture page | **Substantially done** — works, with one open defect (F-8) |
| S0.3 Footnotes and cross-references | **Partial** — placement correct, two defects open (F-10) |
| S0.4 Running head with verse range | **Done** — works |
| S0.5 Tamil, uninstalled font | Not started |
| S0.6 XML input path | Not started |
| S0.7 Figures, raster and vector | Not started |
| S0.8 Write-up, seed class | Not started |

**The headline.** SILE can set a Bible page, and the mechanisms it needs are all present and working — mirrored two-column masters, column balancing, a full-measure footnote frame, and running heads carrying the live verse range of the page. **But its bundled `bible` class cannot be used**, and is not a starting point: it fails to typeset anything the moment you pass it an option (F-5, F-6). BibleCompose writes its own class. The spike's replacement is [`sil/biblecompose.lua`](sil/biblecompose.lua), 298 lines, which produces the page in [`out/render/`](out/render/).

---

## S0.1 — Toolchain

### What is pinned

| | Version | Provenance |
|---|---|---|
| SILE | 0.15.13 | `sile-x86_64`, GitHub release `v0.15.13` |
| | | sha256 `f9f875447ecade9515e984ee66039c67d64c99fbfc904f95fe7f1ed0edbfe194` |
| | | **verified** against the release's own `sile-0.15.13.sha256.txt` |
| Lua VM | LuaJIT 2.1.ROLLING | embedded in the SILE binary |
| Lua rocks | 19, pinned to `sile.rockspec.in` | built via a user-local Lua 5.1.5 + luarocks 3.11.1 |
| HarfBuzz | libharfbuzz.so.0 | system (Ubuntu 22.04) |
| ICU | libicuuc / libicui18n .so.70 | system |
| fontconfig, FreeType, Graphite2 | system | |

`sile --version` → `SILE v0.15.13 (LuaJIT 2.1.ROLLING) [Rust]`. Reproduce with [`setup-wsl.sh`](setup-wsl.sh).

### F-1 — There is no prebuilt SILE for Windows or macOS

Release v0.15.13 publishes one binary asset, `sile-x86_64` (14.7 MB, Linux). Everything else is source.

**Consequence for [P5.7](../docs/ROADMAP.md#phase-5--m5--hardening).** SRS NFR-001 names Windows and macOS Tier-1, and SILE-003 wants a bundled tested runtime. Neither can come from an upstream binary. BibleCompose must build SILE from source in CI for at least two of three platforms. P5.7's `L ⏳` sizing is right and its lead-time flag is mandatory, not cautious.

### F-2 — The Linux binary is not self-contained

`sile-x86_64` embeds LuaJIT and SILE's *own* Lua, but **none of its third-party Lua dependencies**. On a machine with every native library present it still fails:

```
Error: runtime error: module 'lua-utf8' not found
```

Its search path contains `/home/runner/work/sile/sile/…` — the GitHub Actions build directory. Resolving modules one at a time surfaced four native ones in sequence (`lua-utf8`, `lpeg`, `lfs`, `lxp`), each discoverable only by running and reading the next error. The real list is in [`sile.rockspec.in`](https://raw.githubusercontent.com/sile-typesetter/sile/v0.15.13/sile.rockspec.in): **19 rocks**.

**Consequences.**

- **"Just ship the binary" is not an available packaging strategy on any platform.** P5.7 must produce a runtime *tree* — binary plus a Lua module directory — not a file. That changes what the installer contains and what SILE-003's acceptance test proves.
- **`luaexpat` is on the required list**, and it is what parses XML input. [ADR-002](../docs/adr/002-sile-interface.md) chose XML; the path is present in a stock dependency set rather than being an optional extra. It is now load-bearing for BibleCompose and must be in the bundle — worth stating in ADR-002 at S0.8.
- **`luasec` and `luasocket` come with the standard set.** TLS and sockets. Nothing in BibleCompose's use of SILE should open a connection, and NFR-004 requires full offline operation. So P5.9's offline test must cover a *build*, not just application start; and S0.8 should check whether a build without those two rocks still typesets, because the smallest bundle that works has the least to explain.

### F-3 — The whole toolchain installs without root

Relevant because CI runners and locked-down machines are the normal case. Ubuntu 22.04 already carried the native libraries; Lua 5.1.5 (`make posix`, no readline), luarocks 3.11.1, and all 19 rocks built and installed into `$HOME` with **19 installed, 0 failed**. Only `expat`, `zlib`, and `openssl` headers were needed, and all three were already present.

Two wrinkles worth keeping:

- **lpeg 1.1.0 must be built from all six `.c` files.** The five that older instructions list produce a `.so` that loads and then fails with `undefined symbol: tree2cset`; `lpcset.c` is new in this version.
- **The Lua 5.1 ABI is what matters**, not Lua 5.1 itself — the binary embeds LuaJIT 2.1, which is 5.1-compatible.

---

## S0.2–S0.4 — The page

Artifacts: [`sil/john1-text.sil`](sil/john1-text.sil) (content), [`sil/biblecompose.lua`](sil/biblecompose.lua) (the class), [`sil/john1-bc-2col.sil`](sil/john1-bc-2col.sil) (appearance), [`out/john1-bc-2col.pdf`](out/john1-bc-2col.pdf), renders in [`out/render/`](out/render/).

Measured: 5 pages, 6.00 × 9.00 in exactly as configured, 2 fonts, both subset and **embedded** — PDF-003 satisfied by default rather than by effort.

### F-4 — SILE bundles a `bible` class, and it is a demonstration rather than a tool

`sile --class` lists `bible`, `diglot`, and `triglot` among bundled classes. [`upstream/classes-bible.lua`](upstream/classes-bible.lua) is 295 lines and its *architecture is sound* — this is the part worth keeping:

| Mechanism | What it does | Verdict |
|---|---|---|
| `masters` + `twoside` | mirrored page geometry across the spread | keep |
| `infonode` + `chapterverse` | collects references per page; `first-reference` / `last-reference` | **keep — this is the running-head answer** |
| `footnotes` | note frames that steal height from content frames | keep |
| `balanced-frames` | equalises column depth | keep |

But everything above the mechanism is hardcoded in ways the SRS forbids: frame geometry as fixed percentages (`8.3%pw`, `86%pw`, `11.6%ph`) against CFG-002's configurable margins; the English string `"Chapter "` built into the chapter command, which is unusable for a Tamil Bible; and `family = "Gentium"` pinned into the running head. Its two-column path also comments out `balanced-frames` (line 172) and routes every footnote into `footnotesB` with a `-- Later we'll have an option for two fn frames` note, so a note called in the left column would print under the right one.

**Decision: BibleCompose does not subclass `bible`.** It builds on the same packages. ADR-002's "the class is where Bible typesetting lives" survives unchanged; what changes is that the class is ours from the first line, and that is not a cost the roadmap had to absorb — it was already M4's work.

### F-5 — The `bible` class crashes on its own running-head path

`class:endPage` reads `SILE.scratch.headers.right`, and `registerCommand("left-running-head")` writes `SILE.scratch.headers.left` — but nothing ever creates `SILE.scratch.headers`. The class's own `\verse-number` calls `\left-running-head` on **every verse**, so the first verse of any document raises:

```
attempt to index field 'headers' (a nil value)
```

There is no `headers` package in 0.15.13 to supply the table — `\use[module=packages.headers]` fails to resolve — so a document has to create it itself.

### F-6 — The `bible` class's two-column mode is unreachable, and passing the option at all breaks it

Measured directly, stock class, 400 lines of filler, nothing custom:

| Invocation | Result |
|---|---|
| `\begin[class=bible]{document}` | **OK** — 64,455 bytes, single column |
| `\begin[class=bible, twocolumns=false]` | **FAIL** — 15-byte stub, no pages |
| `\begin[class=bible, twocolumns=true]` | **FAIL** — 15-byte stub, no pages |

Two defects compound:

1. **Option values arrive as strings, and `"false"` is truthy in Lua.** `setOptions` does `options.twocolumns or false` and `_init` does `if self_.options.twocolumns then`, so *any* supplied value selects two columns. The option cannot be turned off explicitly.
2. **`twoColumnMaster()` never loads `twoside`**, which is what provides `oddPage`. Only `singleColumnMaster()` loads it. So the two-column path reaches `endPage`, calls `self:oddPage()`, and dies on `attempt to call method 'oddPage' (a nil value)`.

Net: `bible` works only in the one configuration where you pass no options at all, which is single column. **Its two-column mode has never run.** Worth reporting upstream, along with F-5.

*The same class of bug bit our own class within an hour:* `plain.setOptions` invokes declared option setters **after** `setOptions` body runs, overwriting a `tonumber`-coerced column count with the raw string, so `o.columns >= 2` threw *attempt to compare number with string*. Coercion belongs in the setter. Any option BibleCompose passes to a SILE class must be treated as a string until explicitly converted — a rule for P0.4's emitter.

### F-7 — Frame arithmetic silently produces inverted frames

Deriving the folio's `bottom` from `marginbottom` while its `top` came from `bottom(footnotes) + footsep` put bottom *above* top. SILE does not reject this; it warns `Overfull frame folio: 45.83pt shrinkability required` on every page and carries on. A zero-or-negative-height frame is a plausible outcome of user-supplied margins, so **BibleCompose must validate resolved frame geometry itself** before emitting — the backend will not. Same shape as [SRS-REVIEW F5](../docs/SRS-REVIEW.md#f5--sile-substitutes-missing-fonts-silently-so-pdf-003-and-pdf-004-cannot-be-delegated): SILE warns where a publishing tool must block.

### F-8 — Page 1 does not follow the frame chain (open)

**The one unresolved defect.** Pages 2 onward are correctly two-column and balanced. On page 1, column A fills and column B stays empty; text continues at the top of page 2. Three attempts did not fix it:

1. `switchMaster("right")` at the end of postinit — no change.
2. Building the frameset from `options` and assigning `self.defaultFrameset` **before** `plain._init` — no change.
3. Moving `footnotes` and `balanced-frames` out of `registerPostinit` into `_init` — no change.

The frame solver confirms the shape of the problem: SILE solves the frameset **twice**, once for `plain`'s default (`content`, `folio`, `footnotes`) and once for ours (`contentA`, `gutter`, `contentB`, …). Page 1 is laid out against the first solve, and the `next = "contentB"` chain belongs to the second.

Not a blocker for the S0 verdict — the mechanism demonstrably works from page 2 — but it is unfinished, and it should be resolved before the class is trusted, because "the first page of every book is wrong" is exactly the defect that survives to print. Carry into P0.4.

### F-9 — `chapterverse` stringifies whatever it is handed

`save-verse-number` stores `content[1]` verbatim and `format-reference` later `tostring()`s it. Reached through a `\define`d command, `content[1]` is a content node rather than a string, so the running head rendered:

```
John table: 0x610460687de0:table: 0x610461c26d30–table: 0x610460687de0
```

Flattening with `SU.contentToString` at the command boundary fixes it. **This is a boundary rule for the emitter, not a one-off:** anything BibleCompose passes into SILE machinery that will later be stringified must already be a string. Cheap to get right at P0.4, and invisible until it reaches a running head.

### What the page proves

With those fixed, the output is a recognisable Bible page. From [`out/render/john1-bc-2col-p2.png`](out/render/):

- **Two columns, balanced**, justified, with hyphenation, at a 6×9 trim.
- **Running head reads `John 1:15–1:34`** — the live range of references actually on the page, left-aligned on the verso with the book name on the outer edge. **S0.4's question is answered yes**, and `infonode` + `chapterverse` is the mechanism.
- **Footnotes sit at the foot of the page across the full measure**, below both columns — which is what most Bibles do and is better than the per-column frames upstream was reaching for.
- Superscript verse numbers, section headings, parallel-reference lines, and poetry indents all behave.

### F-10 — Two note defects remain open

1. **Notes overlap the body text** when a column runs long. The footnote frame steals height from both content frames, but the balancer does not re-solve against the stolen height, so the last lines of a column collide with the first note. Visible at the foot of column 1 on page 2.
2. **Note numbering is continuous across the document**, so by page 2 the callers read 8 and 9. Real editions restart per page or per chapter, or use symbols. A policy question, but it needs a mechanism, and the mechanism is not free.

Both land on P4.1 and are the reason that item is `M` rather than `S`.

---

## Source text

Both passages are from the `easy-usfm` corpus, which has already cleared redistribution terms. Copied into [`source/`](source/) so the spike is self-contained and a later change to that corpus cannot silently alter what it was judged against.

| File | From | Content |
|---|---|---|
| [`source/john1-bsb.usfm`](source/john1-bsb.usfm) | Berean Standard Bible, public domain | John 1:1–34 — `s1`, `r`, `q1`/`q2`, 8 footnotes with `fr`/`ft`, `m`, `b` |
| [`source/lam1-tam.usfm`](source/lam1-tam.usfm) | Tamil, terms in the corpus manifest | Lamentations 1:1–12 — poetry, notes, `cl` |

Neither carries `\x` cross-references; the corpus files that do are all in languages I cannot proofread. Since S0 hand-writes SILE rather than parsing USFM, the cross-reference in `john1-text.sil` is authored from the `\r` parallel references already in John 1, which is what a real edition carries anyway.

## Tooling built here

- [`setup-wsl.sh`](setup-wsl.sh) — reproducible toolchain, root needed only for optional extras.
- [`inspect-pdf.py`](inspect-pdf.py) — page count, trim size, embedded fonts, without poppler. SILE's libtexpdf backend writes object streams, so the facts a PDF assertion needs are Flate-compressed and invisible to a byte search. This is the seed of P5.3's structural assertions.

---

## The eight questions

Carried from [SRS-REVIEW F2](../docs/SRS-REVIEW.md#f2--the-riskiest-unknown-is-scheduled-last), answered as the evidence arrives.

| | Question | Answer |
|---|---|---|
| 1 | Do two columns balance? | **Yes.** `balanced-frames` works. Verse stranding not yet stress-tested; page 1 chaining open (F-8) |
| 2 | Do footnotes stay on the page of their caller? | **Yes**, full measure at the foot. Overlap and numbering open (F-10) |
| 3 | Can a running head carry the page's reference range? | **Yes**, across a column break. `infonode` + `chapterverse` (F-9 fixed) |
| 4 | Uninstalled font, Tamil shaping? | Not yet — S0.5 |
| 5 | How does XML input map to commands? | Not yet — S0.6. `luaexpat` confirmed present (F-2) |
| 6 | Do `\`, `{`, `%` survive the XML path literally? | Not yet — S0.6 |
| 7 | PDF artwork as a figure? | Not yet — S0.7 |
| 8 | How much custom Lua? | **298 lines** for two-column mirrored masters, footnote frames, balancing, running heads with live ranges, and configurable geometry — replacing a 295-line upstream class that does less and does not run |
