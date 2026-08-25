# BibleCompose

A desktop application that turns a folder of USFM Scripture files into a publication-quality PDF, using SILE as the typesetting backend.

Open a project folder. BibleCompose discovers the books, validates them, merges built-in defaults with your optional settings and style overrides, and composes a PDF. A folder of valid USFM and nothing else is enough to get one.

**The project folder is the source of truth.** No proprietary database, no hidden metadata in your USFM, nothing that stops you editing the files in any other tool.

---

## Status

**M0 through M5 complete, and most of M6.** A folder of USFM opens in a window,
is validated, styled, and composed into a PDF; the application ships as one
executable carrying its own typesetter. 499 tests, 9 crates.

```bash
biblecompose build --books ./MyBible --project ./MyBible
```

New here? [**The guide**](docs/GUIDE.md) goes from a folder of USFM to a PDF and
assumes nothing else.

**What is left is not code.** Packaging needs a macOS and a Linux machine to run
on; installers need signing certificates, which are bought rather than written;
the bundled fonts need their redistribution terms settled with the people who
own them. Everything those wait on is outside this repository.

S0 answered the feasibility question — SILE sets a Bible page in Latin and in Tamil, with balanced columns, footnotes, running heads carrying the live verse range, and vector artwork. Its bundled `bible` class turned out to be unusable, so [`sile/classes/biblecompose.lua`](sile/classes/biblecompose.lua) is ours. Findings and evidence in [spike/NOTES.md](spike/NOTES.md).

| | Milestone | What it means |
|---|---|---|
| **S0** | Typesetting spike ✓ | SILE can set a Bible page. No Rust. |
| **M0** | Skeleton and contract ✓ | The pipeline exists end to end on one book. |
| **S1** | Packaging spike ✓ | What a single binary costs, and whether Windows is a wall. |
| **M1** | USFM to PDF ✓ | Real Scripture through the real parser, in two columns. |
| **M2** | Configuration ✓ | Page, typography, and output settings, from file and GUI. |
| **M3** | Styles ✓ | The visual layer, editable without TOML. |
| **M4** | Publishing structures ✓ | Footnotes, cross-references, figures, running heads. |
| **M5** | Hardening | Eight of nine. The ninth is packaging, and needs two more machines. |
| **M6** | Version 1.0 | Accessibility, localization and the ten acceptance scenarios done; installers, fonts and sign-off are not code. |

73 work items sized S to XL — see [ROADMAP](docs/ROADMAP.md).

## Documents

| | |
|---|---|
| [GUIDE](docs/GUIDE.md) | **Start here.** A folder of USFM to a PDF, assuming nothing else |
| [SRS v0.1](docs/SRS-v0.1.md) | The requirements this design answers — the source document, unedited |
| [SRS-REVIEW](docs/SRS-REVIEW.md) | Analysis of those requirements: findings, gaps, risks, decisions closed |
| [TRACEABILITY](docs/TRACEABILITY.md) | Every MUST in the SRS, and the test that answers it |
| [ARCHITECTURE](docs/ARCHITECTURE.md) | The design |
| [ROADMAP](docs/ROADMAP.md) | The sequence and why it is this one |
| [RELEASING](docs/RELEASING.md) | How a version becomes three installers, and what has to be supplied |

Decisions and their rejected alternatives:
[001 shared USFM core](docs/adr/001-usfm-core.md) ·
[002 SILE interface](docs/adr/002-sile-interface.md) ·
[003 GUI and preview](docs/adr/003-gui.md) ·
[004 no layout crate](docs/adr/004-no-layout-crate.md) ·
[005 provenance](docs/adr/005-provenance.md) ·
[006 single binary](docs/adr/006-single-binary.md)

## Design in one page

```text
Project folder  ← authoritative
      ↓
usfm-core   → UsfmDocument (USJ, source-faithful, spans)     shared with easy-usfm
      ↓ normalization
ScriptureDocument   ← composition-oriented, canon-ordered
      ↓ + ResolvedSettings + ResolvedStyles   (carrying where each value came from)
XML emission   ← Scripture is a text node, never syntax
      ↓
SILE + the BibleCompose class
      ↓
PDF
```

Six choices carry the design:

- **The USFM engine is shared, not rewritten.** [`easy-usfm`](https://github.com/samueldotj/easy-usfm)'s core — parser facade, marker table, diagnostics, corpus, fuzzing, differential oracle — becomes a crate both products depend on.
- **The backend input is XML.** Scripture is a text node, so it cannot become a command. The guarantee that a verse cannot inject Lua is a property of the format, not of an escaping function.
- **Bible typesetting lives in a versioned SILE class.** Rust decides *what*, the class decides *how*.
- **Nothing is neutral that does not need to be.** A `Backend` trait, not a backend-neutral layout model.
- **Resolved settings and styles remember where they came from.** Which is what makes "why does this look like this" answerable.
- **One binary, but still two processes.** The application re-executes itself to typeset, so it ships as a single file without giving up cancellation or crash isolation.

## Relationship to easy-usfm

[`easy-usfm`](https://github.com/samueldotj/easy-usfm) edits one USFM file. BibleCompose composes a folder of them into a book. They share the engine that reads USFM ([ADR-001](docs/adr/001-usfm-core.md)), so a diagnostic means the same thing in both, and the same stack for the desktop shell ([ADR-003](docs/adr/003-gui.md)).
