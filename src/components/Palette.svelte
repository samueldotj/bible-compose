<script lang="ts">
  /**
   * Find a setting: any section, setting, style or template by name, and
   * go to it.
   *
   * A dialog over the window, opened from the box in the top bar or with
   * Ctrl K. Typing lists what matches; Enter or a click opens the section
   * the hit lives on — and the entry of Styles, when it is one — and lights
   * the control up for a moment. The index is built from what the window
   * holds, so a setting added later is found without this file hearing of
   * it (see `lib/search.ts`).
   */
  import { index, search, type Hit } from "../lib/search";
  import { reveal } from "../lib/spotlight.svelte";
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { modal } from "../lib/modal";
  import { phrases, t } from "../lib/i18n";

  let query = $state("");
  let active = $state(0);

  const hits = $derived(index(session.settings, session.presets));
  const found = $derived(search(hits, query));

  $effect(() => {
    void session.loadPresets();
  });

  function close(): void {
    ui.palette = false;
  }

  function go(hit: Hit): void {
    if (hit.tab) session.pane = hit.tab;
    if (hit.subtab) session.stylePane = hit.subtab;
    close();
    if (hit.key) void reveal(hit.key);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (found.length === 0) return;
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

  $effect(() => {
    void query;
    active = 0;
  });
</script>

<svelte:window {onkeydown} />

<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) close();
  }}
>
  <div class="palette" role="dialog" aria-modal="true" aria-label={t("paletteTitle")} tabindex="-1" use:modal>
    <div class="ask">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.3-4.3"></path>
      </svg>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        role="combobox"
        aria-label={t("paletteTitle")}
        aria-expanded={found.length > 0}
        aria-controls="palette-hits"
        aria-autocomplete="list"
        placeholder={t("findSetting")}
        spellcheck="false"
        autofocus
        bind:value={query}
      />
      <span class="count">{phrases().settingsFound(found.length)}</span>
    </div>
    <ul id="palette-hits" role="listbox" aria-label={phrases().searchHits(found.length)}>
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
          <span class="name">{hit.title}</span>
          <span class="path">{hit.path}</span>
          {#if hit.hint}<span class="hint">{hit.hint}</span>{/if}
        </li>
      {:else}
        {#if query.trim() !== ""}
          <li class="none" role="option" aria-selected="false">{t("nothingMatches")}</li>
        {/if}
      {/each}
    </ul>
    <div class="keys">
      <span>{t("moveHint")}</span><span>{t("openHint")}</span><span>{t("closeHint")}</span>
    </div>
  </div>
</div>

<style>
  .backdrop {
    align-items: start;
    padding-block-start: 96px;
  }
  .palette {
    inline-size: min(620px, 92vw);
    max-block-size: min(70vh, 640px);
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line2);
    border-radius: 12px;
    background: var(--panel);
    color: var(--ink);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  .ask {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
    border-block-end: 1px solid var(--line);
    font-size: 16px;
  }
  input {
    flex: 1;
    min-inline-size: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    outline: none;
  }
  .count {
    font-size: 12px;
    color: var(--mut);
    white-space: nowrap;
  }
  ul {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 1px 16px;
    padding: 10px 18px;
    border-block-end: 1px solid var(--line);
    cursor: pointer;
  }
  li.active {
    background: var(--acctint);
  }
  li.none {
    display: block;
    color: var(--mut);
    cursor: default;
  }
  .name {
    font-weight: 600;
  }
  .path {
    font: italic 12.5px var(--serif);
    color: var(--mut);
    align-self: center;
    white-space: nowrap;
  }
  .hint {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--mut);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .keys {
    display: flex;
    gap: 16px;
    padding: 9px 18px;
    font-size: 11.5px;
    color: var(--mut);
  }
</style>
