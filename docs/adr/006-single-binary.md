# ADR-006 — Ship one binary that carries SILE inside it and extracts it once

**Status:** **Accepted** — measured on Linux and Windows by [S1](../ROADMAP.md#s1--packaging-spike). **The chosen option changed from B to C**; see [Decision](#decision). The title changed with it — the original was "…by re-executing it, not by linking SILE in-process", and the second half of that still holds
**Supersedes:** the "not planned" line against in-process SILE in [ROADMAP Part 6](../ROADMAP.md#part-6--deliberately-excluded)
**Relates to:** SILE-003, SILE-004, SILE-009, BLD-006, NFR-001, NFR-007

## Context

The product should install as a single file. [ADR-002](002-sile-interface.md) settled that SILE is invoked as a child process and recorded in-process embedding as "imaginable later"; the question has now been asked directly, so it is worth answering with evidence rather than with that aside.

**Two things are being asked for and they are separable.** One is a single distributable artifact. The other is calling SILE through a function rather than a process. The first is wanted; the second is a means, and it is not the only one.

### What the spike established

- **There is no prebuilt SILE for Windows or macOS.** Release v0.15.13 publishes one binary asset, for Linux ([spike F-1](../../spike/NOTES.md)).
- **The Linux binary is not self-contained.** It embeds LuaJIT and SILE's own Lua but none of its third-party Lua dependencies; the real list is 19 rocks, of which 17 are actually needed ([spike F-2, F-16](../../spike/NOTES.md)).

### What `sile` the crate actually offers

Read from `v0.15.13`:

- **It is a library as well as a binary.** `src/lib.rs` exposes `start_luavm()`, `load_sile()`, `run()`, `inject_paths()`, `version()`, and the binary is gated behind `[[bin]] required-features = ["cli"]`. So linking it is possible in principle.
- **The features aimed at this already exist:** `static = ["rust-embed"]` embeds Lua resources, `vendored = ["mlua/vendored"]` builds the VM from source, and `luajit`/`lua54` choose the interpreter.
- **But `cargo add sile` will not produce a working typesetter.** The canonical build is autotools driving cargo — `./bootstrap.sh && ./configure && make` — the build script keys off `AUTOTOOLS_DEPENDENCIES`, and `harfbuzz-sys` is an *optional* dependency. The shaping and PDF C code is built by `make` and linked in rather than by cargo.

**S1.1 has since confirmed that**, and it is no longer inference ([spike/S1-NOTES.md P-1](../../spike/S1-NOTES.md)). Two independent proofs: `src/embed.rs` — the file `--features static` needs — exists in neither the git checkout nor the published crate, only as an `.in` template that `make` fills in; and the binary links seven static libraries (`justenoughharfbuzz.a`, `justenoughicu.a`, `libtexpdf.a` and four more) built from C sources cargo never sees. docs.rs builds the crate cleanly because it omits `static`, which is a good reminder that documenting is not building.

### The cost that does not move

Whatever calls SILE, the artifact must contain **four C libraries** — HarfBuzz ≥ 2.7.4, fontconfig, ICU, libtexpdf — **and the Lua rock tree**. That work is identical under every option below, and it is already [P5.7](../ROADMAP.md#phase-5--m5--hardening). The choice here changes almost nothing about the hard part.

## Options

### A — Link SILE into the application process (rejected)

`biblecompose` depends on the `sile` crate, starts a Lua VM in-process, and typesets by calling a function.

Rejected, and not because it is difficult. It costs two things the specification names, and buys nothing option B does not:

- **BLD-006 cancellation.** There is no safe way to stop a Lua VM mid-typeset from another thread. A full-Bible build runs for minutes, and Cancel would degrade from "the process is gone in under a second" to "wait it out". The spike's own experience is relevant: a `supereject` in the wrong place made SILE spin forever ([spike/NOTES.md](../../spike/NOTES.md)) — with a child process that is a timeout and a diagnostic, and in-process it is a hung application.
- **NFR-007 crash isolation.** A segmentation fault in HarfBuzz on a malformed font would take the GUI down with the user's unsaved settings and style edits. Today it is a non-zero exit and a diagnostic.

There is a third hazard worth naming even though it is unquantified: SILE keeps a great deal of state in Lua globals (`SILE.scratch`, `SILE.settings`), and nothing establishes that two builds in one process are independent. That is a reentrancy question nobody has had to answer, and inheriting it to save a `fork` is a poor trade.

**S1 added a fourth, and it is the one that would bite hardest on Windows** ([S1-NOTES P-10](../../spike/S1-NOTES.md)). Linking Lua into an executable and letting dynamically loaded C modules find it works on Linux only because ELF makes a static executable's symbols globally visible to `dlopen`. PE has no equivalent: imports bind to a named DLL or they do not resolve. The spike hit this exactly — `sile.exe` with LuaJIT linked in exported nothing, every rock DLL bound to a separate `lua51.dll`, and lpeg's metatables landed in a VM that was not the one running the code. It surfaced four call frames away as `attempt to perform arithmetic on a boolean value`. Any option that puts Lua inside our executable inherits that problem and must answer it by statically linking and hand-registering every `luaopen_*`.

### B — One binary that re-executes itself (was chosen; now rejected)

The busybox pattern. A single executable; when BibleCompose needs to typeset, it spawns **itself** with a reserved argument, and that child calls SILE's `run()` in-process. The parent keeps the process boundary; the user gets one file.

Cancellation and crash isolation are unaffected — there is still a child to kill and still a boundary for a segfault to stop at. That part was right. **Two of the three claims made for it were not.**

- ~~Nothing is written to disk to make it work.~~ **False.** [S1-NOTES P-7](../../spike/S1-NOTES.md) measured it: even with SILE's Lua embedded in the binary, a 2.1 MB tree of `.lua` and `.so` must be real files on a real path, or typesetting dies inside SILE's own module cache. The embedding does not remove the extraction; it just adds a second copy.
- ~~The extra work is embedding the Lua tree and injecting loaders.~~ **Understated.** On Windows the two-VM problem above means option B must additionally statically link LuaJIT, six `justenough` libraries and nine C rocks into our binary and hand-register every `luaopen_*` — permanently, against SILE's internals, revisited at every upgrade.

### C — Embed SILE and extract it on first run (chosen)

Ship the SILE bundle as an embedded resource; extract it to a cache directory the first time it is needed and invoke it as today.

The least work of the three, it still gives one file to distribute, and it is the only option S1 has actually demonstrated end to end on two platforms.

Its cost is a first run that writes executable files to disk, which Windows security software may react to, and locked-down environments where the cache directory is not executable.

## Decision

**Ship a single binary that carries SILE as an embedded bundle and extracts it once (option C), keeping the process boundary. Do not link SILE into the application process.**

This reverses the original choice of option B. The evidence changed, in three steps:

1. **B's distinguishing claim was false** ([P-7](../../spike/S1-NOTES.md)). "Nothing on disk" is what made B worth extra work; a 2.1 MB tree must be extracted either way. B and C differ only in *what else* is in the extracted set.
2. **The remaining distinction was weaker than argued** ([P-10](../../spike/S1-NOTES.md)). The case for B was that C additionally extracts an *executable*, the thing antivirus objects to. But the measured Windows bundle is 30 DLLs — executable code by any definition — which B must extract too. Adding `sile.exe` to a directory that already holds 30 DLLs is not a category change.
3. **B costs materially more on Windows than anywhere else** (P-10, above). Static-linking the entire C layer to dodge PE's symbol rules is a real, recurring engineering burden, and it buys a distinction step 2 just dissolved.

Meanwhile option C **has been demonstrated**: the S1 bundle typesets `john_1_1_5.xml` natively on Windows to a correct 6×9in PDF whose text is identical to the Linux build's.

**Nothing in the current design changes.** The [`Backend`](../../crates/biblecompose-sile/src/lib.rs) trait already abstracts this: an `EmbeddedSileBackend` is a second implementation of a trait that exists, reached the same way, and [ADR-004](004-no-layout-crate.md) argued for exactly that boundary. This is a **packaging decision, not an architectural one**; M0 and M1 are unaffected, and the original ADR anticipated this reversal in its own last consequence — *"the two live options differ in packaging mechanics and in nothing else."* That is why flipping it is cheap.

**Option B is not dead, it is deferred.** If bundle size or antivirus behaviour ever makes extraction unacceptable, the path back is the "genuinely nothing on disk" variant: statically link the C modules and register them through `package.preload`. It is a larger job than the original ADR implied, and it should be taken on evidence of a real problem rather than in anticipation of one.

## Consequences

**Windows was expected to be the hard part. It was, and it is now solved.** Not under MSYS2, as this ADR assumed, but by cross-compiling from Linux with mingw-w64 ([P-9](../../spike/S1-NOTES.md), [P-10](../../spike/S1-NOTES.md), reproduced by [s1-windows-cross.sh](../../spike/s1-windows-cross.sh)). Four lines of patch to SILE, a documented flag list, and ICU taken from MSYS2's package rather than Fedora's. **NFR-001's Tier-1 Windows claim stands.**

What does not stand is any assumption that upstream protects it: SILE's own Windows CI is disabled and its README disclaims Windows support ([P-8](../../spike/S1-NOTES.md)). Every SILE upgrade is a Windows risk we absorb, and that belongs in the risk register rather than here.

**The advanced executable override stays** (SILE-004). A developer testing a newer backend should not have to rebuild the bundle, so `BIBLECOMPOSE_SILE` continues to select an external SILE and `SileBackend` continues to exist alongside the embedded one.

**Version reporting gets simpler and stricter.** SILE-002 wants the backend version in every build log; with the backend inside the artifact, that version is a build-time constant rather than something discovered at runtime, and a mismatch between the application and its SILE class (SILE-009) becomes impossible rather than merely diagnosed.

**Two runtimes stop being possible in one installation**, which removes a support question — "which SILE is it actually using" — that the current arrangement invites.

**The artifact grows, and by more than "tens of megabytes" on Windows.** Measured, not guessed:

| | Linux | Windows |
|---|---|---|
| binary | 14 MB | 1.2 MB |
| runtime tree | 2.1 MB | 77 MB |
| **total** | **~15 MB** | **~78 MB** |
| ICU within that | *system, not shipped* | **32 MB** |

The gap is almost entirely ICU. Linux linked the system copy and never counted it; Windows has no system ICU, so the full 32 MB data library ships. **ICU data filtering is therefore the highest-value size lever available** — keeping break iterators and the locales we support while dropping converters, collation and timezone data should recover most of it. That work has a prerequisite the SRS owns rather than the build: *which scripts does BibleCompose commit to supporting?* Today the answer is inherited from whichever distribution's ICU we happen to link, which is not a decision anyone made.

**A silent-failure hazard comes with that lever.** ICU break data splits into rules and dictionaries, and the dictionaries are what break Thai, Khmer, Lao, Burmese and CJK — scripts with no spaces. Fedora's cross ICU shipped neither, and the failure was loud only because *all* break data was missing. Drop the dictionaries alone and Latin text is perfect while Thai becomes one unbreakable overflowing line, with no error. The [FONT-001..004](../SRS-REVIEW.md) pre-flight should assert that ICU has break data for the document's language, not only that a font covers its glyphs.

**Cross-platform byte-identical output needs the font subset tag fixed.** The Linux and Windows PDFs have identical text and zeroed `/ID`, but differ in 20,075 of 21,571 bytes — a random six-byte subset tag inside a Flate-compressed font stream ([S0 F-15](../../spike/NOTES.md)). Known, and handled in the testkit for comparison purposes; [DET-001/002](../SRS-REVIEW.md) now need it handled for real.

## References

[`sile` crate](https://docs.rs/sile/latest/sile/) · [SILE v0.15.13 `Cargo.toml`](https://github.com/sile-typesetter/sile/blob/v0.15.13/Cargo.toml) · [SILE README build instructions](https://github.com/sile-typesetter/sile/blob/v0.15.13/README.md) · [spike/NOTES.md F-1, F-2, F-16](../../spike/NOTES.md) · [ADR-002](002-sile-interface.md) · [ADR-004](004-no-layout-crate.md)
