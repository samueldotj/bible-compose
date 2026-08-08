# BibleCompose

A desktop application that turns a folder of USFM Scripture files into a publication-quality PDF, using SILE as the typesetting backend.

Open a project folder. BibleCompose discovers the books, validates them, merges built-in defaults with your optional settings and style overrides, and composes a PDF. A folder of valid USFM and nothing else is enough to get one.

**The project folder is the source of truth.** No proprietary database, no hidden metadata in your USFM, nothing that stops you editing the files in any other tool.

---

## Status

**Requirements analysed, design proposed, S0 complete. No Rust yet.**

The typesetting spike is done and the answer is yes: SILE sets a Bible page in Latin and in Tamil — balanced two columns, footnotes, running heads carrying the live verse range, vector artwork. Its bundled `bible` class turned out to be unusable, so [`sile/classes/biblecompose.lua`](sile/classes/biblecompose.lua) is ours. Findings and evidence in [spike/NOTES.md](spike/NOTES.md). Work now starts at M0.

| | Milestone | What it means |
|---|---|---|
| **S0** | Typesetting spike ✓ | SILE can set a Bible page. No Rust. |
| **M0** | Skeleton and contract | The pipeline exists end to end on one book. |
| **M1** | USFM to PDF | Real Scripture through the real parser, in two columns. |
| **M2** | Configuration | Page, typography, and output settings, from file and GUI. |
| **M3** | Styles | The visual layer, editable without TOML. |
| **M4** | Publishing structures | Footnotes, cross-references, figures, running heads. |
| **M5** | Hardening | Full corpus, fonts, cancellation, packaging. |
| **M6** | Version 1.0 | Installers, presets, documentation. |

68 work items sized S to XL — see [ROADMAP](docs/ROADMAP.md).

## Documents

| | |
|---|---|
| [SRS v0.1](BibleCompose_Software_Requirements_Specification_v0.1.docx) | The requirements this design answers |
| [SRS-REVIEW](docs/SRS-REVIEW.md) | Analysis of those requirements: findings, gaps, risks, decisions closed |
| [ARCHITECTURE](docs/ARCHITECTURE.md) | The design |
| [ROADMAP](docs/ROADMAP.md) | The sequence and why it is this one |

Decisions and their rejected alternatives:
[001 shared USFM core](docs/adr/001-usfm-core.md) ·
[002 SILE interface](docs/adr/002-sile-interface.md) ·
[003 GUI and preview](docs/adr/003-gui.md) ·
[004 no layout crate](docs/adr/004-no-layout-crate.md) ·
[005 provenance](docs/adr/005-provenance.md)

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

Five choices carry the design:

- **The USFM engine is shared, not rewritten.** [`easy-usfm`](https://github.com/samueldotj/easy-usfm)'s core — parser facade, marker table, diagnostics, corpus, fuzzing, differential oracle — becomes a crate both products depend on.
- **The backend input is XML.** Scripture is a text node, so it cannot become a command. The guarantee that a verse cannot inject Lua is a property of the format, not of an escaping function.
- **Bible typesetting lives in a versioned SILE class.** Rust decides *what*, the class decides *how*.
- **Nothing is neutral that does not need to be.** A `Backend` trait, not a backend-neutral layout model.
- **Resolved settings and styles remember where they came from.** Which is what makes "why does this look like this" answerable.

## Relationship to easy-usfm

[`easy-usfm`](https://github.com/samueldotj/easy-usfm) edits one USFM file. BibleCompose composes a folder of them into a book. They share the engine that reads USFM ([ADR-001](docs/adr/001-usfm-core.md)), so a diagnostic means the same thing in both, and the same stack for the desktop shell ([ADR-003](docs/adr/003-gui.md)).
