# S1 — Packaging spike, working notes

What a single distributable binary costs, and whether Windows is a wall. Companion to [NOTES.md](NOTES.md), which is S0's record.

The decision this measures is [ADR-006](../docs/adr/006-single-binary.md); the items are [S1.1–S1.5](../docs/ROADMAP.md#s1--packaging-spike).

| Item | Status |
|---|---|
| S1.1 Build from source on Linux | **Question answered** (P-1), build not yet run — needs one privileged install |
| S1.2 The same on Windows | Not started — needs MSYS2 |
| S1.3 The same on macOS | Not started — no macOS available here |
| S1.4 A binary that re-executes itself | Blocked on S1.1 |
| S1.5 Measure; settle ADR-006 | Blocked |

---

## P-1 — `cargo build` alone cannot produce a working SILE. Confirmed.

[ADR-006](../docs/adr/006-single-binary.md) inferred this from the build files and flagged it as the first thing S1.1 should settle. It is now established, from the v0.15.13 source and the published crate, on three independent grounds.

### The single-binary feature refers to a file that does not exist

`--features static` is the one that embeds SILE's Lua resources — precisely what a single binary needs. But:

| | git checkout | published crate |
|---|---|---|
| `src/embed.rs` | **absent** | **absent** |
| `src/embed.rs.in` | present | present |
| `src/embed-includes.rs` | absent | absent |

and `src/lib.rs` contains `pub mod embed;`. The real file is produced by `make`:

```make
src/embed-includes.rs: Makefile-distfiles
        ... > $@
src/embed.rs: src/embed.rs.in src/embed-includes.rs
        $(SED) -e '/@EMBEDDED_INCLUDE_LIST@/r $(word 2,$^)' ... $< > $@
```

`embed-includes.rs` is itself generated, from a distfile listing, into one `#[include = "…"]` attribute per resource. So the list of what gets embedded is decided by the build system, not by the source.

**This is why docs.rs succeeds and a real build would not.** docs.rs builds with `luajit, vendored` — `static` is absent, `mod embed` is `#[cfg(feature = "static")]`, and the missing file is never compiled. A crate that documents cleanly is not evidence that it builds usefully.

### The binary links seven static libraries that only `make` builds

From `Makefile.am`:

```make
$(CARGO_BIN): justenough/.libs/fontmetrics.a
$(CARGO_BIN): justenough/.libs/justenoughfontconfig.a
$(CARGO_BIN): justenough/.libs/justenoughharfbuzz.a
$(CARGO_BIN): justenough/.libs/justenoughicu.a
$(CARGO_BIN): justenough/.libs/justenoughlibtexpdf.a
$(CARGO_BIN): justenough/.libs/svg.a
$(CARGO_BIN): libtexpdf/.libs/libtexpdf.a
```

These are the shaping, font, and PDF layers — the parts that do the actual typesetting. They come from 14 C, Objective-C and C++ sources in `justenough/` plus the `libtexpdf` submodule (140 files). Cargo knows about none of it: `harfbuzz-sys` is an *optional* dependency and is not in the default feature set.

The crate does ship the sources — `justenough/` and `libtexpdf/` are both in the published tarball, along with SILE's `classes/` (15) and `packages/` (94). What it does not ship is anything that compiles them.

### Consequence

**Nothing changes in [ADR-006](../docs/adr/006-single-binary.md)'s decision**, and the paragraph that hedged this can now state it. Both live options — B, re-executing the binary, and C, embedding the executable — need SILE built from source by its own build system first. The choice between them is unaffected; what is affected is P5.7's shape, which must run autotools in CI on every platform rather than adding a dependency to `Cargo.toml`.

It also means **`--features static` is not usable as shipped** for option B. Embedding the Lua tree will either mean running SILE's own `make` to generate `embed.rs`, or doing our own `rust-embed` over the installed data directory. The second is more work but is under our control and does not depend on an upstream build system we do not otherwise use. That is an S1.4 decision.

---

## P-2 — Neither machine here can complete a source build yet

Recorded so the gap is visible rather than implied. Everything above came from reading the source; the build itself has not been run.

**WSL Ubuntu 22.04** — has `gcc`, `g++`, `make`, `pkg-config`, and the runtime libraries S0 proved present. Missing:

| | |
|---|---|
| Build system | `autoconf`, `automake`, `libtool` |
| Rust | no `cargo`, no `rustc` |
| Headers | harfbuzz, fontconfig, ICU, freetype, libpng, lua5.1 |

All of it is one `apt-get` away, but `sudo` is not passwordless here, so it needs the user.

**Windows** — no MSVC on `PATH`, no `cmake`, no `perl`, no MSYS2. Cargo finds a linker for pure-Rust crates, so the MSVC toolchain is installed somewhere, but nothing else of what an autotools build needs is present. S1.2 needs MSYS2 and its toolchain.

**macOS** — not available. S1.3 cannot be attempted from here at all and needs either a machine or a CI runner.

---

## What this already tells P5.7

Three things, none of which depend on finishing the build:

1. **CI must run autotools on every platform.** Not `cargo build`. The GitHub workflow's `backend` job installs a prebuilt SILE today; for packaging it will need `./bootstrap.sh && ./configure && make`, and on Windows that means an MSYS2 runner.
2. **`--features static` is not a shortcut.** The embedding work in [S1.4](../docs/ROADMAP.md#s1--packaging-spike) is ours either way.
3. **The macOS leg needs a runner before it needs effort.** GitHub provides `macos-latest`; that is the cheapest way to answer S1.3 and it can be answered in CI before anyone builds it by hand.
