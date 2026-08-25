# Releasing

How a version of BibleCompose becomes three installers, and what a person has
to supply that a repository cannot.

Related: [ADR-006](adr/006-single-binary.md) · [ROADMAP](ROADMAP.md) ·
[spike/S1-NOTES](../spike/S1-NOTES.md)

---

## What ships

**One executable per platform**, with the typesetter inside it. SILE,
HarfBuzz, fontconfig, ICU, libtexpdf and the Lua rock tree are unpacked to a
cache directory on first run and re-executed as a child process, which is
[ADR-006](adr/006-single-binary.md) option C. The process boundary is kept, so
cancellation still kills a process tree and a backend crash is still isolated.

| | |
|---|---|
| Windows | `.msi` and `.exe` (NSIS), 29 MB and 19 MB |
| macOS | `.dmg` and `.app` |
| Linux | `.deb` and `.AppImage` |

The `.deb` declares no dependencies, and that is deliberate: the runtime
travels inside the executable, so a package that named SILE would be naming
something it does not use.

## The one thing to check

**`.github/workflows/release.yml` ends every job by building a PDF**, not by
compiling successfully. A binary that links and cannot set a page is exactly
what a packaging mistake produces — a shared library that did not get copied, a
Lua module the stage missed, a class shadowed by a stale one — and none of
those fail a build.

The smoke test runs with `BIBLECOMPOSE_SILE`, `BIBLECOMPOSE_SILE_PATH` and
`BIBLECOMPOSE_SILE_BUNDLE` removed from the environment, so what produces the
PDF is the artefact and nothing else on the machine.

## Staging

`scripts/stage-backend.mjs` turns a built runtime into what gets embedded. It
is one script for three platforms because the thing that has to be identical
across three build recipes is what gets *removed*:

* **`luasec` and `luasocket`.** They arrive with SILE's standard rock set and
  nothing here opens a connection ([spike F-16](../spike/NOTES.md)). NFR-004
  says a build needs no network; the strongest form of that is a runtime with
  no socket code in it, which is a claim a person can check with `ls`. A bundle
  that has them anyway is **refused at start-up** rather than warned about.
* **The fontconfig cache.** 2.9 MB of the build machine's font list.
* And it copies this repository's own class *in*, over whatever the runtime
  brought — SILE resolves its class from its own tree, and a stale one shadowed
  the repository's for six weeks of this project's history.

### Three ways the Lua dependency hides

All three platforms failed their first real run, and all three for the same
reason wearing a different hat. Worth writing down, because the messages name
neither the variable nor the cause.

| | |
|---|---|
| macOS | `configure: error: cannot find Lua includes`. Homebrew's `luajit` is keg-only, so it is off the default pkg-config path. Sounds like a missing package; is a missing path. |
| Windows | `cannot find LuaJIT using pkg-config: pkg-config has not been configured to support cross-compilation`. The Rust half asks through the `pkg-config` crate, which refuses to answer while cross-compiling unless `PKG_CONFIG_ALLOW_CROSS=1`. The C half was already configured and says nothing. |
| Linux | Built cleanly and failed at staging: `make install` lays out a prefix tree — `bin/sile`, `share/sile/` — where the Windows cross-build lands flat. `stage-backend.mjs` flattens the first into the second. |

### What the Linux smoke test cannot see

The executable carries SILE. It does **not** carry the system libraries SILE
linked against — HarfBuzz, fontconfig, ICU — and the runner that builds is the
runner that runs, so the smoke test passes whether or not a machine without
them would.

So "a fresh machine with no SILE installed builds a PDF" is proved on Windows,
where the DLLs are cross-built and travel in the bundle, and is **not yet
proved on Linux**. Answering it properly means either static linking or
carrying the shared objects, and either is a decision rather than a patch.

## Signing

**No certificate, key, password or thumbprint appears in this repository, and
none should.** Signing is driven by repository secrets; if they are absent the
build still runs and produces **unsigned** artefacts, and the job logs a
warning saying so. An unsigned build that works is more useful than a failed
one, and an unsigned release that everybody believed was signed is the failure
that warning exists to prevent.

### What has to be supplied

| Secret | For | Where it comes from |
|---|---|---|
| `APPLE_CERTIFICATE` | macOS | A base64 `.p12` of a Developer ID Application certificate |
| `APPLE_CERTIFICATE_PASSWORD` | macOS | The password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | macOS | `Developer ID Application: NAME (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | macOS | Notarization; an **app-specific** password, not the account one |
| `WINDOWS_CERT_THUMBPRINT` | Windows | The SHA-1 thumbprint of a certificate **already in the runner's store** |

Both certificates are **purchased and issued by a third party**, and the lead
time is days rather than minutes — which is why P6.1 sits where it does in the
roadmap.

Windows takes a thumbprint rather than the certificate itself, so no private
key is written to a runner's disk — which also means the certificate has to be
installed on the runner already. A hosted runner has no way to do that safely;
this is what a **self-hosted Windows runner** or a cloud signing service is
for, and it is a decision about where the key lives rather than a line of YAML.

macOS needs the `.p12` because `codesign` reads a keychain. It is imported into
a temporary keychain, unlocked for the job, and thrown away with the runner.

### Three things about the wiring that are easy to get wrong

Each of these was got wrong first, and each fails **silently** — an unsigned
release that every log calls successful.

* **Windows signing is configuration, not an environment variable.** The
  bundler reads `bundle.windows.certificateThumbprint`. There is no
  `TAURI_WINDOWS_CERT_THUMBPRINT`; setting one does nothing. The workflow
  passes it through `--config`, so no credential is written into the tree.
* **`secrets` is not a context a step's `if` can read.** `if: secrets.X != ''`
  is accepted and is always false. The secrets are lifted to the job's `env`
  and the steps test that.
* **Notarization is run explicitly**, with `xcrun notarytool --wait` and
  `stapler`, rather than left to the bundler — what the bundler does about it
  varies by version, and a silent no-op is the one outcome this must not have.

And the workflow **checks the artefacts rather than the secrets**: it asks
`Get-AuthenticodeSignature` and `codesign --verify` what happened. A step that
reported which secrets were present would have passed happily for the whole
time the Windows wiring was reading a variable nothing looks at.

### Verifying afterwards

Signing is not finished when the build is green. On a machine that has never
seen the certificate:

```bash
# macOS
spctl --assess --type execute --verbose BibleCompose.app
```

```powershell
# Windows
Get-AuthenticodeSignature .\BibleCompose_0.1.0_x64-setup.exe | Format-List
```

The acceptance criterion is *no security warning on first launch*, and the only
way to know is to launch it on a machine that has never run a development
build.

## Cutting a release

1. Update `version` in `crates/biblecompose-tauri/Cargo.toml` and in
   `crates/biblecompose-tauri/tauri.conf.json`. A test fails if the two
   disagree — one is what `--version` prints, the other is what the installer
   registers with the operating system, and a release where they differ
   installs over itself or refuses to.
2. Update `CHANGELOG.md`.
3. Tag `vX.Y.Z` and push it. The workflow runs on the tag and ends by creating
   a **draft** release with every artefact and a `SHA256SUMS` covering them.
4. Read the signing warnings in the installer jobs.
5. **Install one artefact on a machine that has never had a development build
   on it.** Everything above this line is testable in CI; this is not.
6. Publish the draft.

The release is a draft on purpose. The workflow does the work and a person
decides whether it ships — which is also the last gate before an unsigned build
reaches anybody, since the signing steps only warn and a warning nobody reads
is not a gate.

`workflow_dispatch` builds everything and creates no release, so "does this
still work" is a question that can be asked without publishing an answer.

### What lands on the release page

| | |
|---|---|
| `biblecompose-linux-x86_64`, `-macos-*`, `-windows-x86_64.exe` | The command line, one file, typesetter included |
| `.msi`, `-setup.exe`, `.dmg`, `.deb`, `.AppImage` | The window, installed |
| `SHA256SUMS` | Every file above |

The staged runtimes are intermediates and are kept for a day, not released:
they are 79 MB apiece and only the installer jobs want them.

The executables are named for their platform and architecture rather than
`biblecompose`, because Linux's and macOS's are otherwise the same filename —
which nothing notices until they are on one page together. The architecture is
asked of the runner rather than assumed; `macos-latest` is arm64, and a hosted
runner's shape is not a constant.

## Building an installer locally

Needs a runtime to stage. On a machine where the application has already run
once, the unpacked one is under the cache directory:

```bash
node scripts/stage-backend.mjs <runtime> stage
BIBLECOMPOSE_SILE_BUNDLE="$PWD/stage" npx tauri build --features embedded-sile
```

Without `--features embedded-sile` the window is a shell that expects to find
SILE on the machine, which is right for development and wrong for a publisher.
