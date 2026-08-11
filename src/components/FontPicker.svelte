<script lang="ts">
  /**
   * GUI-003: choosing a font from what exists, rather than spelling one.
   *
   * Not the operating system's font dialog, for two reasons. It offers every
   * face installed on this machine with no idea which of them can draw the
   * book — and choosing one that cannot is precisely the mistake FONT-002
   * exists to catch, so a picker that allows it silently has only moved the
   * error later. And it knows nothing about the fonts the *project* ships or
   * the ones the typesetting backend brings with it, which are the two sets a
   * publisher most wants: a font that travels with the book renders the same
   * way on somebody else's machine, and one merely installed here does not.
   *
   * So the list is the same one a build resolves against, in the same order,
   * with the coverage answer beside each name.
   */
  import { untrack } from "svelte";
  import { backend, type FontChoice } from "../lib/services/backend";
  import { session } from "../lib/session.svelte";

  const {
    current,
    onchoose,
    onclose,
  }: {
    current: string;
    onchoose: (family: string) => void;
    onclose: () => void;
  } = $props();

  let fonts = $state<readonly FontChoice[] | null>(null);
  let failure = $state<string | null>(null);
  let filter = $state("");
  /** Whether to hide the ones that cannot set this Scripture. */
  let coveringOnly = $state(true);
  /**
   * The initial value deliberately: the dialog opens on whatever is set, and
   * from then on the selection is the person's. It is a fresh component each
   * time it opens, so there is nothing stale to carry over.
   */
  let selected = $state(untrack(() => current));

  /**
   * Loaded when the dialog opens rather than kept in the session.
   *
   * Reading three hundred character maps takes a moment, and an installed
   * font can appear between one opening and the next. Neither is worth a
   * cache that can be wrong.
   */
  $effect(() => {
    void backend()
      .fonts(session.project?.root ?? null)
      .then((list) => (fonts = list))
      .catch((e: unknown) => {
        failure = String(e);
        fonts = [];
      });
  });

  /** Whether anything in the list has been checked against Scripture at all. */
  const checked = $derived((fonts ?? []).some((f) => f.missing !== undefined));

  const shown = $derived.by(() => {
    const needle = filter.trim().toLowerCase();
    return (fonts ?? []).filter((f) => {
      if (needle && !f.family.toLowerCase().includes(needle)) return false;
      // The one currently set always shows, whatever the filters say. A
      // dialog that hides the answer to "what is this now" is disorienting,
      // and it is the case where the font does not cover the book that a
      // publisher most needs to see it.
      if (f.family === current) return true;
      if (coveringOnly && checked && (f.missing ?? 0) > 0) return false;
      return true;
    });
  });

  const groups = $derived.by(() => {
    const order = [
      { id: "project", title: "In this project", note: "ships with the book" },
      { id: "backend", title: "Bundled", note: "ships with BibleCompose" },
      { id: "system", title: "Installed here", note: "on this machine only" },
    ] as const;
    return order
      .map((g) => ({ ...g, rows: shown.filter((f) => f.source === g.id) }))
      .filter((g) => g.rows.length > 0);
  });

  function coverage(font: FontChoice): string {
    if (font.missing === undefined) return "";
    if (font.missing === 0) return "sets this Scripture";
    return `cannot draw ${font.missing} character${font.missing === 1 ? "" : "s"}`;
  }

  function confirm(): void {
    if (selected && selected !== current) onchoose(selected);
    onclose();
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window {onkeydown} />

<!--
  A backdrop that closes on click, with the dialog stopping the event. The
  keyboard has Escape and the Cancel button, so the backdrop is a convenience
  and not the only way out — which is what makes it acceptable on a div.
-->
<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Choose a font">
    <header>
      <h2>Choose a font</h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        placeholder="Search fonts"
        spellcheck="false"
        autofocus
        bind:value={filter}
      />
    </header>

    {#if checked}
      <label class="only">
        <input type="checkbox" bind:checked={coveringOnly} />
        Only fonts that can set this Scripture
      </label>
    {:else if !session.project}
      <p class="note">No project is open, so nothing has been checked against Scripture.</p>
    {/if}

    <div class="list">
      {#if fonts === null}
        <p class="note">Reading the fonts on this machine…</p>
      {:else if failure}
        <p class="error">{failure}</p>
      {:else if shown.length === 0}
        <p class="note">
          Nothing matches{coveringOnly && checked ? " that can set this Scripture" : ""}.
        </p>
      {:else}
        {#each groups as group (group.id)}
          <h3>{group.title} <span>— {group.note}</span></h3>
          {#each group.rows as font (font.family)}
            <button
              type="button"
              class="font"
              class:selected={font.family === selected}
              class:short={(font.missing ?? 0) > 0}
              onclick={() => (selected = font.family)}
              ondblclick={confirm}
            >
              <span class="family" style={`font-family: ${JSON.stringify(font.family)}, serif`}>
                {font.family}
              </span>
              <span class="coverage">{coverage(font)}</span>
            </button>
          {/each}
        {/each}
      {/if}
    </div>

    <footer>
      <span class="chosen">{selected}</span>
      <button type="button" onclick={onclose}>Cancel</button>
      <button type="button" class="primary" onclick={confirm}>Use this font</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.4);
    z-index: 10;
  }
  .dialog {
    display: flex;
    flex-direction: column;
    inline-size: min(34rem, 90vw);
    block-size: min(32rem, 85vh);
    padding: 0.9rem 1rem 0.8rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
  }
  header {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  h3 {
    position: sticky;
    inset-block-start: 0;
    margin: 0.7rem 0 0.2rem;
    padding-block: 0.2rem;
    background: Canvas;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.7;
  }
  h3 span {
    text-transform: none;
    letter-spacing: normal;
    opacity: 0.75;
  }
  input[type="search"] {
    flex: 1;
    max-inline-size: 16rem;
    padding: 0.25rem 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.9rem;
  }
  .only {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    padding-block: 0.45rem;
    font-size: 0.82rem;
    opacity: 0.85;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding-inline-end: 0.2rem;
    border-block: 1px solid color-mix(in oklab, currentColor 15%, transparent);
  }
  .font {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    justify-content: space-between;
    inline-size: 100%;
    padding: 0.3rem 0.45rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .font:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .font.selected {
    border-color: color-mix(in oklab, currentColor 40%, transparent);
    background: color-mix(in oklab, currentColor 12%, transparent);
  }
  .family {
    font-size: 1.02rem;
  }
  .coverage {
    flex: none;
    font-size: 0.74rem;
    opacity: 0.6;
  }
  .font.short .coverage {
    color: #c0392b;
    opacity: 0.9;
  }
  footer {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding-block-start: 0.7rem;
  }
  .chosen {
    flex: 1;
    overflow: hidden;
    font-size: 0.82rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.7;
  }
  footer button {
    padding: 0.28rem 0.7rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  footer button.primary {
    border-color: transparent;
    background: color-mix(in oklab, currentColor 20%, transparent);
    font-weight: 600;
  }
  .note,
  .error {
    margin: 0.5rem 0;
    font-size: 0.85rem;
    opacity: 0.7;
  }
  .error {
    color: #c0392b;
    opacity: 1;
  }
</style>
