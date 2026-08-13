# S0 — Typesetting spike, working notes

Running log of what the spike establishes. Findings that survive get promoted into [ADR-002](../docs/adr/002-sile-interface.md) and [SRS-REVIEW](../docs/SRS-REVIEW.md) at S0.8; this file is the raw record, kept so a decision can be traced to what was actually observed.

**Environment:** WSL2, Ubuntu 22.04.5 LTS, x86_64. The host is Windows 11; SILE publishes no Windows binary (F-1), so the spike runs in WSL and the Windows packaging question is deferred to P5.7.

| Item | Status |
|---|---|
| S0.1 Toolchain pinned, versions recorded | **Done** |
| S0.2 Two-column Scripture page | **Substantially done** — works, with one open defect (F-8) |
| S0.3 Footnotes and cross-references | **Partial** — placement correct, two defects open (F-10) |
| S0.4 Running head with verse range | **Done** — works |
| S0.5 Tamil, uninstalled font | **Done** — works, and found F-11 |
| S0.6 XML input path | **Done** — [ADR-002](../docs/adr/002-sile-interface.md) confirmed decisively |
| S0.7 Figures, raster and vector | **Done** — PNG, JPG and vector PDF all place |
| S0.8 Write-up, seed class | **Done** — class promoted, four documents amended |

**S0 is complete.**

**The headline.** SILE can set a Bible page, in Latin and in Tamil, with footnotes, live running heads, and vector artwork. Every mechanism needed is present and working. **But its bundled `bible` class cannot be used** and is not a starting point — it fails to typeset anything the moment you pass it an option (F-5, F-6). BibleCompose writes its own class; the spike's replacement is [`sile/classes/biblecompose.lua`](../sile/classes/biblecompose.lua), 298 lines, which produces the pages in [`out/render/`](out/render/).

**The pattern worth carrying out of S0.** Five separate times, SILE did the wrong thing without saying so: it substitutes a font that cannot render the text (F-12), applies another language's hyphenation (F-11), accepts inverted frame geometry (F-7), embeds artwork from outside the project (F-14), and — through SIL syntax — executes commands that arrive inside Scripture text (F-13). None of these produces a non-zero exit. Four produce a *clean, successful build*.

The design already anticipated one of these ([SRS-REVIEW F5](../docs/SRS-REVIEW.md#f5--sile-substitutes-missing-fonts-silently-so-pdf-003-and-pdf-004-cannot-be-delegated)). The spike's contribution is that it is not one, it is a category: **SILE is a typesetter, and a typesetter's job is to set what it is given. Refusing bad input is the application's job, and there is no part of it BibleCompose can delegate.** That is the argument for the pre-flight layer in [ARCHITECTURE §7](../docs/ARCHITECTURE.md#7-pre-flight), and it should be widened beyond fonts.

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

Artifacts: [`sil/john1-text.sil`](sil/john1-text.sil) (content), [`sile/classes/biblecompose.lua`](../sile/classes/biblecompose.lua) (the class), [`sil/john1-bc-2col.sil`](sil/john1-bc-2col.sil) (appearance), [`out/john1-bc-2col.pdf`](out/john1-bc-2col.pdf), renders in [`out/render/`](out/render/).

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

## S0.5 — Complex script and an uninstalled font

Artifacts: [`sil/lam1-text.sil`](sil/lam1-text.sil) plus four wrappers differing in one line each; renders in [`out/render/`](out/render/).

Font: Noto Serif Tamil v2.004, Regular and Bold, in [`assets/fonts/`](assets/fonts/). **fontconfig reports zero Tamil fonts on this machine**, so the face can only come from the file — the FONT-003 condition exactly.

### An uninstalled font file works, and Tamil shapes correctly

`\font[filename=…]` loaded the face, and the PDF carries `ORXORM+NotoSerifTamil-Regular` **subset and embedded**. The page ([`tamil-correct-p1.png`](out/render/tamil-correct-p1.png)) shows correct conjunct formation and vowel-sign placement, two balanced columns, footnote at the foot, and a **running head in Tamil carrying the range** — `புலம்பல்கள் 1:1–1:4`. The reference machinery is script-independent, which is worth knowing: it is not quietly Latin-only.

FONT-003 is satisfied by SILE without help. That is the only one of the three font requirements that is.

### F-11 — SILE hyphenates Tamil with visible hyphens, and setting the language does not stop it

The first Tamil page broke words as `ந-கரம்`, `வருபவர்-கள்`, `உட்கார்ந்தி-ருக்கிறாளே` — Latin-style hyphenation applied to a script that does not use it. Compare [`tamil-hyphenated-p1.png`](out/render/tamil-hyphenated-p1.png) against [`tamil-correct-p1.png`](out/render/tamil-correct-p1.png).

Isolated across three variants:

| Setting | Result |
|---|---|
| `\font[…, language=ta]` only | hyphenated — the font attribute drives OpenType shaping, not the hyphenator |
| `document.language = ta` | **still hyphenated** — SILE has no Tamil patterns and does not say so |
| `document.language = und` | **correct** — no hyphens, clean word-boundary wrapping |

`document.hyphenate` is not a setting; the lever is the language value.

**The failure mode is the important part.** Asking for Tamil does not get you "no hyphenation for Tamil", it gets you *another language's* hyphenation applied to Tamil, silently. A publisher who sets `language = "ta"` in `biblecompose.toml` — the obvious, correct thing to do — gets a Bible with wrong hyphens throughout and no diagnostic.

**Consequence.** BibleCompose must know which languages the pinned backend actually has patterns for, and map its `typography.language` accordingly: pattern available → pass it through; no patterns → pass `und` and, if the project asked for hyphenation, say why it is off. This is a table that ships with the class and is versioned with it (SILE-009). Lands on P2.4 (unit and value parsing) and P3.4. **A missing requirement** — proposed as FONT-004 in §4 below.

This is the third finding of the same shape, and the shape is now the point: **SILE warns or stays silent where a publishing tool must block.** Fonts ([SRS-REVIEW F5](../docs/SRS-REVIEW.md#f5--sile-substitutes-missing-fonts-silently-so-pdf-003-and-pdf-004-cannot-be-delegated)), frame geometry (F-7), hyphenation (F-11).

> **Correction, from implementing FONT-004.** The observation above holds and the explanation does not. SILE 0.15.13 *does* ship Tamil patterns — `languages/ta/hyphens-tex.lua`, auto-generated from TeX — and they are what hyphenated the page. Re-measured on one book of Lamentations: `ta` drew 510 hyphens, `am` and a nonexistent tag drew 7 (the number in the source text), and `en` drew 7 as well, because English patterns do not match Tamil letters. So a language with no patterns gets no hyphenation rather than another language's, and the fix is not the pattern table proposed here — a table of languages with patterns would have passed `ta` through. It is the script that decides. See [ARCHITECTURE §7.3](../docs/ARCHITECTURE.md#73-hyphenation).

### F-12 — A page of tofu is a clean, passing build

The control that settles PDF-004. Identical document, identical text, only the font changed to DejaVu Serif, which has no Tamil coverage at all:

- **Exit code 0. No warning. No error. No mention of a missing glyph.**
- A valid 14,633-byte PDF, one page, correct 6×9in trim, both fonts subset and **embedded**.
- Every Tamil character rendered as `.notdef` — [`tamil-no-coverage-p1.png`](out/render/tamil-no-coverage-p1.png) is a full page of empty boxes.

**This changes a design decision, not just a requirement.** [SRS-REVIEW F4](../docs/SRS-REVIEW.md#f4--reproducible-is-two-different-claims-and-must-be-split) specifies DET-002's PDF assertions as structural — page count, geometry, embedded font list, extracted text, image presence. **Every one of those passes on the tofu page.** Page count 1, geometry exact, fonts embedded, and the text layer extracts fine because the codepoints are present even though no glyph is.

So P5.3's structural assertions are necessary and **not sufficient**, and P5.2's codepoint-coverage pre-flight is the only thing between a project and a printed run of empty boxes. FONT-002 is not defence in depth; it is the sole defence. Worth saying plainly in the requirement.

---

## S0.6 — The XML input path

This is the experiment [ADR-002](../docs/adr/002-sile-interface.md) rests on. **It confirms the decision, and more sharply than expected.**

### How SILE maps XML

| Probe | Result |
|---|---|
| `<sile class="plain">…</sile>` as the whole document | works |
| `<sile>` with a `<document>` child | works |
| Arbitrary element `<em>` | **maps to the `\em` command** — the vocabulary mechanism ADR-002 assumed |
| Unknown element `<nosuchthing>` | **hard error, no PDF** |
| Namespace prefix `<bc:em>` | **fails** |

Two consequences for the emitter:

- **No namespace prefixes.** ADR-002 flagged this as a spike question with plain distinctive element names as the fallback; take the fallback. Nothing in the design depended on it.
- **Unknown elements fail loudly**, which is the behaviour a versioned contract wants: a class that does not know an element stops rather than dropping Scripture silently. This makes the `version` attribute enforceable — SILE-009 gets a real mechanism instead of a convention.

### F-13 — The escaping claim is not merely true, it is the difference between a corrupt build and a stopped one

Three runs, identical characters, differing only in input format.

**XML.** `Backslash \bd is not a command. Braces {literal}. Percent 100% off. Amp & lt < gt >.` rendered **exactly as written** — see [`escaping-xml-literal.png`](out/render/escaping-xml-literal.png). `\bd` is a genuine SILE command name and came out as text; `%` begins a comment in SIL and came out as a percent sign; `&`, `<`, `>` decoded from entities to literal characters.

**SIL, same characters.** `! Unknown command bd`, hard error, **no PDF**.

**SIL, with command names that exist** — the case that matters. Text reading `The word became flesh \par and dwelt among us, full of \skip[height=40pt] grace and truth.`:

> **0 errors reported. Build succeeded. 11,073-byte PDF.**
>
> And the verse is silently torn into three pieces with a 40-point gap driven through the middle of it — [`escaping-sil-injected.png`](out/render/escaping-sil-injected.png).

The same bytes through XML produce one unbroken line with `\par` and `\skip[height=40pt]` visible as literal text.

**Why this is stronger than the ADR argued.** ADR-002 rejected SIL templating on the grounds that safety would depend on a perfect escaping function. The spike shows the failure is worse than "an escape gets missed": a missed escape does not crash, it **succeeds**. There is no exception, no non-zero exit, no diagnostic — just Scripture silently reflowed by its own content. Every guarantee BibleCompose makes about not altering Scripture (BLD-004, FUN-002, NFR-007) would be intact at the source level and violated in the output, and nothing in the build would notice.

USFM contains backslashes by construction. This is not a hypothetical input.

---

## S0.7 — Figures

Artifacts: [`sil/figures.sil`](sil/figures.sil), [`out/figures.pdf`](out/figures.pdf), [`figures-p1.png`](out/render/figures-p1.png). Test artwork in [`assets/images/`](assets/images/).

**All three formats place correctly**, with no errors: PNG at a specified width, JPG at a specified width, and **PDF as genuine vector** — the artwork's text renders as crisp outlines, not pixels, and its own fonts arrive in the output as additional subsets. Scaling by height alone preserves aspect ratio.

This answers SRS §4.2's `creation-map.pdf` ([SRS-REVIEW F14a](../docs/SRS-REVIEW.md#f14--smaller-gaps-and-conflicts)): **vector artwork is supported and needs no conversion.** For maps and diagrams in print that is the difference between sharp and merely adequate, and P4.3 does not need a raster fallback path.

Two things to carry to P4.3:

- **An included PDF brings its whole page box, whitespace included.** The artwork's 3×2in page reserved 2.4×1.6in in the output, margins and all. BibleCompose cannot assume supplied PDFs are trimmed to their artwork, so figure sizing needs either a documented expectation or a bounding-box crop.
- **An included PDF brings its fonts.** Relevant to PDF-003 pre-flight and to output size; a project with many PDF figures accumulates font subsets BibleCompose never chose.

### F-14 — Images fail loudly, but SILE enforces no path containment

Unlike fonts, bad image sources stop the build: a missing file, a wrong-format file, and a non-image all produce an error and no PDF. Good.

But **location is not checked**. An absolute path to a valid image well outside the project embedded silently and produced a 16,515-byte PDF. SILE validates *format*, never *provenance*.

SRS §15 already requires relative asset references to resolve inside the project directory. The spike confirms the requirement cannot be delegated: the containment check is BibleCompose's, performed after canonicalization so that `..` and symlinks are both covered, and it is the only such check in the pipeline. P4.3.

---

## S0.8 — Write-up

### F-15 — The PDF is not byte-reproducible, and the reason is not timestamps

[SRS-REVIEW F4](../docs/SRS-REVIEW.md#f4--reproducible-is-two-different-claims-and-must-be-split) split determinism into a byte-reproducible backend input and a structurally-equivalent PDF, assuming the usual culprits. Measured, SILE is better than assumed and still not reproducible.

**What SILE already does right:** the document `/ID` is written as all zeros, and there is no `CreationDate` or `ModDate` anywhere in the output. Someone upstream cared.

**What still varies.** Four builds of identical input:

| run | bytes | sha256 | subset tags |
|---|---|---|---|
| 1 | 41,984 | `7a90abf7…` | `AYABNL`, `EPZXXP` |
| 2 | 41,985 | `221e5e81…` | `HQTCEM`, `SZXXQG` |
| 3 | 41,984 | `994ed084…` | `RJMIKL`, `WWGYZR` |
| 4 | 41,985 | `d7332606…` | `QTJNYA`, `TVLZQI` |

**The font subset tag is randomly generated per run.** The six-letter prefix on every subset font name changes each time, and the file size fluctuates by a byte as the tag compresses differently.

Two consequences, both narrow and both easy to discover the hard way:

- DET-002's embedded-font comparison must **strip the six-letter prefix**, or every run fails on a difference that carries no information.
- The reason belongs in the test as a comment. "PDFs contain timestamps" is the wrong explanation here and would send someone hunting for a `SOURCE_DATE_EPOCH` that does not exist.

### F-16 — The bundle can omit the networking rocks

F-2 flagged `luasec` and `luasocket` as an oddity in the dependency set, given NFR-004. Tested by moving both rocks and their C modules out of the tree and rebuilding the spike document: **the build succeeds unchanged.**

So P5.7 bundles **17 rocks, not 19**, and the shipped runtime contains no TLS and no socket code whatsoever. That turns NFR-004's "no internet connection required" from a behavioural claim into a structural one for the backend half of the pipeline — the code that could open a connection is not present. Cheaper to defend and easier to explain than a captured-traffic test, though P5.9 should still run one for the application half.

### What was amended

| Document | Change |
|---|---|
| [`sile/classes/biblecompose.lua`](../sile/classes/biblecompose.lua) | promoted from `spike/sil/`; both spike documents rebuild from the new location unchanged |
| [ADR-002](../docs/adr/002-sile-interface.md) | status Accepted; what the spike proved (F-13) and disproved (F-15); namespace and unknown-element behaviour; the class now exists |
| [SRS-REVIEW](../docs/SRS-REVIEW.md) | FONT-004 added; FONT-002 and DET-002 sharpened; F5 confirmed and extended; F14a resolved |
| [ARCHITECTURE §7](../docs/ARCHITECTURE.md#7-pre-flight) | "Fonts and scripts" widened to "Pre-flight" — fonts, hyphenation, geometry, assets; §8 gains the real determinism reason |
| [ROADMAP](../docs/ROADMAP.md) | S0 outcome recorded; F-8/F-9 to P0.4, F-15 to P0.5, F-10 to P4.1, F-14 to P4.3, FONT-004 to P5.2, F-2/F-16 to P5.7 |

### What S0 did not settle

Carried forward honestly rather than quietly closed:

- **F-8**, page 1 does not follow the frame chain. Three approaches tried. **P0.4.**
- **F-10**, notes overlap a long column, and numbering runs continuously through the document. **P4.1.**
- **Verse stranding at a column foot** was never stress-tested — the spike's text was too short to force the case. It is the one question from [SRS-REVIEW F2](../docs/SRS-REVIEW.md#f2--the-riskiest-unknown-is-scheduled-last) that remains genuinely open, and it belongs to the first milestone that sets a whole book. **P1.7.**

None of the three changes the verdict; all three would change a printed page.

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
| 3 | Can a running head carry the page's reference range? | **Yes**, across a column break, and in Tamil as well as Latin (F-9) |
| 4 | Uninstalled font, Tamil shaping? | **Yes** — `\font[filename=…]`, subset and embedded, correct conjuncts. But hyphenation is wrong and silent (F-11), and a font without coverage is a clean passing build (F-12) |
| 5 | How does XML input map to commands? | **Element name → command name.** No namespace prefixes; unknown elements are a hard error, which makes the version contract enforceable |
| 6 | Do `\`, `{`, `%` survive the XML path literally? | **Yes, exactly.** And the SIL control shows the alternative fails *silently and successfully* (F-13) |
| 7 | PDF artwork as a figure? | **Yes, as true vector.** No conversion path needed. Watch the page box and the inherited fonts |
| 8 | How much custom Lua? | **298 lines** for two-column mirrored masters, footnote frames, balancing, running heads with live ranges, and configurable geometry — replacing a 295-line upstream class that does less and does not run |

All eight answered. The one question that arrived late and is still open is verse stranding at a column foot, which needs a whole book to provoke — P1.7.

---

## Proposed requirement

For v0.2 of the SRS, in its format, alongside the nine in [SRS-REVIEW §4](../docs/SRS-REVIEW.md#4-requirements-the-srs-is-missing).

| ID | Requirement | Priority | Acceptance / Verification |
|---|---|---|---|
| FONT-004 | The application shall determine whether the backend has hyphenation patterns for the configured language, and where it does not, shall disable hyphenation rather than allow another language's patterns to be applied; if the project requested hyphenation, the reason it is inactive shall be reported. | MUST | A Tamil project with `hyphenation = true` produces output with no hyphens and one informational diagnostic, rather than Latin-style hyphens inserted into Tamil words. |

Two existing requirements also need their wording sharpened by what S0.5 and S0.6 found:

- **FONT-002** should say that coverage checking is the *only* defence, because DET-002's structural PDF assertions pass on a page of tofu (F-12).
- **DET-002** should say so too, so nobody later reads the structural assertions as sufficient.
