# S1 — Packaging spike, working notes

What a single distributable binary costs, and whether Windows is a wall. Companion to [NOTES.md](NOTES.md), which is S0's record.

The decision this measures is [ADR-006](../docs/adr/006-single-binary.md); the items are [S1.1–S1.5](../docs/ROADMAP.md#s1--packaging-spike).

| Item | Status |
|---|---|
| S1.1 Build from source on Linux | **Done** — builds in 68s (P-6); prerequisites in P-4 |
| S1.2 The same on Windows | **Substantially answered — P-9.** Cross-compiled from Linux with mingw-w64; `sile.exe` is a native PE that runs on Windows 11, loads every module, and typesets as far as line breaking. One blocker left, and it is a packaging gap, not a porting one |
| S1.3 The same on macOS | Queued in CI — no macOS available locally |
| S1.4 A binary that re-executes itself | Not built, but **costed** (P-6, P-7): a 2.1 MB tree must reach disk either way |
| S1.5 Measure; settle ADR-006 | **Linux measured: ~15 MB** (P-7). Waiting on S1.2/S1.3 to settle |

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

**Windows — not attempted locally.** The local inventory is better than this note first said: **MSVC 2022 Build Tools are installed** (`cl.exe` 14.44, `vcvars64.bat`), along with the Windows SDK, chocolatey and winget. What is absent is `cmake`, `perl`, MSYS2, and administrator rights. But the missing tools are not the real obstacle — see **P-8**, which is the answer to "why not just build it here": upstream does not have a Windows build to run. Queued on the CI runner, with the caveat P-8 raises about *which* build the runner should attempt.

**macOS — not available here at all.** Only answerable on a runner.

Both remaining legs are in [packaging-spike.yml](../.github/workflows/packaging-spike.yml), which now carries the prerequisites P-4 found and the flags P-5 and P-6 settled — so the runners should not have to rediscover them. Its Linux job detects whether the runner's HarfBuzz has the 6.0 subsetter and warns if not, and reports the self-containment question rather than asserting it.

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

## P-7 — What actually has to ship: 15 MB, and the embedding does not reduce it

P-6 left a hypothesis: extract only the ten native modules and let the embedded
Lua serve the rest. **Measured, and it is not enough.** But the measurement
gives the number S1.5 wanted, so the item is largely answered anyway.

### The matrix

`sile --version`, same binary, varying only what is on the module paths:

| what is on disk | result |
|---|---|
| nothing | `module 'lua-utf8' not found` |
| the 16 `.so` only | **works** |
| the full rock tree | works |

So the embedded Lua *is* being used at startup — the native modules were the
only gap, exactly as P-6 predicted.

**Typesetting is a different answer.** With the `.so` alone, or even the whole
`lua_modules/lib` on `LUA_CPATH`, a real document fails with
`attempt to concatenate a nil value` inside SILE's own module cache. Adding the
`.lua` files on `LUA_PATH` fixes it. Why the embedded copies serve `--version`
but not a build is SILE's business; the consequence is ours and it is simple:
**both halves have to be real files.**

### The artifact

Stripping luarocks' own bookkeeping — `lua_modules/lib` is 5.0 MB, of which
4.1 MB is `rocks-5.1/` metadata, HTML documentation and rockspecs that nothing
loads — the runtime that must accompany the binary is:

| | files | size |
|---|---|---|
| `.lua` modules | 132 | 1.2 MB |
| `.so` modules | 16 | 904 KB |
| **runtime tree** | **148** | **2.1 MB** |
| SILE binary | 1 | 14 MB |
| **total** | | **~15 MB** |

Verified: with exactly that tree and nothing else — no rock tree, no
`LUA_PATH` inherited, `env -i` — the binary typesets
[`tests/golden/john_1_1_5.xml`](../tests/golden/john_1_1_5.xml) to a correct
21,428-byte PDF.

15 MB is a comfortable number. It is smaller than the ADR's "tens of megabytes,
most of it ICU" guess, because ICU is dynamically linked here rather than
bundled — which is a Linux answer and may not survive S1.2 and S1.3.

### What it means for the options

**`--enable-embedded-resources` does not reduce what ships.** It embeds a
second copy of the Lua into the binary while the same files must still be on
disk for typesetting. For BibleCompose's purposes it is currently *worse* than
useless: 14 MB of binary carrying resources that do not remove the 2.1 MB tree.
A build without it would be smaller. Worth re-testing before P5.7 rather than
assuming — but on this evidence the flag is not the lever it appears to be.

**Option B and option C converge more than the ADR expected.** Both must put a
2.1 MB tree somewhere the process can read. B can embed it in our binary and
extract on first run; C extracts an executable plus the tree. The distinction
narrows to *whether an executable is among the extracted files* — which still
favours B, because that is the part antivirus objects to, but the gap is
smaller than "nothing is written to disk" implied. **ADR-006's option B
description overstates this and should be corrected at S1.5.**

The genuinely-nothing-on-disk variant remains possible — statically link the
ten C rocks and register them in `package.preload` — but P-6 already priced
that as the whole job, and P-7 adds that the Lua half would need solving too.

---

## P-8 — Windows is not a toolchain problem. Upstream has no Windows build of the Rust CLI.

> **Superseded in part by [P-9](#p-9--windows-works-cross-compiled-from-linux-running-natively-nine-walls-deep).**
> Everything below about upstream's state is accurate and still worth knowing.
> The conclusion drawn from it — that a Windows build is a porting project — was
> too pessimistic: cross-compiling from Linux works, and P-9 records the cost.
> Kept in place rather than rewritten, because the reasoning is what made P-9
> worth attempting.

Asked why S1.2 could not simply be done on the development machine. The shallow
answer is the missing tools in P-3. The real answer is worse, and it lands on
[NFR-001](../docs/SRS-v0.1.md), not on the schedule.

### The autotools build has no Windows support at all

`configure.ac` in v0.15.13 contains **zero** occurrences of `mingw`, `windows`,
`cygwin` or `win32`. The `./bootstrap.sh && ./configure && make` route that S1.1
used on Linux is not a route upstream has ever pointed at Windows.

### Windows has a *separate* build, and it is a different product

There is a `CMakeLists.txt` whose entire body is `if (WIN32)`. It does not
invoke cargo anywhere. What it produces is:

```cmake
configure_file(sile.in sile.lua)
...
COMMAND "<INSTALL_DIR>/bin/glue.exe" "<INSTALL_DIR>/bin/srlua.exe"
        "${CMAKE_CURRENT_BINARY_DIR}/sile.lua"
        "${CMAKE_CURRENT_BINARY_DIR}/sile.exe"
```

`srlua` glues a Lua interpreter to a script. So the Windows `sile.exe` is the
**pre-Rust Lua CLI**, not the 0.15 Rust binary S1.1 built. The six `justenough`
libraries are built as MSVC `SHARED` DLLs with `/EXPORT:luaopen_*` and loaded at
runtime — the opposite of the seven static `.a` files the Rust binary links
(P-1). Everything ADR-006 assumes about the Linux artifact — `--features
static`, embedded resources, one binary to re-execute — describes an object this
path does not produce.

The sources it needs are all still present (`justenough/*.c`, `sile.in`,
`silewin32.h`, `core/ classes/ languages/ packages/ lua-libraries/`), so it is
stale rather than gutted.

### It also builds its own supply chain, from forks

`ExternalProject_Add` clones and compiles, per build: expat R_2_2_6, **ICU from
`hunter-packages/icu` v63.1-p5 plus a local `icu.diff`**, harfbuzz 6.0,
freetype VER-2-12-1, libpng, zlib, fontconfig (`fontconfig.diff` is ~8,600
lines), Lua (`lua.diff`), and srlua (`srlua.diff`). Four vendored patches
against four upstreams, one of which is a third-party ICU fork pinned seven
major versions behind the ICU 70 the Linux build used.

`cmake_minimum_required(VERSION 3.0)` is a hard error under CMake ≥ 4.0 —
workable with `CMAKE_POLICY_VERSION_MINIMUM`, but a fair signal of last touch.

### Upstream says so plainly

`azure-pipelines.yml` — the pipeline the README's Windows badge points at:

```yaml
trigger: none
  # Disable Windows CI builds until somebody is actually working on fixing them
```

targeting `windows-2019` and `Visual Studio 16 2019`. And `README.md`:

> Nobody is currently maintaining Windows compatibility in SILE and we expect
> the state to be a bit broken.

The README's advice to Windows users is to use WSL.

### So: why not natively, here?

Not because the machine lacks MSVC — it has it. Because doing S1.2 natively
means **being the person who un-breaks SILE on Windows**: reviving a disabled
CMake build, against a forked ICU, to produce a Lua binary that is not the
artifact ADR-006 is about. That is a project, not a spike step, and it should be
decided as one rather than drifted into.

### Consequence — two, and the second is the expensive one

**1. The queued CI job is testing an unsupported route.**
[packaging-spike.yml](../.github/workflows/packaging-spike.yml)'s Windows job
uses MSYS2/MINGW64 with autotools and `mingw-w64-x86_64-rust`. That was a
reasonable guess, and it may yet work — mingw supplies every dependency as a
package, and autoconf handles host triples without being told about them. But it
is *our* route, not upstream's, and a red result would not distinguish "Windows
cannot build SILE" from "this route was never meant to". **The job should try
both**: MINGW64 autotools, and CMake/MSVC per the Azure pipeline. Either green
is an answer; both red is the finding.

**2. NFR-001's Tier-1 Windows claim rests on something unowned.** Even in the
best case — MINGW64 autotools works — we would be the only party building SILE
that way, and no upstream CI protects it. Every SILE upgrade becomes a Windows
risk we absorb. That belongs in the risk register in
[SRS-REVIEW.md](../docs/SRS-REVIEW.md), and the fallback worth pricing is
shipping the **Lua** SILE on Windows, since that is the one upstream at least
once supported.

**S1.5 cannot settle ADR-006 until this is resolved.** The ADR chooses between
ways of shipping one binary; on Windows there may not be one binary to ship.

---

## P-9 — Windows works. Cross-compiled from Linux, running natively, nine walls deep.

[P-8](#p-8--windows-is-not-a-toolchain-problem-upstream-has-no-windows-build-of-the-rust-cli)
concluded that a Windows build meant un-breaking SILE on Windows and should be
treated as a project. **That was too pessimistic, and this supersedes it.** The
route P-8 dismissed as "ours, not upstream's" turns out to work.

WSL cannot host a Windows *binary*, but it can host the Windows *build*. A
Fedora 41 container supplies a complete mingw-w64 cross stack; the resulting
`sile.exe` was copied to this Windows 11 machine and run there. Every result
below is from native execution, not wine and not WSL.

### The cross stack is better than the native Linux one

| | cross (Fedora `mingw64-*`) | Ubuntu native (S1.1) |
|---|---|---|
| HarfBuzz | **9.0.0** | 2.7.4 |
| ICU | 74.2 | 70.1 |
| fontconfig | 2.15.0 | 2.13.1 |
| freetype | 2.13.2 | — |

That matters beyond convenience: `configure` reported
`harfbuzz-subset >= 6.0.0... yes`, so the cross build gets font variations and
subsetting — the capability [P-5](#p-5--two-build-time-requirements-that-are-not-about-rust-at-all)
had to switch off on Linux.

**LuaJIT was the only gap** — there is no `mingw64-luajit`. LuaJIT cross-builds
in one command (`make CROSS=x86_64-w64-mingw32- TARGET_SYS=Windows`) and is
explicitly designed to; it yielded a PE `luajit.exe` and `lua51.dll`. With a
hand-written `luajit.pc`, every one of SILE's dependencies resolved.

### Nine failures, in order

The value is the sequence, as in [P-4](#p-4--what-a-source-build-actually-demands-discovered-one-refusal-at-a-time).
Nothing here was hard; there was just a lot of it, and none of it is documented
anywhere because nobody has walked this path.

| # | Failure | Cause | Fix |
|---|---|---|---|
| 1 | `cmp is required` | `diffutils` absent | install |
| 2 | `pdfinfo is required` | `poppler-utils` absent | install |
| 3 | `cannot find Lua module cassowary` | my `--with-system-luarocks` was wrong for cross | drop it |
| 4 | rockspec name mismatch | autoconf sets `program_prefix=${host_alias}-` when cross compiling; SILE feeds the transformed name into the rockspec, so `sile-dev-1.rockspec` declares `package = "x86_64-w64-mingw32-sile"` | `--program-prefix=` |
| 5 | `'rusile.dll' is not a standard library name` | `configure` **generates** `aminclude.am`, which is then newer than `Makefile.in`, so `make` re-runs automake — and automake 1.16.5 rejects the file configure just wrote (all three of `.so`, `.dylib`, `.dll`) | order timestamps so automake does not re-run |
| 6 | 12 `libtexpdf` objects fail | `libtexpdf.h` **has** a MinGW branch, but a stale one: it defines `ftello`→`ftello64`, and modern mingw-w64 declares `ftello64` itself with `_off64_t`, conflicting with the `_off_t` prototypes the macro rewrites | delete the branch (3 lines) |
| 7 | 5 of 6 `justenough` libraries fail | `silewin32.h` — SILE's *own* Windows compatibility header — implements `strcasestr()` with `tolower()` and never includes `<ctype.h>` | add the include (1 line) |
| 8 | `cannot stat .../librusile.dll` | `CARGO_TARGET_TRIPLE` defaults to `rustc -vV \| sed -n 's/host: //p'` — **the build machine** — with no reference to `--host`. So configure knew the host was Windows (it set `LIBEXT = .dll`) and then looked under the Linux target directory | it is `AC_ARG_VAR`: pass `CARGO_TARGET_TRIPLE=x86_64-pc-windows-gnu` |
| 9 | still `librusile.dll`, then `sile` | Windows DLLs take no `lib` prefix and executables end in `.exe`; the Makefile hardcodes Unix names | alias the files |

After 9, `make` exits 0 and produces `sile.exe`: **PE32+ x86-64, 2.0 MB**.

Total source patching: **four lines, in two files.**

### The one real problem: two Lua VMs

The first `sile.exe` ran on Windows and got as far as lpeg, then died with
`attempt to perform arithmetic on a boolean value`. The cause is structural:

```
sile.exe      imports: KERNEL32, msvcrt, ntdll      ← no Lua DLL
sile.exe      exports: 0 Lua symbols
lua-utf8.so   imports: lua51.dll
lpeg.so       imports: lua51.dll
```

`sile.exe` had LuaJIT statically linked inside it and exported none of it, while
every rock DLL bound to a *separate* `lua51.dll`. Two LuaJIT VMs in one process:
`luaopen_lpeg` registered its metatables into a VM that was not the one running
the code.

**This works on Linux only by an ELF accident** — a statically linked
executable's symbols are globally visible, so `dlopen`ed modules resolve against
the executable. PE has no equivalent; imports bind to a named DLL or nothing.
It is also why upstream's CMake path builds a `lua51.dll` and links everything
to it.

`configure` already has the switch: **`--with-system-lua-sources`**, which stops
mlua vendoring LuaJIT. With it, `sile.exe` shrank 2.0 MB → **1.2 MB**, gained
`lua51.dll` in its imports, and the VM split disappeared.

**This is the finding that generalises.** Anything statically linking Lua into
the executable and expecting `dlopen`ed C modules to find it is relying on ELF
behaviour that Windows does not have. It applies to [ADR-006](../docs/adr/006-single-binary.md)
option A directly, and it is the second independent reason to reject it.

### The C rocks cross-build too

[P-7](#p-7--what-actually-ships-is-15-mb-and-the-embedding-does-not-shrink-it)'s
16 native `.so` were the wall I expected to be worst. SILE's `Makefile-luarocks`
does drive luarocks with the native toolchain regardless of `--host` — the first
run produced 8 ELF objects in `lua_modules/lib64`. But luarocks takes the
toolchain as variables, and pointing it at the cross compiler works.

One wrinkle: luarocks emits `gcc -shared -L… -lluajit-5.1 -o out.so obj.o`, with
the library **before** the object. GNU ld resolves left to right, so it is
discarded before anything needs it. A three-line wrapper that appends the
library last fixes every rock at once.

| rock | result |
|---|---|
| luautf8, lpeg, luafilesystem, compat53 (×4), luaexpat, lua-zlib | **9 PE32+ DLLs** |
| bit32 | fails — needs POSIX `strerror_r`; **not needed**, configure skips it under LuaJIT |
| linenoise | fails — needs `termios.h`; it is the REPL line editor, not typesetting |

The `.so` extension does not need changing: SILE searches both `?.so` and `?.dll`
on its cpath, and `LoadLibrary` does not care what the file is called.

### How far it actually gets

Run natively on Windows 11, from the staged tree:

```
> .\sile.exe --version
SILE v0.15.13-dirty (LuaJIT 2.1.1785763465) [Rust]
```

and on a real document (`tests/golden/john_1_1_5.xml`, `--class biblecompose`)
it parses the XML, loads the class, resolves DejaVu Serif through fontconfig,
enters the HarfBuzz shaper, and stops in the ICU line breaker:

```
Word break parser failure: U_MISSING_RESOURCE_ERROR
```

### The last blocker is packaging, not porting

Fedora's `mingw64-icu` ships a **data-reduced** ICU. Counting break-iterator
resources in each data library:

| ICU data | `brkitr` entries |
|---|---|
| Ubuntu native (S1.1, works) | 32 |
| Fedora native | 78 |
| **Fedora `mingw64-icu` 74.2** | **0** |

So the cross ICU has no break-iterator data at all, and SILE cannot break lines.
Nothing about Windows is implicated — the library is simply packaged without it.
Three ways out, none attempted yet: MSYS2's `mingw-w64-x86_64-icu`, ICU's
published `icudt74l.dat` with `ICU_DATA` set, or building ICU data ourselves.
That is where S1.2 resumes.

### What this changes

1. **NFR-001's Tier-1 Windows claim is defensible again.** P-8 doubted it. On
   this evidence Windows is reachable, from Linux, in CI, with four lines of
   patch and a documented flag list.
2. **`--with-system-lua-sources` is mandatory on Windows**, not optional. It
   should be in the build recipe with the reason attached, because the failure
   it prevents (`arithmetic on a boolean`) points nowhere near its cause.
3. **[packaging-spike.yml](../.github/workflows/packaging-spike.yml)'s Windows
   job is the wrong shape.** It attempts a native MSYS2 build. A Linux cross job
   is faster, is now a known-good path, and matches how P5.7 would ship. Keep
   MSYS2 as a second data point; make cross the primary.
4. **P-8's conclusion is superseded** on the question of feasibility. What
   survives from it is accurate and still matters: upstream does not test any of
   this, so every SILE upgrade is a Windows risk we absorb, and that belongs in
   the risk register.
5. **The ICU data question is now on the critical path** for both Windows and
   [FONT-001..004](../docs/SRS-REVIEW.md) — a data-reduced ICU would break
   line breaking silently on exactly the complex scripts the pre-flight exists
   to protect.

---

## What this already tells P5.7

Three things, none of which depend on finishing the build:

1. **CI must build from source on every platform.** Not `cargo build`. The GitHub workflow's `backend` job installs a prebuilt SILE today; for packaging it will need `./bootstrap.sh && ./configure && make`. The Windows job should **cross-compile from Linux** rather than run MSYS2 natively — P-9 walked that route end to end and [s1-windows-cross.sh](s1-windows-cross.sh) reproduces it.
2. **`--features static` is not a shortcut.** The embedding work in [S1.4](../docs/ROADMAP.md#s1--packaging-spike) is ours either way.
3. **The macOS leg needs a runner before it needs effort.** GitHub provides `macos-latest`; that is the cheapest way to answer S1.3 and it can be answered in CI before anyone builds it by hand.
