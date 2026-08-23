/**
 * The passage the Contents tab sets as an example.
 *
 * 1 John 1 and the opening of 2, in the Berean Standard Bible — public domain,
 * and the same translation the fixtures in `biblecompose-scripture` use, so
 * there is one answer in this repository to "whose words are these".
 *
 * Two chapters rather than one because the chapter number is one of the things
 * being switched on and off, and a chapter number is only visible where a
 * chapter begins. The text is not abridged where it appears: 1 John 1 is whole,
 * and chapter 2 stops after six verses because a preview is a page and not a
 * publication.
 *
 * The footnotes are the ones the BSB carries at these verses. The
 * cross-reference is illustrative — BSB prints its parallel passages as the
 * `\r` line under a heading rather than as `\x` notes — and it is here because
 * `notes.show_cross_references` governs `\x`, and a switch that visibly does
 * nothing teaches the wrong thing about itself.
 */

export interface SampleVerse {
  readonly number: number;
  readonly text: string;
  /** A footnote marker sits after this fragment of the verse. */
  readonly footnote?: { readonly after: string; readonly mark: string; readonly note: string };
  /** And a cross-reference marker, likewise. */
  readonly reference?: { readonly after: string; readonly mark: string; readonly note: string };
}

export interface SampleSection {
  readonly heading: string;
  /** The parallel passages BSB prints under the heading — USFM's `\r`. */
  readonly parallels?: string;
  readonly verses: readonly SampleVerse[];
}

export interface SampleChapter {
  readonly number: number;
  /**
   * USFM's `\cl` — the words a translation gives the chapter, beside or
   * instead of the figure.
   *
   * Illustrative, like the introduction and the outline below: BSB carries no
   * `\cl`. It is here because `numbering.show_chapter_labels` governs it, and
   * a switch with nothing to switch teaches the wrong thing about itself. A
   * translation that has them usually has them in its own language —
   * `அத்தியாயம் 1` — and this page is in English throughout.
   */
  readonly label?: string;
  readonly sections: readonly SampleSection[];
}

export const SAMPLE_BOOK = "1 John";
/** USFM's `\toc1` — the fuller form, for a head slot that asks for it. */
export const SAMPLE_ALT_BOOK = "The First Epistle of John";

/**
 * The book's front matter: an introduction and an outline.
 *
 * Editorial rather than Scripture, and written for this preview rather than
 * taken from an edition — BSB ships no book introductions, and the point here
 * is to have something for `contents.show_book_introductions` and
 * `contents.show_introductory_outlines` to act on. A switch with nothing to
 * switch teaches the wrong thing about itself, which is the whole reason this
 * tab is a page and not a list of names.
 *
 * `\is` and `\ip` are the introduction's heading and prose; `\iot` and `\io`
 * are the outline's. They are separate settings because they are separate
 * markers, and an edition that wants the outline without the essay is asking
 * for exactly that.
 */
export const SAMPLE_INTRO = {
  heading: "Introduction",
  paragraphs: [
    "This letter was written to assure believers of eternal life and to warn " +
      "them against teachers who denied that Christ had come in the flesh.",
  ],
} as const;

export const SAMPLE_OUTLINE = {
  heading: "Outline",
  entries: [
    { level: 1, text: "Fellowship with God", reference: "1:1–2:6" },
    { level: 2, text: "Walking in the light", reference: "1:5–10" },
    { level: 1, text: "Love and obedience", reference: "2:7–17" },
  ],
} as const;

export const SAMPLE: readonly SampleChapter[] = [
  {
    number: 1,
    label: "Chapter One",
    sections: [
      {
        heading: "The Word of Life",
        parallels: "Luke 24:36–49; John 20:19–23",
        verses: [
          {
            number: 1,
            text:
              "That which was from the beginning, which we have heard, which we have seen with " +
              "our own eyes, which we have gazed upon and touched with our own hands—this is the " +
              "Word of life.",
            reference: {
              after: "the beginning,",
              mark: "a",
              note: "1:1 John 1:1; John 1:14",
            },
          },
          {
            number: 2,
            text:
              "And this is the life that was revealed; we have seen it and testified to it, and " +
              "we proclaim to you the eternal life that was with the Father and was revealed to us.",
          },
          {
            number: 3,
            text:
              "We proclaim to you what we have seen and heard, so that you also may have " +
              "fellowship with us. And this fellowship of ours is with the Father and with His " +
              "Son, Jesus Christ.",
          },
          {
            number: 4,
            text: "We write these things so that our joy may be complete.",
            footnote: {
              after: "our",
              mark: "a",
              note: "1:4 BYZ and TR read your instead of our.",
            },
          },
        ],
      },
      {
        heading: "Walking in the Light",
        parallels: "John 8:12–29",
        verses: [
          {
            number: 5,
            text:
              "And this is the message we have heard from Him and announce to you: God is light, " +
              "and in Him there is no darkness at all.",
          },
          {
            number: 6,
            text:
              "If we say we have fellowship with Him yet walk in the darkness, we lie and do not " +
              "practice the truth.",
          },
          {
            number: 7,
            text:
              "But if we walk in the light as He is in the light, we have fellowship with one " +
              "another, and the blood of Jesus His Son cleanses us from all sin.",
            footnote: {
              after: "But",
              mark: "b",
              note: "1:7 NA does not include But.",
            },
          },
          {
            number: 8,
            text: "If we say we have no sin, we deceive ourselves, and the truth is not in us.",
          },
          {
            number: 9,
            text:
              "If we confess our sins, He is faithful and just to forgive us our sins and to " +
              "cleanse us from all unrighteousness.",
          },
          {
            number: 10,
            text:
              "If we say we have not sinned, we make Him out to be a liar, and His word is not in us.",
          },
        ],
      },
    ],
  },
  {
    number: 2,
    label: "Chapter Two",
    sections: [
      {
        heading: "Jesus Our Advocate",
        verses: [
          {
            number: 1,
            text:
              "My little children, I am writing these things to you so that you will not sin. " +
              "But if anyone does sin, we have an advocate before the Father—Jesus Christ, the " +
              "Righteous One.",
          },
          {
            number: 2,
            text:
              "He Himself is the atoning sacrifice for our sins, and not only for ours but also " +
              "for the sins of the whole world.",
            footnote: {
              after: "atoning sacrifice",
              mark: "a",
              note: "2:2 Or the propitiation",
            },
          },
          {
            number: 3,
            text:
              "By this we can be sure that we have come to know Him: if we keep His commandments.",
          },
          {
            number: 4,
            text:
              "If anyone says, “I know Him,” but does not keep His commandments, he is a liar, " +
              "and the truth is not in him.",
          },
          {
            number: 5,
            text:
              "But if anyone keeps His word, the love of God has been truly perfected in him. By " +
              "this we know that we are in Him:",
          },
          { number: 6, text: "Whoever claims to abide in Him must walk as Jesus walked." },
        ],
      },
    ],
  },
];
