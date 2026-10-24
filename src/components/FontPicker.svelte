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
   *
   * # And what to do when none of them will do
   *
   * The list above has a dead end in it. A translator whose machine has no
   * face for their script sees every entry marked as unable to draw the book —
   * correct, and no help at all. FONT-006 is the way out, and it sits at the
   * top of this dialog rather than in a settings pane somewhere, because this
   * is where somebody is standing when they find out they have a problem.
   *
   * A suggestion is openly licensed and gets fetched into `assets/fonts/`,
   * which is the detail that makes it worth doing at all: a font in the
   * project travels with the book (FONT-003), so the page sets the same way on
   * the machine of whoever prints it. A font merely installed here does not.
   */
  import { untrack } from "svelte";
  import {
    asDiagnostics,
    backend,
    type Diagnostic,
    type FontChoice,
    type FontSuggestion,
  } from "../lib/services/backend";
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

  let suggestions = $state<readonly FontSuggestion[]>([]);
  /** The family being fetched, so only its own row shows the wait. */
  let fetching = $state<string | null>(null);
  let refused = $state<Diagnostic[]>([]);
  let added = $state<string | null>(null);

  const language = $derived(
    session.settings.find((s) => s.key === "project.language")?.value ?? "",
  );

  async function load(): Promise<void> {
    const root = session.project?.root ?? null;
    fonts = await backend().fonts(root);
    // A dialog that cannot say what the machine has is broken; one that cannot
    // say what could be fetched is merely less useful, so a failure here is
    // swallowed rather than replacing the list with an error.
    suggestions = await backend()
      .fontSuggestions(root, language)
      .catch(() => []);
  }

  $effect(() => {
    void load().catch((e: unknown) => {
      failure = String(e);
      fonts = [];
    });
  });

  /**
   * Fetch one family and select it, without choosing it.
   *
   * Two steps on purpose. Downloading a font and changing what the book is set
   * in are different decisions, and a publisher looking at four families
   * should be able to fetch one and look at it in the list — with its real
   * coverage count beside it — before committing the page to it.
   */
  async function add(suggestion: FontSuggestion): Promise<void> {
    const root = session.project?.root;
    if (!root || fetching) return;

    fetching = suggestion.family;
    refused = [];
    added = null;
    try {
      const packaged = await backend().installFont(root, suggestion.family);
      added = phrases().fontAdded(packaged.family, packaged.missing);
      selected = packaged.family;
      await load();
    } catch (e: unknown) {
      refused = asDiagnostics(e);
    } finally {
      fetching = null;
    }
  }

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

  /**
   * The suggestions worth showing: only with a project open, because there is
   * nowhere to put a font without one, and an offer that cannot be taken is
   * worse than no offer.
   */
  const offered = $derived.by(() => {
    if (!session.project) return [];
    const needle = filter.trim().toLowerCase();
    return suggestions.filter((s) => !needle || s.family.toLowerCase().includes(needle));
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
      {#if offered.length > 0}
        <h3 class="kicker">{t("fontsToFetch")} <span>— {t("fontsToFetchNote")}</span></h3>
        {#each offered as suggestion (suggestion.family)}
          <div class="font offer" class:selected={suggestion.family === selected}>
            <span class="family" style={`font-family: ${JSON.stringify(suggestion.family)}, serif`}>
              {suggestion.family}
            </span>
            <span class="why muted">
              {suggestion.script} · {phrases().faces(suggestion.faces)} · {suggestion.note}
            </span>
            {#if suggestion.packaged}
              <span class="coverage">{t("inThisProjectAlready")}</span>
            {:else}
              <button
                type="button"
                class="btn small"
                disabled={fetching !== null}
                onclick={() => void add(suggestion)}
              >
                {fetching === suggestion.family
                  ? t("fetchingFont")
                  : `${t("addToProject")} · ${phrases().downloadSize(suggestion.bytes)}`}
              </button>
            {/if}
          </div>
        {/each}

        {#if added}
          <p class="note added">{added}</p>
        {/if}
        {#each refused as problem (problem.code + problem.message)}
          <p class="error">{problem.message}{problem.help ? ` — ${problem.help}` : ""}</p>
        {/each}
        <p class="note muted offline">{t("fetchNeedsConnection")}</p>
      {/if}

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
  /* A suggestion is a row of three parts rather than two: the name, why it is
     being suggested, and the one control. It does not become selected by being
     clicked — the whole row is not a button, because the button in it does
     something a click should never do by accident. */
  .font.offer {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 10px;
    cursor: default;
  }
  .font.offer:hover {
    background: transparent;
  }
  .font.offer .why {
    grid-column: 1;
    grid-row: 2;
    font-size: 12px;
  }
  .font.offer .btn,
  .font.offer .coverage {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    white-space: nowrap;
  }
  .added {
    color: var(--acc);
  }
  .offline {
    font-size: 12px;
    padding-block-end: 4px;
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
