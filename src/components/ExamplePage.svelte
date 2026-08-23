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
  import { session } from "../lib/session.svelte";

  /** Which set of switches to put beside the page. */
  const { which }: { which: "contents" | "headers" } = $props();

  /** A setting's value, defaulting to on so the example is never blank. */
  function on(key: string): boolean {
    return session.settings.find((s) => s.key === key)?.value !== "false";
  }

  function toggle(key: string, next: boolean): void {
    void session.setSetting(key, next ? "true" : "false");
  }

  const chapters = on("numbering.show_chapter_numbers");
  const verses = $derived(on("numbering.show_verse_numbers"));
  const footnotes = $derived(on("notes.show_footnotes"));
  const refs = $derived(on("notes.show_cross_references"));
  const intros = $derived(on("contents.show_book_introductions"));
  const outlines = $derived(on("contents.show_introductory_outlines"));
  const headings = $derived(on("contents.show_section_headings"));
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
    switches: readonly { key: string; label: string; under?: string; note?: string }[];
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
      title: "Setting",
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
      title: "Numbering",
      switches: [
        { key: "numbering.show_chapter_numbers", label: "Chapter numbers" },
        { key: "numbering.show_verse_numbers", label: "Verse numbers" },
        {
          key: "numbering.hide_first_verse_number",
          label: "Hide first verse number",
          // Nothing to hide when no verse number is shown at all.
          under: "numbering.show_verse_numbers",
        },
      ],
    },
    {
      title: "Notes",
      switches: [
        { key: "notes.show_footnotes", label: "Footnotes" },
        { key: "notes.show_cross_references", label: "Cross-references" },
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

  /** What a slot may hold. The order the list is offered in. */
  const CHOICES: readonly { value: string; label: string }[] = [
    { value: "empty", label: "Empty" },
    { value: "page_number", label: "Page number" },
    { value: "reference_range", label: "Reference range" },
    { value: "first_reference", label: "First reference" },
    { value: "last_reference", label: "Last reference" },
    { value: "book_name", label: "Book name" },
    { value: "alt_book_name", label: "Alt book name" },
  ];

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
      <legend>Header</legend>
      <div class="row">
        {#each HEADER as s (s.key)}
          <label
            onpointerenter={() => (lit = s.key)}
            onpointerleave={() => (lit = null)}
            onfocusin={() => (lit = s.key)}
            onfocusout={() => (lit = null)}
          >
            <select
              aria-label={`Header ${s.label.toLowerCase()}`}
              value={chosen(s.key)}
              disabled={!session.editable}
              onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
            >
              {#each CHOICES as choice (choice.value)}
                <option value={choice.value}>{choice.label}</option>
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
            {#if i === 0 && chapters}
              <span class="chapter" class:lit={shows("numbering.show_chapter_numbers")}>
                {chapter.number}
              </span>
            {/if}
            {#each section.verses as verse (verse.number)}
              {#if verses}<span class="verse" class:lit={shows("numbering.show_verse_numbers")}
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
      <legend>Footer</legend>
      <div class="row">
        {#each FOOTER as s (s.key)}
          <label
            onpointerenter={() => (lit = s.key)}
            onpointerleave={() => (lit = null)}
            onfocusin={() => (lit = s.key)}
            onfocusout={() => (lit = null)}
          >
            <select
              aria-label={`Footer ${s.label.toLowerCase()}`}
              value={chosen(s.key)}
              disabled={!session.editable}
              onchange={(e) => void session.setSetting(s.key, e.currentTarget.value)}
            >
              {#each CHOICES as choice (choice.value)}
                <option value={choice.value}>{choice.label}</option>
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
              {@const idle = s.under !== undefined && !on(s.under)}
              <li class:nested={s.under !== undefined} class:idle>
                <label
                  onpointerenter={() => (lit = s.key)}
                  onpointerleave={() => (lit = null)}
                  onfocusin={() => (lit = s.key)}
                  onfocusout={() => (lit = null)}
                >
                  <input
                    type="checkbox"
                    checked={on(s.key)}
                    disabled={!session.editable || idle}
                    onchange={(e) => toggle(s.key, e.currentTarget.checked)}
                  />
                  {s.label}
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
</style>
