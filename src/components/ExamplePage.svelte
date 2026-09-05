<script lang="ts">
  /**
   * What goes on the page, shown on a page.
   *
   * These settings were a column of checkboxes with names like "Reference range
   * in head". That names a thing without showing it: a publisher who has not
   * seen a running head with a reference range in it cannot tell from the words
   * whether they want one, and finding out meant a full build.
   *
   * So, as on the Page tab, the example *is* the control. A page of 1 John with
   * the switches beside the parts they govern — tick the running head and a
   * running head appears, untick verse numbers and they go.
   *
   * Two tabs share this page and take a switch set each: what is *in* the text
   * and what surrounds it are separate decisions, made at separate times. The
   * page itself always tells the whole truth — turn the running head off on one
   * tab and it is gone from the other, because it is gone from the book.
   *
   * The page is not a proof: it is not SILE, it does not use the project's
   * fonts or trim, and it makes no claim about where a line will break. What it
   * is exact about is the only thing these tabs decide — which of these things
   * are on the page at all.
   *
   * The passage is Berean Standard Bible, public domain, and the same
   * translation the Scripture fixtures use.
   */
  import {
    SAMPLE,
    SAMPLE_ALT_BOOK,
    SAMPLE_BOOK,
    SAMPLE_INTRO,
    SAMPLE_OUTLINE,
  } from "../lib/sample";
  import { wordsFor } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";

  /** Which set of switches to put beside the page. */
  const { which }: { which: "contents" | "headers" } = $props();

  /** A setting's value, defaulting to on so the example is never blank. */
  function on(key: string): boolean {
    return session.settings.find((s) => s.key === key)?.value !== "false";
  }

  async function toggle(key: string, next: boolean): Promise<void> {
    await session.setSetting(key, next ? "true" : "false");
    // Drop caps decide the first verse's number: the initial is its marker,
    // so the number goes. Written into the project rather than only implied
    // by the backend, so the setting says what the page does. Switching
    // drop caps off hands the choice back and does not undo it.
    if (key === "contents.drop_caps" && next) {
      await session.setSetting("numbering.hide_first_verse_number", "true");
    }
  }

  const chapters = on("numbering.show_chapter_numbers");
  const labels = $derived(on("numbering.show_chapter_labels"));
  const verses = $derived(on("numbering.show_verse_numbers"));
  const footnotes = $derived(on("notes.show_footnotes"));
  const refs = $derived(on("notes.show_cross_references"));
  const intros = $derived(on("contents.show_book_introductions"));
  const outlines = $derived(on("contents.show_introductory_outlines"));
  const headings = $derived(on("contents.show_section_headings"));
  const dropcaps = $derived(on("contents.drop_caps"));

  type Section = (typeof SAMPLE)[number]["sections"][number];
  type Verse = Section["verses"][number];

  /**
   * The opening initial of a section's first verse — one grapheme cluster,
   * which is what the backend drops too. `Intl.Segmenter` is the browser's
   * UAX #29, so an example in Tamil would show a syllable and not half of one.
   */
  const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
  function opening(section: Section): string {
    const text = section.verses[0]?.text.trimStart() ?? "";
    const first = segmenter.segment(text)[Symbol.iterator]().next();
    return first.done ? "" : first.value.segment;
  }

  /** The verses to set, with the initial taken off the first when it drops. */
  function versesFor(section: Section, i: number): readonly (Verse & { opened?: boolean })[] {
    if (!(dropcaps && i === 0)) return section.verses;
    const [first, ...rest] = section.verses;
    if (!first) return section.verses;
    const initial = opening(section);
    return [{ ...first, text: first.text.trimStart().slice(initial.length), opened: true }, ...rest];
  }
  const justified = $derived(on("typography.justify"));

  /** What one slot holds, and what that looks like on this page. */
  function slot(key: string): string {
    const value = session.settings.find((s) => s.key === key)?.value ?? "empty";
    switch (value) {
      case "page_number":
        return "412";
      case "reference_range":
        return "1:1–2:6";
      case "first_reference":
        return "1:1";
      case "last_reference":
        return "2:6";
      case "book_name":
        return SAMPLE_BOOK;
      case "alt_book_name":
        return SAMPLE_ALT_BOOK;
      default:
        return "";
    }
  }

  const header = $derived([
    slot("headers.header_left"),
    slot("headers.header_center"),
    slot("headers.header_right"),
  ]);
  const footer = $derived([
    slot("headers.footer_left"),
    slot("headers.footer_center"),
    slot("headers.footer_right"),
  ]);
  const anything = (line: string[]) => line.some((s) => s !== "");

  /** Split a verse so a marker can be dropped in after a given fragment. */
  function around(text: string, after: string): [string, string] {
    const at = text.indexOf(after);
    if (at < 0) return [text, ""];
    const cut = at + after.length;
    return [text.slice(0, cut), text.slice(cut)];
  }

  /** The apparatus at the foot, in the order a reader meets it. */
  const apparatus = $derived.by(() => {
    const out: { kind: "note" | "ref"; mark: string; text: string }[] = [];
    for (const chapter of SAMPLE) {
      for (const section of chapter.sections) {
        for (const verse of section.verses) {
          if (verse.reference && refs) {
            out.push({ kind: "ref", mark: verse.reference.mark, text: verse.reference.note });
          }
          if (verse.footnote && footnotes) {
            out.push({ kind: "note", mark: verse.footnote.mark, text: verse.footnote.note });
          }
        }
      }
    }
    return out;
  });

  /** Which switch the pointer is on, so the page can show what it governs. */
  let lit = $state<string | null>(null);

  /** The Contents switches, grouped by the question they answer. */
  const CONTENTS: readonly {
    title: string;
    switches: readonly {
      key: string;
      label: string;
      /** Idle unless this setting is on: there is nothing for it to act on. */
      under?: string;
      /** Decided, and shown on, while this setting is on. */
      implied?: string;
      note?: string;
      /** For a number: the range the resolver accepts. */
      range?: readonly [number, number];
    }[];
  }[] = [
    {
      title: "Front matter",
      switches: [
        { key: "contents.show_book_introductions", label: "Book introductions" },
        { key: "contents.show_introductory_outlines", label: "Introductory outlines" },
        { key: "contents.show_section_headings", label: "Section headings" },
      ],
    },
    {
      title: "Paragraph",
      switches: [
        { key: "typography.justify", label: "Justify paragraphs" },
        {
          key: "typography.keep_poetry_indentation",
          label: "Keep poetry indentation",
          // 1 John is prose throughout. Saying so beats a switch that looks
          // broken because the passage gives it nothing to do.
          note: "no poetry in this passage",
        },
      ],
    },
    {
      title: "Drop caps",
      switches: [
        {
          key: "contents.drop_caps",
          label: "Drop caps",
          note: "The chapter number takes a line of its own, and the first verse goes unnumbered.",
        },
        {
          key: "contents.drop_cap_lines",
          label: "Lines a drop cap spans",
          // Meaningless without an initial to span them.
          under: "contents.drop_caps",
          // The resolver's own bounds, so the field cannot offer a number
          // the file would refuse.
          range: [2, 6],
        },
      ],
    },
    {
      title: "Numbering",
      switches: [
        { key: "numbering.show_chapter_numbers", label: "Chapter numbers" },
        {
          key: "numbering.show_chapter_labels",
          label: "Chapter labels",
          // A translation either carries `\cl` or it does not, and most do
          // not — so say that this switch may have nothing to act on.
          note: "USFM's \\cl, where a translation has it",
        },
        { key: "numbering.show_verse_numbers", label: "Verse numbers" },
        {
          key: "numbering.hide_first_verse_number",
          label: "Hide first verse number",
          // Nothing to hide when no verse number is shown at all.
          under: "numbering.show_verse_numbers",
          // And nothing to decide under a dropped initial, which is the
          // first verse's marker.
          implied: "contents.drop_caps",
        },
      ],
    },
    {
      title: "Notes",
      switches: [
        { key: "notes.show_footnotes", label: "Footnotes" },
        { key: "notes.footnote_callers", label: "Footnote marks", under: "notes.show_footnotes" },
        { key: "notes.show_cross_references", label: "Cross-references" },
        {
          key: "notes.cross_reference_callers",
          label: "Reference marks",
          under: "notes.show_cross_references",
        },
        {
          key: "notes.cross_reference_placement",
          label: "References go",
          under: "notes.show_cross_references",
        },
        {
          key: "notes.restart_numbering",
          label: "Marks start again",
          // Both sequences, and the only boundary this passage has is the
          // chapter — so say which one the example can actually show.
          note: "at chapter 2, in this passage",
        },
      ],
    },
  ];

  /** The three places at the top of the page, and the three at the foot. */
  const HEADER = [
    { key: "headers.header_left", label: "Left" },
    { key: "headers.header_center", label: "Centre" },
    { key: "headers.header_right", label: "Right" },
  ] as const;
  const FOOTER = [
    { key: "headers.footer_left", label: "Left" },
    { key: "headers.footer_center", label: "Centre" },
    { key: "headers.footer_right", label: "Right" },
  ] as const;

  /**
   * What a slot may hold, from the schema rather than from a list here.
   *
   * There used to be a list here, and it was a second statement of
   * `HeadSlot::NAMES` in a language that cannot be checked against the first —
   * so an eighth thing a head could hold would have reached the file format
   * and not this dropdown, or, worse, the other way round.
   */
  function choicesFor(key: string): readonly string[] {
    return session.settings.find((s) => s.key === key)?.choices ?? [];
  }

  function chosen(key: string): string {
    return session.settings.find((s) => s.key === key)?.value ?? "empty";
  }

  const shows = (key: string) => lit === key;
</script>

<div class="example" class:stacked={which === "headers"}>
  {#if which === "headers"}
    <!--
      The controls sit where their content does: the header's three above the
      page, the footer's three below it, and left, centre and right across the
      width in that order. A dropdown standing over the slot it fills needs no
      label saying which slot that is.
    -->
    <fieldset class="controls">
      <legend>{t("header")}</legend>
      <div class="row">
        {#each HEADER as s (s.key)}
          <label
            onpointerenter={() => (lit = s.key)}
            onpointerleave={() => (lit = null)}
            onfocusin={() => (lit = s.key)}
            onfocusout={() => (lit = null)}
          >
            <select
              aria-label={phrases().headerSlot(s.label.toLowerCase())}
              value={chosen(s.key)}
              disabled={!session.editable}
              onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
            >
              {#each choicesFor(s.key) as choice (choice)}
                <option value={choice}>{wordsFor(choice)}</option>
              {/each}
            </select>
          </label>
        {/each}
      </div>
    </fieldset>
  {/if}

  <!-- The page. Serif, ragged, one column: this is about what is on it, not
       about how it is set — the Page tab answers that. -->
  <div class="paper">
    <div class="line head" class:hidden={!anything(header)}>
      {#each header as part, i (i)}
        <span class="slot" class:lit={shows(HEADER[i]!.key)}>{part}</span>
      {/each}
    </div>

    <div class="body" class:ragged={!justified}>
      {#if intros}
        <div class="front" class:lit={shows("contents.show_book_introductions")}>
          <h3>{SAMPLE_INTRO.heading}</h3>
          {#each SAMPLE_INTRO.paragraphs as para (para)}
            <p class="prose">{para}</p>
          {/each}
        </div>
      {/if}

      {#if outlines}
        <div class="front" class:lit={shows("contents.show_introductory_outlines")}>
          <h3>{SAMPLE_OUTLINE.heading}</h3>
          <ul class="outline">
            {#each SAMPLE_OUTLINE.entries as entry (entry.text)}
              <li class:deep={entry.level > 1}>
                <span>{entry.text}</span>
                <span class="ref">{entry.reference}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#each SAMPLE as chapter (chapter.number)}
        {#each chapter.sections as section, i (section.heading)}
          <!-- Only the section heading. The parallel-passage line below it
               is a different USFM marker and a different setting, which is
               how the backend treats it too. -->
          {#if headings}
            <h3 class:lit={shows("contents.show_section_headings")}>{section.heading}</h3>
          {/if}
          {#if section.parallels}
            <p class="parallels">({section.parallels})</p>
          {/if}

          <p class="prose">
            <!-- The number and, beside it, the label — which is where the
                 backend puts it: `\cl` is its own paragraph and the chapter
                 anchor sits inside it, so the number is set and then the
                 label. An edition that wants one or the other turns one off.
                 With a dropped initial the pair takes a line of its own, as
                 the backend sets it: the initial is the large thing at the
                 corner then, and the number is not set beside it. -->
            {#if i === 0 && (chapters || (chapter.label && labels))}
              <span class="opening" class:own-line={dropcaps}>
                {#if chapters}
                  <span class="chapter" class:lit={shows("numbering.show_chapter_numbers")}>
                    {chapter.number}
                  </span>
                {/if}
                {#if chapter.label && labels}
                  <span class="label" class:lit={shows("numbering.show_chapter_labels")}>
                    {chapter.label}
                  </span>
                {/if}
              </span>
            {/if}
            {#if dropcaps && i === 0 && opening(section)}<span
                class="initial"
                class:lit={shows("contents.drop_caps")}>{opening(section)}</span
              >{/if}{#each versesFor(section, i) as verse (verse.number)}
              {#if verses && !verse.opened}<span
                  class="verse"
                  class:lit={shows("numbering.show_verse_numbers")}
                  >{verse.number}</span
                >{/if}{#if verse.reference}{@const parts = around(verse.text, verse.reference.after)}
                {parts[0]}{#if refs}<sup class="ref" class:lit={shows("notes.show_cross_references")}
                    >{verse.reference.mark}</sup
                  >{/if}{parts[1]}
              {:else if verse.footnote}
                {@const parts = around(verse.text, verse.footnote.after)}
                {parts[0]}{#if footnotes}<sup class="note" class:lit={shows("notes.show_footnotes")}
                    >{verse.footnote.mark}</sup
                  >{/if}{parts[1]}
              {:else}
                {verse.text}
              {/if}
            {/each}
          </p>
        {/each}
      {/each}
    </div>

    {#if apparatus.length > 0}
      <div
        class="apparatus"
        class:lit={shows("notes.show_footnotes") || shows("notes.show_cross_references")}
      >
        {#each apparatus as entry (entry.kind + entry.mark + entry.text)}
          <p><sup>{entry.mark}</sup> {entry.text}</p>
        {/each}
      </div>
    {/if}

    <div class="line foot" class:hidden={!anything(footer)}>
      {#each footer as part, i (i)}
        <span class="slot" class:lit={shows(FOOTER[i]!.key)}>{part}</span>
      {/each}
    </div>
  </div>

  {#if which === "headers"}
    <fieldset class="controls">
      <legend>{t("footer")}</legend>
      <div class="row">
        {#each FOOTER as s (s.key)}
          <label
            onpointerenter={() => (lit = s.key)}
            onpointerleave={() => (lit = null)}
            onfocusin={() => (lit = s.key)}
            onfocusout={() => (lit = null)}
          >
            <select
              aria-label={phrases().footerSlot(s.label.toLowerCase())}
              value={chosen(s.key)}
              disabled={!session.editable}
              onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
            >
              {#each choicesFor(s.key) as choice (choice)}
                <option value={choice}>{wordsFor(choice)}</option>
              {/each}
            </select>
          </label>
        {/each}
      </div>
    </fieldset>
  {:else}
    <!-- The switches, beside what they switch, in the groups they belong to. -->
    <div class="groups">
      {#each CONTENTS as group (group.title)}
        <fieldset>
          <legend>{group.title}</legend>
          <ul class="switches">
            {#each group.switches as s (s.key)}
              {@const implied = s.implied !== undefined && on(s.implied)}
              {@const idle = (s.under !== undefined && !on(s.under)) || implied}
              {@const setting = session.settings.find((x) => x.key === s.key)}
              <li class:nested={s.under !== undefined} class:idle>
                <label
                  onpointerenter={() => (lit = s.key)}
                  onpointerleave={() => (lit = null)}
                  onfocusin={() => (lit = s.key)}
                  onfocusout={() => (lit = null)}
                >
                  <!--
                    Which control this is is the schema's answer and not a
                    field in the list above, so a setting that becomes a choice
                    gets a dropdown here without anyone remembering to say so.
                  -->
                  {#if setting?.kind === "integer"}
                    {s.label}
                    <input
                      type="number"
                      class="count"
                      min={s.range?.[0]}
                      max={s.range?.[1]}
                      value={setting.value}
                      disabled={!session.editable || idle}
                      onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
                    />
                  {:else if setting?.kind === "choice"}
                    {s.label}
                    <select
                      value={setting.value}
                      disabled={!session.editable || idle}
                      onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
                    >
                      {#each setting.choices ?? [] as choice (choice)}
                        <option value={choice}>{wordsFor(choice)}</option>
                      {/each}
                    </select>
                  {:else}
                    <input
                      type="checkbox"
                      checked={on(s.key) || implied}
                      disabled={!session.editable || idle}
                      onchange={(e) => void toggle(s.key, e.currentTarget.checked)}
                    />
                    {s.label}
                  {/if}
                </label>
                {#if s.note}
                  <span class="note">{s.note}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </fieldset>
      {/each}
    </div>
  {/if}
</div>

<style>
  .example {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1.2rem;
    align-items: start;
  }
  /* Head above, page between, foot below — the controls in the order the
     page reads.

     `align-items` has to be reset here. The grid above wants `start`, and the
     same word in a flex column means "size to your content" — which had the
     header box shrink to the width of three dropdowns while the page beside it
     stayed full width. Stretch is what makes the three boxes one column. */
  .example.stacked {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.5rem;
  }
  .controls {
    margin: 0;
    /* The same inline padding as the paper, and the same one-pixel border, so
       the two boxes have their content edges in the same place. Without it the
       outer dropdowns sat 1.2rem wide of the slots they fill: the page's
       padding was inset and the controls' was not. */
    padding: 0 1.2rem 0.5rem;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 3px;
  }
  legend {
    padding-inline: 0.3rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.6;
  }
  /* The same three-part measure the page's own head and foot use, so each
     dropdown stands over the slot it fills. No gap, for the same reason: a
     gap would push the outer two inwards, off the slots they belong to. */
  .row {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
  }
  .row label:first-child {
    justify-self: start;
  }
  .row label:last-child {
    justify-self: end;
  }

  .paper {
    display: flex;
    flex-direction: column;
    min-block-size: 22rem;
    padding: 1rem 1.2rem 0.7rem;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 3px;
    background: color-mix(in oklab, Canvas 94%, CanvasText);
    font-family: Georgia, "Times New Roman", serif;
    font-size: 0.82rem;
    line-height: 1.45;
  }

  /* Three slots on one line: the outer two at the margins, the middle one
     between them. The same arrangement top and bottom, because a head and a
     foot are the same three questions. */
  .line {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: baseline;
    min-block-size: 1.4rem;
    font-size: 0.72rem;
    font-variant: small-caps;
    letter-spacing: 0.04em;
  }
  .line .slot:first-child {
    text-align: start;
  }
  .line .slot:last-child {
    text-align: end;
  }
  .head {
    padding-block-end: 0.3rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 18%, transparent);
  }
  .foot {
    padding-block-start: 0.4rem;
  }
  /* Kept in the layout when empty, so filling a slot does not shift the page
     under the pointer that filled it. */
  .line.hidden {
    visibility: hidden;
  }
  /* An empty slot still holds its place, or the other two would move. */
  .slot:empty::after {
    content: "";
  }
  .count {
    inline-size: 3.2rem;
    padding-block: 0.15rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    font: inherit;
    font-size: 0.82rem;
  }
  select {
    padding-block: 0.15rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    font: inherit;
    font-size: 0.82rem;
  }

  .body {
    flex: 1;
    padding-block-start: 0.6rem;
  }
  h3 {
    margin: 0.7rem 0 0.1rem;
    font-size: 0.82rem;
    font-weight: 700;
  }
  h3:first-child {
    margin-block-start: 0;
  }
  .parallels {
    margin: 0 0 0.25rem;
    font-size: 0.72rem;
    font-style: italic;
    opacity: 0.7;
  }
  .prose {
    margin: 0;
    text-align: justify;
    hyphens: auto;
  }
  /* What `typography.justify` does, on the page it does it to. */
  .body.ragged .prose {
    text-align: start;
  }
  .front {
    margin-block-end: 0.6rem;
  }
  .outline {
    list-style: none;
    margin: 0.15rem 0 0;
    padding: 0;
    font-size: 0.78rem;
  }
  .outline li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .outline li.deep {
    padding-inline-start: 1rem;
  }
  .outline .ref {
    opacity: 0.65;
    font-variant-numeric: tabular-nums;
  }
  /* Beside the drop figure, at body size: it is a line of the translation's
     own words, not a display element of ours. */
  .label {
    margin-inline-end: 0.35em;
  }
  .chapter {
    float: inline-start;
    margin-inline-end: 0.3rem;
    font-size: 2.1rem;
    font-weight: 700;
    line-height: 0.9;
  }
  .verse {
    margin-inline-end: 0.15rem;
    font-size: 0.62rem;
    font-weight: 700;
    vertical-align: super;
  }
  sup.note,
  sup.ref {
    font-size: 0.62rem;
  }
  sup.ref {
    font-style: italic;
  }

  .apparatus {
    margin-block-start: 0.7rem;
    padding-block-start: 0.4rem;
    border-block-start: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    font-size: 0.68rem;
    line-height: 1.35;
  }
  .apparatus p {
    margin: 0;
  }

  /* What the switch under the pointer governs. */
  .lit {
    border-radius: 3px;
    outline: 2px solid var(--lit, #b45309);
    outline-offset: 2px;
    background: color-mix(in oklab, #b45309 18%, transparent);
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .groups fieldset {
    margin: 0;
    padding: 0.1rem 0.7rem 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 18%, transparent);
    border-radius: 6px;
  }
  .groups legend {
    padding-inline: 0.3rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.6;
  }
  .switches {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.85rem;
  }
  .switches li.nested {
    padding-inline-start: 1.2rem;
  }
  .switches li.idle {
    opacity: 0.45;
  }
  .switches .note {
    display: block;
    padding-inline-start: 1.4rem;
    font-size: 0.72rem;
    font-style: italic;
    opacity: 0.55;
  }
  .switches li {
    padding-block: 0.15rem;
  }
  label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    cursor: pointer;
    white-space: nowrap;
  }
  /* A dropped initial: three lines tall, the text wrapping round it. The
     number above it takes its own line, as the backend sets it — and stops
     floating, or it would sit beside the initial and read as one glyph. */
  .opening.own-line {
    display: block;
    line-height: 1.1;
  }
  .opening.own-line .chapter {
    float: none;
    margin-inline-end: 0.15rem;
  }
  .initial {
    float: left;
    font-size: 3.1em;
    line-height: 0.82;
    font-weight: 600;
    padding-inline-end: 0.06em;
    margin-block-start: 0.04em;
  }
</style>
