# S1 — Packaging spike, working notes

What a single distributable binary costs, and whether Windows is a wall. Companion to [NOTES.md](NOTES.md), which is S0's record.

The decision this measures is [ADR-006](../docs/adr/006-single-binary.md); the items are [S1.1–S1.5](../docs/ROADMAP.md#s1--packaging-spike).

| Item | Status |
|---|---|
| S1.1 Build from source on Linux | **Done** — builds in 68s (P-6); prerequisites in P-4 |
| S1.2 The same on Windows | Queued in CI ([packaging-spike.yml](../.github/workflows/packaging-spike.yml)) |
| S1.3 The same on macOS | Queued in CI — no macOS available locally |
| S1.4 A binary that re-executes itself | Not started — **larger than ADR-006 estimated** (P-6) |
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

## P-2 — SILE can already embed everything Lua. The release chooses not to.

This is the good news, and it changes the size of [S1.4](../docs/ROADMAP.md#s1--packaging-spike).

`configure` has the flag:

```
--enable-embedded-resources
        Compile resources such as Lua module files directly into the Rust CLI binary
```

and when it is on, `Makefile.am` switches what happens to the Lua tree:

```make
if !EMBEDDED_RESOURCES
nobase_dist_pkgdata_DATA    = $(SILEDATA) $(LUALIBRARIES)
nobase_nodist_pkgdata_DATA  = $(BUILT_SOURCES_LUA) $(LUAMODULES)
endif !EMBEDDED_RESOURCES
```

Off, those become installed data files. On, the same three variables — `SILEDATA`, `LUALIBRARIES`, **`LUAMODULES`** — are what `src/embed-includes.rs` is generated from. `LUAMODULES` is the vendored luarocks tree.

**So a build configured that way embeds the rocks, not just SILE's own Lua.**

### Which explains [F-2](NOTES.md), and it was not a flaw

S0 found the released binary failing on `module 'lua-utf8' not found` and concluded it "is not self-contained". Correct as observed, but the cause is a deliberate step in their release process:

```make
dist-hook-devendor-luarocks: dist-hook-decore-automake
        cd $(distdir)
        $(SED) -i -e '/^LUAMODULES/d;/^\tlua_modules/d' Makefile.in
```

The distribution tarball has luarocks **removed** from its Makefile, so anything built from a release expects the rocks to come from the system. A build from git does not have that done to it.

### What this means for ADR-006

Reading the build files, this looked like most of option B's work being already done upstream. **The build in P-6 disproved that.** The paragraph is kept because the mechanism above is real and correct as far as it goes — the Lua *is* embedded — but the conclusion drawn from it was wrong, and P-6 is the finding that matters.

---

## P-3 — Where each platform stands

**Linux — done.** WSL Ubuntu 22.04 builds it (P-6). The prerequisites were installed as P-4 records; `sudo` was not available for most of it, which turned out not to matter.

**Windows — not attempted locally.** No MSVC on `PATH`, no `cmake`, no `perl`, no MSYS2. Cargo finds a linker for pure-Rust crates, so the MSVC toolchain is installed somewhere, but none of what an autotools build needs is. Queued on the CI runner instead, which is both faster to try and closer to how P5.7 will really build it.

**macOS — not available here at all.** Only answerable on a runner.

Both remaining legs are in [packaging-spike.yml](../.github/workflows/packaging-spike.yml). The workflow predates P-6 and its Linux job still asserts self-containment, which will now fail; that assertion needs relaxing to "builds and reports a version", with the rock question tracked separately.

---

## P-4 — What a source build actually demands, discovered one refusal at a time

`configure` does not report everything it wants up front; it stops at the first
thing it cannot find. Each of these was a separate run. Recorded in order
because the *sequence* is the useful part — P5.7's CI needs all of them
installed before the first attempt, not after five.

| # | Refusal | Cause | Resolution |
|---|---|---|---|
| 1 | `jq is required` | not installed | install `jq` |
| 2 | `--enable-font-variations was given, but harfbuzz version not new enough` | see P-5 | `--disable-font-variations` |
| 3 | `cannot find suitable Lua interpreter` | SILE defaults to **LuaJIT**, which was absent — plain Lua 5.1 was present and passes configure's own probes, but is not in the list it searches | install `luajit` + `libluajit-5.1-dev` |
| 4 | `font family Gentium Plus not found` | see P-5 | `FCMATCH=true`, or install the font |

Everything here was installed **without root**, by `apt-get download` and
`dpkg-deb -x` into `$HOME` — both work unprivileged, since only dpkg's
*database* needs root and the files alone are enough. Worth knowing for locked-
down build machines, and it is how this spike proceeded at all.

Versions the build was given: HarfBuzz 2.7.4, fontconfig 2.13.1, ICU 70.1,
LuaJIT 2.1.0-beta3, gcc 11.4, autoconf 2.71, cargo 1.91.

## P-5 — Two build-time requirements that are not about Rust at all

Both are the sort of thing that turns a CI job red for a reason nobody expects,
so they are called out separately rather than buried in the table above.

### HarfBuzz decides whether variable fonts work

SILE's minimum is HarfBuzz 2.7.4. Ubuntu 22.04 ships **exactly** 2.7.4 — so it
builds, but `--enable-font-variations` (the default) additionally wants
`harfbuzz-subset >= 6.0.0` and fails.

The consequence is not a flag, it is a feature: **the build platform's HarfBuzz
version determines whether the shipped BibleCompose supports OpenType variable
fonts.** That is not academic for this product — Noto Serif Tamil, the face S0
used, is published as a variable font, and a publisher choosing an optical size
or a weight axis is doing something a Bible edition plausibly wants.

So P5.7 has a decision it did not know it had: build on a base new enough for
HarfBuzz 6+, or ship without variable-font support and say so. It also lands on
[FONT-002](../docs/SRS-REVIEW.md#4-requirements-the-srs-is-missing)'s
neighbourhood — a variable font requested on a build that cannot do variations
is another silent-substitution opportunity.

### The build requires a font to be installed

`QUE_FONT(Gentium Plus)` makes configure fail unless that family is resolvable
through `fc-match`. Gentium Plus is SILE's own default font, not ours.

`FCMATCH=true` skips the check, which is what this build does — BibleCompose
sets its own font on every document, so SILE's default is never reached. But it
is worth stating plainly that **a font is a build-time dependency of the
typesetter**, and that skipping the check means the resulting SILE has no
working default font. For us that is correct; for anyone invoking that SILE
directly it would be a trap.

## P-6 — SILE builds from source, and the result is still not self-contained. The reason is C modules.

**S1.1's build works.** From a clean checkout, with the prerequisites in P-4:

```
./bootstrap.sh
./configure --enable-embedded-resources --disable-font-variations FCMATCH=true
make -j16
```

`make exit 0 after 68s`. A 14 MB binary at `target/x86_64-unknown-linux-gnu/release/sile`, linking the seven static libraries P-1 predicted plus system HarfBuzz, fontconfig, ICU, zlib and libpng. The Rust link line confirms `--features luajit --features vendored --features static`, so the embedding path really did run.

**And it still fails exactly as the released binary did:**

```
Error: module 'lua-utf8' not found: no field package.preload['lua-utf8']
```

### Why — and it is not what P-2 hoped

The vendoring worked. `lua_modules/` holds 272 Lua files and 16 shared objects, and `src/embed-includes.rs` has 904 include lines, 502 of them from the rock tree. **All 16 `.so` files are in that list**, `lua-utf8.so` included. The bytes are in the binary.

They cannot be loaded. `inject_embedded_loaders` installs three searchers, and the one for native code hard-codes six names:

```rust
"fontmetrics" | "justenoughfontconfig" | "justenoughharfbuzz" |
"justenoughicu" | "justenoughlibtexpdf" | "svg"
    => lua.create_c_function(luaopen_…)
_   => format!("C Module '{module}' is not linked in Rust binary")
```

Those six are **SILE's own** C modules, statically linked and registered by hand. Everything else — `lua-utf8`, `lpeg`, `lfs`, `lxp`, `zlib`, `ssl`, `socket`, `bit32`, `linenoise`, `compat53` — gets that error string. The Lua searcher below it only serves `.lua` paths.

The underlying constraint is not a SILE limitation: **`dlopen` needs a real file.** A C extension embedded as a byte array cannot be loaded without first writing it to disk, or being statically linked and registered in `package.preload`. SILE does the latter for the six it owns, and embeds the other ten as dead weight.

### What this costs option B

[ADR-006](../docs/adr/006-single-binary.md) estimated option B's extra work over option C as "embedding the Lua tree and the rocks via `rust-embed`". The Lua half is free. **The C half is the whole job**, and it is bigger than the ADR assumed:

To reach one file with nothing extracted, BibleCompose's re-executed child must build those ten rocks as **static** libraries, link them into our binary, start the Lua VM itself, register every `luaopen_*` in `package.preload`, and only then hand off to SILE's `run()`. That is real work, on ten third-party C projects, on three platforms.

Two honest alternatives, both cheaper:

- **Extract the `.so` files on first run** and leave everything else embedded. Ten small files to a cache directory, not a whole SILE installation. This is option C's compromise applied narrowly — and it is *much* less objectionable than extracting an executable, because a shared library is not something antivirus treats as a program.
- **Upstream the registration.** SILE already has the mechanism for six modules; extending it to the vendored rocks is a contained change in the same file, and it would make `--enable-embedded-resources` deliver what its name promises for everyone. Worth raising regardless of what BibleCompose does.

Neither changes the **decision** in ADR-006 — the process boundary is still worth keeping and option A is still rejected for the reasons given. What changes is the estimate, and S1.5 should record the corrected one rather than the original.

---

## What this already tells P5.7

Three things, none of which depend on finishing the build:

1. **CI must run autotools on every platform.** Not `cargo build`. The GitHub workflow's `backend` job installs a prebuilt SILE today; for packaging it will need `./bootstrap.sh && ./configure && make`, and on Windows that means an MSYS2 runner.
2. **`--features static` is not a shortcut.** The embedding work in [S1.4](../docs/ROADMAP.md#s1--packaging-spike) is ours either way.
3. **The macOS leg needs a runner before it needs effort.** GitHub provides `macos-latest`; that is the cheapest way to answer S1.3 and it can be answered in CI before anyone builds it by hand.
