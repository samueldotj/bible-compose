# ADR-002 — Generate XML for SILE, not TeX-like `.sil`

**Status:** Accepted — confirmed by the S0 spike
**Relates to:** SRS §10 (SILE integration), §15 (security), SILE-005, SILE-009
**Evidence:** [spike/NOTES.md](../../spike/NOTES.md) F-13, and the renders in [spike/out/render/](../../spike/out/render/)

## Context

BibleCompose generates input for SILE and runs it. SRS §10.1 asks for *"a controlled intermediate SILE document/package rather than emitting SILE commands throughout the application"* — the right instinct — but calls the artefact a `.sil` file, and `.sil` names SILE's TeX-like syntax.

Three requirements bear on the format:

- **§15** — *"Generated SILE input must treat Scripture and configuration values as data. User-controlled content shall not be allowed to inject arbitrary Lua/SILE execution."*
- **SILE-005** — the generated input shall be deterministic for identical normalized input and resolved configuration, so golden-file tests stay stable.
- **BLD-008 / SILE-008** — the intermediate is retained for debugging when the user asks, so a human reads it occasionally.

SILE accepts three first-class input languages: SIL (the TeX-like syntax), XML, and Lua. It decides between the first two by looking at the first character: an angle bracket means XML. In XML input, elements are processed as command invocations, which is what makes a custom vocabulary possible.

## Options

### A — Emit TeX-like `.sil` by templating (rejected)

The obvious approach, and the one the SRS's wording implies.

It puts Scripture text into a syntax where `\`, `{`, `}`, and `%` are meaningful. Safety then depends on an escaping function applied at every site where user-controlled text is written — every verse, every footnote, every book name, every heading, and every string a user can type into a settings field, including a font name and an output filename. Miss one site, or get one character class wrong, and §15's guarantee is gone with no test that would notice.

The failure is also not hypothetical. USFM legitimately contains backslashes; a `\` surviving into `.sil` output is a live command, not a stray character.

Determinism is achievable but hand-maintained: whitespace and grouping have to be normalized by convention rather than by a serializer.

### B — Emit JSON, read it from a Lua class (considered)

Scripture becomes JSON string data, which is as safe as XML and equally deterministic. The Lua class reads the file and drives `SILE.typesetter` programmatically.

Rejected on two counts. It requires a JSON decoder inside the class, which is a dependency on what the bundled SILE happens to ship. And it abandons SILE's own document-processing path — every construct becomes bespoke Lua rather than a command with a definition, which is more code in the language with the weakest tooling in the stack.

### C — Emit XML consumed by a BibleCompose class (chosen)

## Decision

**BibleCompose emits an XML document whose vocabulary is defined by a versioned BibleCompose SILE class.**

```xml
<biblecompose version="1" class="biblecompose">
  <styles>…resolved style map, as data…</styles>
  <book code="MAT" name="Matthew">
    <para style="p">
      <chapter n="3"/><verse n="1"/>
      <text>In those days came </text>
      <char style="nd"><text>John</text></char>
      <note style="f" caller="+"><para style="ft"><text>Or …</text></para></note>
    </para>
  </book>
</biblecompose>
```

**Scripture is a text node.** A backslash is a backslash, a brace is a brace, and the only characters needing treatment are `<`, `>`, and `&`, handled by the serializer rather than by us. §15's guarantee becomes a property of the format instead of a property of our vigilance. This is the whole reason for the decision; everything below is a consequence.

**Determinism is the serializer's job** (SILE-005). Fixed attribute order, no insignificant whitespace, `\n` line endings on every platform, and no `HashMap` anywhere upstream — Rust randomizes its iteration order per process, and one `HashMap` on this path produces golden-test failures that reproduce on one machine in three.

**The resolved style map is emitted as data, not as commands.** A style is a set of typed values the class reads and applies; it never becomes a fragment of Lua or SIL. This keeps the second half of §15 — configuration values are as untrusted as Scripture, because a user types them.

**The `version` attribute is the compatibility contract** (SILE-009). The class refuses a version it does not know, with one sentence, rather than failing somewhere inside Lua with a stack trace. Class and application are versioned and shipped together.

**Invocation stays a child process.** SILE 0.15 is a Rust binary with an embedded Lua VM and does publish a crate, so linking it in is imaginable. It is the wrong trade: a child process gives a hard failure boundary when Lua errors (NFR-007) and it gives cancellation (BLD-006), which an in-process VM does not. Arguments go through the process API as an array; nothing is concatenated into a shell string.

That aside was later examined properly, because a single-file installation is wanted. The conclusion held and the reasoning is now recorded in [ADR-006](006-single-binary.md): the crate *is* linkable, and the two guarantees above are why it is not linked. A single binary is obtained instead by re-executing the application, which keeps the boundary this paragraph depends on.

**The class is where Bible typesetting lives.** Two-column frames, note placement, cross-reference placement, running heads carrying a verse range, chapter-opening treatment. The Rust side decides *what* and the class decides *how*, which is the boundary that lets layout improve without recompiling and lets Spike 0 explore it before any Rust exists.

That class now exists: [`sile/classes/biblecompose.lua`](../../sile/classes/biblecompose.lua), 298 lines out of S0. It is not a subclass of SILE's bundled `bible` class and deliberately so — that class typesets only when passed no options at all, and its two-column mode has never run ([spike/NOTES.md](../../spike/NOTES.md) F-5, F-6). Ours keeps upstream's architecture (`masters`, `twoside`, `infonode` + `chapterverse`, `footnotes`, `balanced-frames`) and none of its hardcoding.

## What the spike proved

S0.6 ran the identical characters through both formats. The result is stronger than the argument above, and it changes why this decision matters.

**XML.** `Backslash \bd is not a command. Braces {literal}. Percent 100% off. Amp & lt < gt >.` rendered exactly as written. `\bd` is a real SILE command name; it came out as text.

**SIL, same characters.** `! Unknown command bd` — hard failure, no PDF.

**SIL, using command names that exist.** Text reading `The word became flesh \par and dwelt among us, full of \skip[height=40pt] grace and truth.` reported **zero errors, exited zero, and produced a valid PDF** — with the verse silently torn into three pieces and a 40-point gap driven through the middle of it.

This ADR argued that templating `.sil` would make safety depend on a perfect escaping function. That understated it. **A missed escape does not fail — it succeeds.** No exception, no non-zero exit, no diagnostic; Scripture reflowed by its own content, in a build that every check would call good. BLD-004, FUN-002, and NFR-007 would all hold at the source and be violated in the output, and nothing in the pipeline would notice.

USFM contains backslashes by construction. This is not a hypothetical input, and no amount of care would have made Option A safe.

## What the spike disproved

**Nothing about the decision — but one assumption behind SILE-005 was wrong.** The ADR assumed the emitted input is the only thing needing determinism because the PDF is hopeless for the usual reasons (timestamps, document IDs). SILE is better than that: it zeroes `/ID` and writes no `CreationDate` at all.

The PDF is nonetheless not byte-reproducible. Four builds of identical input gave four different hashes, because **the font subset tag is randomly generated per run** — `AYABNL+DejaVuSerif`, then `HQTCEM+`, then `RJMIKL+`. File size fluctuates by a byte as the tag compresses differently.

Two consequences, both narrow and both easy to get wrong later:

- DET-002's structural assertions must compare font names **with the six-letter subset prefix stripped**, or every run fails on a difference that means nothing.
- The reason the PDF cannot be byte-compared is worth stating precisely in the test, because "timestamps" is the wrong explanation and would send someone hunting for a `SOURCE_DATE_EPOCH` that does not exist.

## Consequences

**The intermediate is more verbose than `.sil` and less pleasant to read by hand.** This is the real cost. It is paid by `keep_intermediates` producing something structured enough to diff meaningfully, which is what debugging a layout regression actually needs, and by the golden tests being diffable line by line.

**No namespace prefixes.** S0.6 measured it: `<em>` resolves to the `\em` command, but `<bc:em>` fails outright. The vocabulary therefore uses plain distinctive element names, which was the stated fallback. Nothing else in the design depended on it.

**Unknown elements are a hard error, and that is a gift.** `<nosuchthing>` stops the build rather than being skipped. It means the `version` attribute is enforceable by construction: a class that does not know an element cannot silently drop Scripture, so SILE-009 gets a real mechanism instead of a convention.

**Mapping SILE's errors back to source is harder from XML than it would be from a hand-built emitter that tracked its own line numbers.** SILE-007 requires backend failures to become understandable diagnostics. The answer is that the emitter records a map from output line to Scripture reference as it writes, so a SILE error at line 40,112 becomes "Matthew 3:1" before the user sees it. Cheap to build during emission, impossible to reconstruct afterwards, so it goes in from the first emitted element.

**If SILE is ever replaced, the XML is not the thing that carries over** — it is a SILE input format, not a neutral one. That is deliberate ([ADR-004](004-no-layout-crate.md)): the `Backend` trait is the portable boundary, and a second backend would emit its own format from the same model.

## References

[SILE input formats](https://sile-typesetter.org/manual/sile-0.15.12.pdf) · [SILE v0.15.0 release notes](https://sile-typesetter.org/blog/release-v0.15.0/) · [`sile` crate](https://docs.rs/crate/sile/latest) · [Freely-Given-org/BibleTypesetter](https://github.com/Freely-Given-org/BibleTypesetter) (prior art for the class)
