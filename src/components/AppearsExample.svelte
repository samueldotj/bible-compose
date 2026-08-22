<script lang="ts">
  /**
   * What appears on the page, shown on a page.
   *
   * These eight settings were a column of checkboxes with names like "Reference
   * range in head". That names a thing without showing it: a publisher who has
   * not seen a running head with a reference range in it cannot tell from the
   * words whether they want one, and finding out meant a full build.
   *
   * So, as on the Page tab, the example *is* the control. A page of 1 John with
   * the switches beside the parts they govern — tick the running head and a
   * running head appears, untick verse numbers and they go. The page is not a
   * proof: it is not SILE, it does not use the project's fonts or trim, and it
   * makes no claim about where a line will break. What it is exact about is the
   * only thing this tab decides — which of these things are on the page at all.
   *
   * The passage is Berean Standard Bible, public domain, and the same
   * translation the Scripture fixtures use.
   */
  import { SAMPLE, SAMPLE_BOOK } from "../lib/sample";
  import { session } from "../lib/session.svelte";

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
  const heads = $derived(on("headers.enabled"));
  const headBook = $derived(on("headers.show_book_name"));
  const headRange = $derived(on("headers.show_reference_range"));
  const folio = $derived(on("headers.show_page_number"));

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

  const SWITCHES: readonly { key: string; label: string }[] = [
    { key: "numbering.show_chapter_numbers", label: "Chapter numbers" },
    { key: "numbering.show_verse_numbers", label: "Verse numbers" },
    { key: "notes.show_footnotes", label: "Footnotes" },
    { key: "notes.show_cross_references", label: "Cross-references" },
    { key: "headers.enabled", label: "Running heads" },
    { key: "headers.show_book_name", label: "Book name in head" },
    { key: "headers.show_reference_range", label: "Reference range in head" },
    { key: "headers.show_page_number", label: "Page numbers" },
  ];

  const shows = (key: string) => lit === key;
</script>

<div class="example">
  <!-- The page. Serif, ragged, one column: this is about what is on it, not
       about how it is set — the Page tab answers that. -->
  <div class="paper">
    <div
      class="head"
      class:hidden={!heads}
      class:lit={shows("headers.enabled") ||
        shows("headers.show_book_name") ||
        shows("headers.show_reference_range")}
    >
      <span class:faint={!headBook}>{headBook ? SAMPLE_BOOK : ""}</span>
      <span class:faint={!headRange}>{headRange ? "1:1–2:6" : ""}</span>
    </div>

    <div class="body">
      {#each SAMPLE as chapter (chapter.number)}
        {#each chapter.sections as section, i (section.heading)}
          <h3>{section.heading}</h3>
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

    {#if heads && folio}
      <div class="folio" class:lit={shows("headers.show_page_number")}>412</div>
    {/if}
  </div>

  <!-- The switches, beside what they switch. -->
  <ul class="switches">
    {#each SWITCHES as s (s.key)}
      {@const nested = s.key.startsWith("headers.") && s.key !== "headers.enabled"}
      <li class:nested class:off={nested && !heads}>
        <label
          onpointerenter={() => (lit = s.key)}
          onpointerleave={() => (lit = null)}
          onfocusin={() => (lit = s.key)}
          onfocusout={() => (lit = null)}
        >
          <input
            type="checkbox"
            checked={on(s.key)}
            disabled={!session.editable || (nested && !heads)}
            onchange={(e) => toggle(s.key, e.currentTarget.checked)}
          />
          {s.label}
        </label>
      </li>
    {/each}
  </ul>
</div>

<style>
  .example {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1.2rem;
    align-items: start;
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

  .head {
    display: flex;
    justify-content: space-between;
    min-block-size: 1.4rem;
    padding-block-end: 0.3rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 18%, transparent);
    font-size: 0.72rem;
    font-variant: small-caps;
    letter-spacing: 0.04em;
  }
  /* Kept in the layout when it is off, so turning it on does not shift the
     page under the pointer that turned it on. */
  .head.hidden {
    visibility: hidden;
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
  .folio {
    padding-block-start: 0.5rem;
    font-size: 0.72rem;
    text-align: center;
  }

  /* What the switch under the pointer governs. */
  .lit {
    border-radius: 3px;
    outline: 2px solid var(--lit, #b45309);
    outline-offset: 2px;
    background: color-mix(in oklab, #b45309 18%, transparent);
  }
  .faint {
    opacity: 0;
  }

  .switches {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.85rem;
  }
  .switches li {
    padding-block: 0.15rem;
  }
  .switches li.nested {
    padding-inline-start: 1.2rem;
  }
  .switches li.off {
    opacity: 0.45;
  }
  label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    cursor: pointer;
    white-space: nowrap;
  }
</style>
