<script lang="ts">
  /**
   * Find a tab, a setting, a style or a template by name, and go there.
   *
   * A box at the end of the tab strip. Typing lists what matches; Enter or
   * a click opens the tab the hit lives on — and the Styles section, when it
   * is one — and lights the control up for a moment. Escape clears. The
   * index is built from what the window holds, so a setting added later is
   * found without this file hearing of it (see `lib/search.ts`).
   */
  import { index, search, type Hit } from "../lib/search";
  import { reveal } from "../lib/spotlight.svelte";
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";

  let query = $state("");
  let open = $state(false);
  let active = $state(0);
  let box: HTMLInputElement | undefined = $state();

  const hits = $derived(index(session.settings, session.presets));
  const found = $derived(search(hits, query));

  $effect(() => {
    // The templates are compiled in and read once; ask for them the first
    // time the box could list them.
    if (open) void session.loadPresets();
  });

  /** Go to a hit: its tab, its section, and the control itself. */
  function go(hit: Hit): void {
    if (hit.tab) session.pane = hit.tab;
    if (hit.subtab) session.stylePane = hit.subtab;
    query = "";
    open = false;
    if (hit.key) void reveal(hit.key);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      query = "";
      open = false;
      box?.blur();
      return;
    }
    if (!open || found.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      active = (active + 1) % found.length;
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      active = (active - 1 + found.length) % found.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      const hit = found[Math.min(active, found.length - 1)];
      if (hit) go(hit);
    }
  }

  // A new query starts the selection at the top.
  $effect(() => {
    void query;
    active = 0;
  });
</script>

<div class="search" role="search">
  <input
    bind:this={box}
    type="search"
    role="combobox"
    aria-label={t("searchHint")}
    aria-expanded={open && found.length > 0}
    aria-controls="search-hits"
    aria-autocomplete="list"
    placeholder={t("searchHint")}
    spellcheck="false"
    bind:value={query}
    onfocus={() => (open = true)}
    onblur={() => window.setTimeout(() => (open = false), 150)}
    {onkeydown}
  />
  {#if open && query.trim() !== ""}
    <ul id="search-hits" class="hits" role="listbox" aria-label={phrases().searchHits(found.length)}>
      {#each found as hit, i (hit.id)}
        <li
          role="option"
          aria-selected={i === active}
          class:active={i === active}
          onpointerenter={() => (active = i)}
          onmousedown={(e) => {
            e.preventDefault();
            go(hit);
          }}
        >
          <span class="title">{hit.title}</span>
          <span class="path">{hit.path}</span>
        </li>
      {:else}
        <li class="none" role="option" aria-selected="false">{t("nothingMatches")}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .search {
    position: relative;
    margin-inline-start: auto;
    align-self: center;
  }
  input {
    inline-size: 15rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
  }
  input:focus {
    outline: 2px solid #b45309;
    outline-offset: 1px;
  }
  .hits {
    position: absolute;
    inset-inline-end: 0;
    inset-block-start: calc(100% + 0.3rem);
    z-index: 5;
    min-inline-size: 22rem;
    max-inline-size: 30rem;
    margin: 0;
    padding: 0.25rem;
    list-style: none;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 6px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 6px 24px rgb(0 0 0 / 0.3);
  }
  li {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.3rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }
  li.active {
    background: color-mix(in oklab, #b45309 22%, transparent);
  }
  li.none {
    cursor: default;
    opacity: 0.6;
  }
  .path {
    opacity: 0.6;
    font-size: 0.78rem;
    white-space: nowrap;
  }
</style>
