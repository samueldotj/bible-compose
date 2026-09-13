<script lang="ts">
  /**
   * GUI-003: choosing a font from what exists, rather than spelling one.
   *
   * Not the operating system's font dialog, for two reasons. It offers every
   * face installed on this machine with no idea which of them can draw the
   * book — and choosing one that cannot is precisely the mistake FONT-002
   * exists to catch. And it knows nothing about the fonts the *project* ships
   * or the ones the typesetting backend brings with it, which are the two
   * sets a publisher most wants: a font that travels with the book renders
   * the same way on somebody else's machine, and one merely installed here
   * does not.
   *
   * So the list is the same one a build resolves against, in the same order,
   * with the coverage answer beside each name.
   */
  import { untrack } from "svelte";
  import { backend, type FontChoice } from "../lib/services/backend";
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";
  import { modal } from "../lib/modal";

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
  let coveringOnly = $state(true);
  let selected = $state(untrack(() => current));

  $effect(() => {
    void backend()
      .fonts(session.project?.root ?? null)
      .then((list) => (fonts = list))
      .catch((e: unknown) => {
        failure = String(e);
        fonts = [];
      });
  });

  const checked = $derived((fonts ?? []).some((f) => f.missing !== undefined));

  const shown = $derived.by(() => {
    const needle = filter.trim().toLowerCase();
    return (fonts ?? []).filter((f) => {
      if (needle && !f.family.toLowerCase().includes(needle)) return false;
      if (f.family === current) return true;
      if (coveringOnly && checked && (f.missing ?? 0) > 0) return false;
      return true;
    });
  });

  const groups = $derived.by(() => {
    const order = [
      { id: "project", title: t("fontsInProject"), note: t("fontsInProjectNote") },
      { id: "backend", title: t("fontsBundled"), note: t("fontsBundledNote") },
      { id: "system", title: t("fontsInstalled"), note: t("fontsInstalledNote") },
    ] as const;
    return order.map((g) => ({ ...g, rows: shown.filter((f) => f.source === g.id) })).filter((g) => g.rows.length > 0);
  });

  function coverage(font: FontChoice): string {
    if (font.missing === undefined) return "";
    if (font.missing === 0) return t("setsThisScripture");
    return phrases().cannotDraw(font.missing);
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

<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="dialog picker" role="dialog" aria-modal="true" aria-label={t("chooseFont")} tabindex="-1" use:modal>
    <header>
      <h2>{t("chooseFont")}</h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="search" class="input" placeholder={t("searchFonts")} spellcheck="false" autofocus bind:value={filter} />
    </header>

    {#if checked}
      <label class="only">
        <input type="checkbox" bind:checked={coveringOnly} />
        {t("coveringOnly")}
      </label>
    {:else if !session.project}
      <p class="note muted">{t("noProjectToCheckAgainst")}</p>
    {/if}

    <div class="list">
      {#if fonts === null}
        <p class="note muted">{t("readingFonts")}</p>
      {:else if failure}
        <p class="error">{failure}</p>
      {:else if shown.length === 0}
        <p class="note muted">{coveringOnly && checked ? t("nothingMatchesCovering") : t("nothingMatches")}</p>
      {:else}
        {#each groups as group (group.id)}
          <h3 class="kicker">{group.title} <span>— {group.note}</span></h3>
          {#each group.rows as font (font.family)}
            <button
              type="button"
              class="font"
              class:selected={font.family === selected}
              class:short={(font.missing ?? 0) > 0}
              onclick={() => (selected = font.family)}
              ondblclick={confirm}
            >
              <span class="family" style={`font-family: ${JSON.stringify(font.family)}, serif`}>{font.family}</span>
              <span class="coverage">{coverage(font)}</span>
            </button>
          {/each}
        {/each}
      {/if}
    </div>

    <footer>
      <span class="chosen mono muted">{selected}</span>
      <button type="button" class="btn" onclick={onclose}>{t("cancel")}</button>
      <button type="button" class="btn primary" onclick={confirm}>{t("useThisFont")}</button>
    </footer>
  </div>
</div>

<style>
  .picker {
    inline-size: min(34rem, 92vw);
    block-size: min(34rem, 85vh);
  }
  header {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  header .input {
    inline-size: 16rem;
  }
  .only {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12.5px;
  }
  .list {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    border-block: 1px solid var(--line);
  }
  h3 {
    position: sticky;
    inset-block-start: 0;
    margin: 0;
    padding: 10px 0 4px;
    background: var(--panel);
  }
  .font {
    display: flex;
    gap: 10px;
    align-items: baseline;
    justify-content: space-between;
    inline-size: 100%;
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .font:hover {
    background: color-mix(in oklab, var(--acctint) 55%, transparent);
  }
  .font.selected {
    border-color: var(--acc);
    background: var(--acctint);
  }
  .family {
    font-size: 15px;
  }
  .coverage {
    flex: none;
    font-size: 12px;
    color: var(--mut);
  }
  .font.short .coverage {
    color: var(--err-ink);
  }
  footer {
    align-items: center;
  }
  .chosen {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note,
  .error {
    margin: 8px 0;
  }
  .error {
    color: var(--err-ink);
  }
</style>
