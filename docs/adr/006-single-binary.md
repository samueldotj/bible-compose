# ADR-006 — Ship one binary by re-executing it, not by linking SILE in-process

**Status:** Proposed — the cost is not yet measured, and [S1](../ROADMAP.md#s1--packaging-spike) measures it
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

That last point is inference from the build files and the README rather than something tried, and confirming it is [S1.1](../ROADMAP.md#s1--packaging-spike)'s first job.

### The cost that does not move

Whatever calls SILE, the artifact must contain **four C libraries** — HarfBuzz ≥ 2.7.4, fontconfig, ICU, libtexpdf — **and the Lua rock tree**. That work is identical under every option below, and it is already [P5.7](../ROADMAP.md#phase-5--m5--hardening). The choice here changes almost nothing about the hard part.

## Options

### A — Link SILE into the application process (rejected)

`biblecompose` depends on the `sile` crate, starts a Lua VM in-process, and typesets by calling a function.

Rejected, and not because it is difficult. It costs two things the specification names, and buys nothing option B does not:

- **BLD-006 cancellation.** There is no safe way to stop a Lua VM mid-typeset from another thread. A full-Bible build runs for minutes, and Cancel would degrade from "the process is gone in under a second" to "wait it out". The spike's own experience is relevant: a `supereject` in the wrong place made SILE spin forever ([spike/NOTES.md](../../spike/NOTES.md)) — with a child process that is a timeout and a diagnostic, and in-process it is a hung application.
- **NFR-007 crash isolation.** A segmentation fault in HarfBuzz on a malformed font would take the GUI down with the user's unsaved settings and style edits. Today it is a non-zero exit and a diagnostic.

There is a third hazard worth naming even though it is unquantified: SILE keeps a great deal of state in Lua globals (`SILE.scratch`, `SILE.settings`), and nothing establishes that two builds in one process are independent. That is a reentrancy question nobody has had to answer, and inheriting it to save a `fork` is a poor trade.

### B — One binary that re-executes itself (chosen)

The busybox pattern. A single executable; when BibleCompose needs to typeset, it spawns **itself** with a reserved argument, and that child calls SILE's `run()` in-process. The parent keeps the process boundary; the user gets one file.

- Cancellation and crash isolation are exactly what they are today, because there is still a child process to kill and still a boundary for a segfault to stop at.
- Nothing is written to disk to make it work.
- The extra work over option C is embedding the Lua tree and the rocks via `rust-embed` and injecting loaders through `inject_paths`.

### C — Embed the executable and extract it on first run (fallback)

Ship the `sile` binary as an embedded resource; extract it to a cache directory the first time it is needed and invoke it as today.

The least work of the three and it still gives one file to distribute. Two costs: a first run that writes an executable to disk, which Windows security software reacts badly to; and locked-down environments where the cache directory is not executable.

Kept as the fallback rather than rejected, because it is the option that survives if S1 finds that building SILE from source on Windows is impractical.

## Decision

**Ship a single binary by re-executing it (option B), keeping the process boundary. Do not link SILE into the application process.**

**Nothing in the current design changes.** The [`Backend`](../../crates/biblecompose-sile/src/lib.rs) trait already abstracts this: an `EmbeddedSileBackend` is a second implementation of a trait that exists, reached the same way, and [ADR-004](004-no-layout-crate.md) argued for exactly that boundary. This is a **packaging decision, not an architectural one**, and M0 and M1 are unaffected.

The decision is Proposed rather than Accepted because the number that matters — what it costs to build SILE from source on three platforms — has not been measured. [S1](../ROADMAP.md#s1--packaging-spike) measures it, before P5.7 needs the answer.

## Consequences

**Windows is the hard part, and it is the hard part under every option.** HarfBuzz, fontconfig, ICU and libtexpdf under autotools means MSYS2, and there is no upstream binary to fall back on. If S1 finds that wall is real, it does not change this ADR — it changes NFR-001's claim that Windows is a Tier-1 target, which is exactly the kind of thing worth learning at M1 rather than at M5.

**The advanced executable override stays** (SILE-004). A developer testing a newer backend should not have to rebuild the bundle, so `BIBLECOMPOSE_SILE` continues to select an external SILE and `SileBackend` continues to exist alongside the embedded one.

**Version reporting gets simpler and stricter.** SILE-002 wants the backend version in every build log; with the backend inside the artifact, that version is a build-time constant rather than something discovered at runtime, and a mismatch between the application and its SILE class (SILE-009) becomes impossible rather than merely diagnosed.

**Two runtimes stop being possible in one installation**, which removes a support question — "which SILE is it actually using" — that the current arrangement invites.

**The artifact grows** to tens of megabytes, most of it ICU. Not a concern for a desktop publishing application that will also carry fonts.

**If option C is taken instead**, the only thing that changes is where the child comes from. The `Backend` implementation, the cancellation path, the log capture, and every test above them are identical. That is worth stating plainly: the two live options differ in packaging mechanics and in nothing else.

## References

[`sile` crate](https://docs.rs/sile/latest/sile/) · [SILE v0.15.13 `Cargo.toml`](https://github.com/sile-typesetter/sile/blob/v0.15.13/Cargo.toml) · [SILE README build instructions](https://github.com/sile-typesetter/sile/blob/v0.15.13/README.md) · [spike/NOTES.md F-1, F-2, F-16](../../spike/NOTES.md) · [ADR-002](002-sile-interface.md) · [ADR-004](004-no-layout-crate.md)
