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
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | macOS | Notarization; an app-specific password, not the account one |
| `WINDOWS_CERT_THUMBPRINT` | Windows | The thumbprint of a code-signing certificate already in the runner's store |

Windows takes a thumbprint rather than the certificate itself, so the private
key is never written to a runner's disk. macOS needs the `.p12` because
`codesign` needs a keychain; it is imported to a temporary keychain and
discarded with the runner.

Both are **purchased and issued by a third party**, and the lead time is days
rather than minutes — which is why P6.1 sits where it does in the roadmap.

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

1. Update `version` in the workspace `Cargo.toml` and in
   `crates/biblecompose-tauri/tauri.conf.json`. They are checked against each
   other by CI.
2. Update `CHANGELOG.md`.
3. Tag `vX.Y.Z` and push it. The workflow runs on the tag.
4. Download the artefacts, and **install one of them on a machine that has
   never had a development build on it**. Everything above this line is
   testable in CI; this is not.

## Building an installer locally

Needs a runtime to stage. On a machine where the application has already run
once, the unpacked one is under the cache directory:

```bash
node scripts/stage-backend.mjs <runtime> stage
BIBLECOMPOSE_SILE_BUNDLE="$PWD/stage" npx tauri build --features embedded-sile
```

Without `--features embedded-sile` the window is a shell that expects to find
SILE on the machine, which is right for development and wrong for a publisher.
