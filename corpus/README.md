# Composition corpus

Thirteen complete Bible books, about 1.2 MB, committed and pinned by checksum.

**Whole books, not samples.** [`usfm-core`'s corpus](https://github.com/samueldotj/easy-usfm/tree/main/corpus)
exercises a *parser*, so it is chosen for coverage per file and includes front
matter and fragments. A compositor needs something else: the failures that
matter here — a verse stranded at the foot of a column, a running head that
stops updating, a note that collides with the one below it — only appear over
pages of continuous text. A fragment cannot produce them.

The two corpora are complementary and this one does not replace the other.
Parser-level guarantees are inherited with the dependency ([ADR-001](../docs/adr/001-usfm-core.md)).

## What is here

| | |
|---|---|
| `books/` | the committed files, named `BOOK-translation.usfm` |
| `manifest.toml` | one `[[file]]` per book: checksum, provenance, terms, classification |

Verified by `cargo test -p biblecompose-testkit`, which is a CI gate rather
than something to remember:

```sh
cargo test -p biblecompose-testkit --test corpus
```

It re-hashes every file, checks that each records where it came from and on
what terms, confirms every file is a whole book, and **re-derives the scripts
and feature classes from the bytes** rather than believing the manifest. A
manifest that describes what it wishes were true is worse than no manifest.

`cargo test -p biblecompose-testkit --test normalize_corpus` is the other half:
every book normalizes with no Scripture text lost (P1.6).

## How the selection was made

Greedy set cover. Repeatedly take the book that covers the most still-uncovered
goals per kilobyte, with a penalty for repeating a book code or a language so
the result is not five translations of Mark. Thirteen books cover all
twenty-four goals; the pool was 189 whole books and 19 MB.

Size is a real constraint, not a preference — a corpus nobody wants to clone is
a corpus nobody runs — and there is a test asserting the total stays under
4 MB so the next person to add a book learns that a budget exists.

**Scripts** — Latin, Greek, Cyrillic, Hebrew, Arabic, Devanagari, Tamil,
Bengali, Thai, Khmer, Myanmar, Han. Chosen for what they demand of shaping
rather than for speaker numbers: combining marks, conjunct formation, visual
reordering, right-to-left, and the absence of word spacing. Each is supplied by
a book where it is 94–98 % of the letters, not by an incidental character.

**Feature classes** — notes, poetry, lists, tables, figures, introductions,
titles, character styles, attributes, nested markers, verse ranges, alternate
numbering.

Milestones, sidebars and custom `\z` markers are **deliberately absent**: no
whole book in the pool contains one. They are covered by `usfm-core`'s authored
fixtures, which is the right place for them — this corpus is for what published
books actually do.

## Where the files came from

Vendored from `usfm-core`'s corpus, which sources them from
[eBible.org](https://ebible.org) and a few curated repositories. **All thirteen
checksums are identical to the upstream manifest's**, so the vendoring altered
no bytes and either corpus can be checked against the other.

| Book | Language | Scripts | Translation | Terms |
|---|---|---|---|---|
| 1TI | Limbu | Latin, Limbu | `lifNT` | © 2009 Wycliffe Bible Translators, Inc |
| 2SA | Chinese | Han, Latin | `cmn-cu89t` | Public domain |
| DAN | Serbian | Cyrillic, Latin | `srponspc` | © 2005, 2017 Biblica, Inc |
| ECC | Thai | Latin, Thai | `thaKJV` | © 2003 Philip Pope |
| EPH | Malayalam | Latin, Malayalam | `mal2015` | © 2015 The Free Bible Foundation |
| EST | Hindi | Devanagari, Latin | `freebiblesindia-hindi` | CC BY-SA 4.0 |
| EZR | Burmese | Latin, Myanmar | `mya` | Public domain |
| GAL | Saṃskṛtam | Khmer, Latin | `sankhm` | © 2018 SanskritBible.in |
| HOS | Assamese | Bengali, Latin | `asmfb` | © 2017, 2018 Bridge Connectivity Solutions |
| LAM | Tamil | Latin, Tamil | `freebiblesindia-tamil` | CC BY-SA 4.0 |
| NEH | Hebrew | Hebrew | `hbo` | Public domain |
| PRO | Greek, Ancient | Greek, Latin | `grclxx` | Public domain |
| ROM | Arabic, Standard | Arabic, Latin | `arbnav` | © 1988, 1997, 2012 Biblica, Inc |

## Licensing, stated plainly

**Most of these books are under copyright.** Four are public domain; the rest
are not. They are committed here because their distributor marks them
redistributable, and the manifest records the copyright line and that flag for
every file, so the basis for including each one is auditable without
re-reading anything.

Three things worth being explicit about:

- **The redistributable flag is the distributor's assertion, not legal advice.**
  It is good enough for test data in a public repository. For anything you
  intend to ship in a released artefact, check the publisher's own terms.
- **Two files are CC BY-SA 4.0** — the FreeBiblesIndia Hindi and Tamil texts,
  which require attribution (*"Original work available at
  http://www.freebiblesindia.in"*) and carry a ShareAlike condition. This
  repository is otherwise MIT and now contains CC BY-SA content, which is worth
  knowing before any of it is copied elsewhere.
- **None of this is shipped.** These files are test input. Nothing in `corpus/`
  is compiled into a release artefact or installed on a user's machine; the
  fonts and the SILE runtime are the only third-party content that ships, and
  they are P6.2's problem.

## Adding a book

Put it in `books/`, add a `[[file]]` entry with a real `source`, `copyright`
and `redistributable`, and run the verify test. It will tell you if the
checksum is wrong, if the classification does not match the bytes, or if the
file is not a whole book.

Prefer public domain where a script is already covered by something
restrictive — the corpus does not need to be a licensing liability to do its
job.
