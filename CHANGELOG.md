# Changelog

Notable changes, newest first. Versions follow [semantic versioning](https://semver.org/).

## Unreleased

* **Drop caps.** On the Contents tab: each chapter opens with its first letter
  dropped into the text, spanning a chosen number of lines. The letter is the
  chapter's first *syllable* — a Tamil consonant with its vowel sign, a
  Devanagari conjunct — rather than its first code point, which would set half
  a syllable three lines tall. With drop caps on, the chapter number takes a
  line of its own and the first verse goes unnumbered — and "Hide first
  verse number" is set with it, and held while drop caps are on.
* **Every build is a full, fresh build.** The Draft and Clean boxes beside
  Generate PDF are gone, with the draft mark, the `-draft.pdf` file and the
  build cache that let an unchanged project skip the typesetter. The CLI's
  `--draft` and `--clean` are gone with them.
* **Head and foot slots are templates.** Each slot is text with fields in
  braces — `{Book}:{FirstChapter}-{FirstVerse}` — chosen from a dropdown of
  the fields or typed into a Custom box; a **?** beside the box opens the list
  of fields.
  The old names (`book_name`, `page_number`…) still read.
* **Left and right pages have their own heads and feet.** The Headers &
  Footers tab shows the spread — the left-hand page beside the right-hand
  one — with six slots on each, so a page number can sit at the outer edge
  of both. In the file, `[headers]` is now `[headers.left_page]` and
  `[headers.right_page]`; a file with the old keys is told so, key by key.
* **Five more templates**: single column, reference, study Bible, pocket
  Bible and journaling Bible, beside the three there were.
* The templates have a tab of their own, and so does what the PDF says
  about itself: publisher, subject, file name and bookmarks are on a **PDF
  metadata** tab instead of an "Other" box under Contents.
* The command-line executable, double-clicked, now says what it is and waits
  instead of closing; it is named `biblecompose-cli-*` on the release page.

## 0.1.1

The start screen now says how to get a Bible.

* **Where to find Scripture to typeset.** The screen used to explain what a
  project is — true, and no use to somebody who has no USFM, which is most
  people opening this for the first time. It now links to
  [Open.Bible](https://www.open.bible/bibles), says how to unpack what you
  download, and says that starting a new translation means selecting an empty
  folder.
* A link on that screen opens in your own browser rather than in the
  application's window, and only `https` addresses are opened.
* Fixed: the three steps were laid out as separate boxes rather than as
  sentences, so the link sat apart from the words before it.

## 0.1.0 — never published

Superseded by 0.1.1 before any artefact was built. Its contents are below
because they are the substance of the first release rather than history.

The first release. A folder of USFM becomes a printable PDF, on a machine with
nothing else installed.

### Composition

* **Scripture**: paragraphs, poetry `q1`–`q4`, section headings `s1`–`s4`,
  `\d`, `\sp`, `\r`, `\sr`, lists `li1`–`li4` and `lim1`–`lim4`, tables with
  measured columns, and the common character styles.
* **Apparatus**: footnotes and cross-references as separate types, with their
  own caller sequences, styles and numbering policies. Cross-references can be
  set at the foot, inline, or under the paragraph that called them.
* **Figures** from project assets, with format sniffed from the bytes rather
  than the extension, and a containment check that a path cannot escape.
* **Running heads and folios** with live verse ranges, correct across a column
  break and on a page whose first verse began earlier.
* **Chapter openings** as drop figures the text runs into.
* A heading is never left alone at the foot of a column.

### The publication

* **Three presets** to start from: standard two-column, a reader's edition, and
  large print. Applying one writes into your settings file, where you can then
  change one line at a time.
* **Styles** keyed by USFM marker, with inheritance by level and an inspector
  that says which file decided each value.
* **PDF properties** — title, author and subject — and named destinations for
  every book and chapter, in the `JHN.3.16` form reference parsers already
  speak. Verse-level anchors are available with `output.anchors = "verse"`.
* **Draft builds**, stamped on every page and written beside the finished PDF
  rather than over it.
* The PDF is named after the publication, in `output/` inside the project.

### Before the build, not after it

Every one of these used to be discovered from the page, or from the backend, or
not at all:

* A font that cannot draw your Scripture, naming the character, its count and a
  verse it appears in.
* A font that resolves but has not got the face a style asked for — silently
  substituted otherwise.
* A figure that is missing, outside the project, or not an image.
* An output folder that does not exist or cannot be written to.
* A settings file that will not parse, which now blocks the build rather than
  being reported and then ignored.

### Building

* **One executable per platform**, carrying its own typesetter. Nothing to
  install alongside it.
* **No internet connection is used or needed.** Nothing below the window links
  an HTTP client, and the shipped typesetter has no socket code in it at all.
* **Your Scripture is never written to**, asserted by checksum after every
  acceptance test.
* A build with nothing changed is skipped and returns at once; `Clean` runs it
  anyway.
* Backend failures arrive as sentences saying what happened and whose fault it
  is, with the raw log kept and collapsed.

### Known limitations

* **The bundled font has no italic**, so styles asking for italic are set in
  the regular face. The build warns (`FONT-005`) and names them. Put a family
  that has one in `assets/fonts/`, or choose one that is installed.
* **Artefacts are unsigned.** Windows shows a SmartScreen warning and macOS
  requires an explicit override in System Settings. Signing needs certificates
  that are bought rather than written.
* **Linux and macOS carry SILE but not the system libraries it links against**
  — HarfBuzz, fontconfig, ICU. The machine that builds them is the machine the
  smoke test runs on, so *works on a fresh machine* is proved on Windows and
  assumed on the other two. The Windows executable carries its own copies.
* **Table cells do not wrap.** A table wider than its column says so on the
  backend log and runs past the margin rather than looking fine and being
  wrong.
* **In two columns, a word wider than the column overhangs it.** Measured at
  five glyph runs on two lines of a Tamil book; there is no legal break inside
  a Tamil word. One column is clean.
* Diagnostics are English only. The window's own text is a catalogue a second
  locale can replace without touching Rust; the diagnostics are not, because
  they interpolate values a template would have to be given.

### Platforms

Windows, macOS and Linux. Windows is verified end to end; the other two are
built by the same workflow and have not yet been run.
