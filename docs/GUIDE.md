# BibleCompose

Turn a folder of USFM into a printable PDF.

This guide assumes nothing except that you have some Scripture in USFM and want
a book out of it. If you get to the end and have a PDF, it has done its job.

Related: [RELEASING](RELEASING.md) · [ROADMAP](ROADMAP.md)

---

## 1. Start with a folder

A BibleCompose project **is a folder**. There is no database, no import step
and no project file you have to create — a folder with `GEN.usfm` and
`JHN.usfm` in it is already a project, and copying it to another machine
copies everything.

```
My Bible/
  GEN.usfm
  JHN.usfm
```

Sub-folders are searched too, so a folder organised by testament or by
translator works as it is.

**Books are identified by their `\id` marker, not by their filename.** A file
called `genesis-draft-3.usfm` beginning `\id GEN` is Genesis. If two files
claim the same book, the build stops and names both — that is the one thing a
compositor must not guess about.

## 2. Open it

Press **Open a project…** and choose the folder. The window fills in with the
books it found, in canonical order rather than filesystem order.

If anything is wrong with the Scripture, the **Problems** button says how many
and opening it says what. A red count means the build is blocked; a grey one
means there are warnings, which are worth reading and do not stop anything.

## 3. Press Generate PDF

That is the whole of the minimum. Everything unset uses a built-in default that
produces a conventional Bible page: 6×9 inches, two columns, footnotes at the
foot, a running head with the book and the verse range.

The PDF lands in `output/` inside the project, named after the publication.
Click the path on the bar to open it in your usual PDF reader.

---

## Making it look like your edition

### Start from a template

The **Template** tab offers eight to start from:

| | |
|---|---|
| **Standard two-column** | The conventional Bible page |
| **Single column** | The same page in one column, with slightly larger type |
| **Reader's edition** | One column, no verse numbers, no apparatus — set like a novel |
| **Large print** | 14pt in one column, ragged right |
| **Reference** | Two dense columns on a 5.5×8.5in page; the head gives the page's first and last reference; every verse is a PDF destination |
| **Study Bible** | 7×10in with introductions, outlines, headings, footnotes and cross-references all on |
| **Pocket Bible** | 4.25×6.75in, small type in one column, cross-references left out |
| **Journaling Bible** | One column beside a two-inch outer margin left empty for notes |

Choosing one **writes its settings into your project**, where you can then
change them one at a time. It is a starting point, not a mode: after you apply
one there is no preset any more, only settings — which is why nothing in the
window claims you are "in" large print.

### Change one thing at a time

Every tab is settings, and every setting writes to `biblecompose.toml` in your
folder as you change it. There is no Save. Anything you set shows a marker
saying it came from your file rather than from the defaults, and a **Reset**
beside it puts the built-in value back.

| Tab | |
|---|---|
| **Scripture** | Which books, and in what order |
| **Template** | Three kinds of book to start from |
| **Contents** | What appears — introductions, headings, chapter labels, drop caps |
| **Headers & Footers** | What goes in each of the six slots |
| **Page** | Trim size, columns, margins |
| **Styles** | Typography, and every marker's appearance |
| **Figures** | What a figure with no file does to the build |
| **PDF metadata** | Publisher, subject, the file's name, how far its bookmarks reach |

### Styles

The **Styles** tab is where `\q1`'s indent and `\s1`'s size live. They are
keyed by USFM marker, so a change to `poetry.q1` moves every first-level poetry
line and nothing else. Deeper levels inherit from shallower ones: setting
`heading.s1` moves `s2` through `s4` unless they say otherwise.

**Inspect** answers the other question — for any element, what it ended up
looking like and which file decided.

---

## When something goes wrong

### The build is blocked

Open **Problems**. A blocked build lists *every* reason at once rather than
stopping at the first, so one pass through the panel is enough.

The common ones:

| | |
|---|---|
| Two files declare the same book | Delete or move one; nothing else can be decided for you |
| A font cannot draw your Scripture | The message names the character and a verse it appears in |
| A figure's file is not there | Add it, fix the path, or set `assets.missing_figure = "omit"` while you work |
| `biblecompose.toml` will not parse | The message gives the line and column |

### The typesetter failed

Messages beginning `SILE-` come from the typesetter. Most of them are defects
in BibleCompose rather than in your Scripture, and the message says which — if
it says to report it, please do, with the backend log. Every build writes one,
and the **Open folder** button's tooltip says where it is.

### It is slow

A full Bible takes minutes, and almost all of it is the typesetter.

* **Tick Draft.** A draft is stamped on every page and written *beside* your
  real PDF rather than over it, so a proof can never be mistaken for the book.
* **Untick some books** on the Scripture tab. A draft of one book is one book's
  work.
* A build with nothing changed is skipped entirely and returns at once. If you
  changed something outside the project — a system font, artwork elsewhere on
  the disk — tick **Clean** to make it run anyway.

### The italics are not italic

The font BibleCompose ships with has a regular and a bold face and no italic,
so styles asking for italic are set in the regular face. The build warns about
this (`FONT-005`) and names the styles. Choose a family that has an italic, or
put the font file in `assets/fonts/` inside your project.

---

## Things worth knowing

**Your Scripture is never written to.** Not by a build, not by a settings
change, not ever. Every acceptance test checks the checksum of every source
file afterwards.

**Nothing needs the internet.** Not opening a project, not validating, not
building. There is no HTTP client anywhere below the window, and the shipped
typesetter has no socket code in it at all.

**A font a project ships travels with it.** Put a `.ttf` or `.otf` in
`assets/fonts/` and it is found before anything installed on the machine, so a
publication sets the same way on a machine that has never seen that font.

**Hiding a number does not remove the place it marks.** A reader's edition with
no verse numbers still knows its own verse ranges — which is how its running
heads stay right — and can still be linked into.

---

## From a terminal

Everything above is available without the window:

```bash
biblecompose build --books ./My\ Bible --project ./My\ Bible
```

```bash
biblecompose validate --books ./My\ Bible
```

`build` takes `--draft`, `--clean`, `--output` and `--keep-intermediates`.
`biblecompose version` reports the application and the typesetter it carries.
