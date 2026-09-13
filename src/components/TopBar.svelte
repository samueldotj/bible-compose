<script lang="ts">
  /**
   * The strip along the top: what is open, the way to any setting, what is
   * wrong, and the one button that makes a book.
   *
   * Generate PDF lives here rather than at the foot because it is the
   * control you reach for after changing anything, on any section, and the
   * top right is where a hand goes for that. While a build runs the same
   * button cancels it: only one of the two is ever meaningful, and a
   * disabled Cancel beside an enabled Generate is a thing to read before
   * acting.
   */
  import { languageName } from "../lib/languages";
  import { preferences, setTheme, type Theme } from "../lib/preferences.svelte";
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { phrases, t } from "../lib/i18n";

  const name = $derived.by(() => {
    const set = session.settings.find((s) => s.key === "project.name")?.value.trim();
    if (set) return set;
    const root = session.project?.root ?? "";
    return root.split(/[/\\]/).filter(Boolean).pop() ?? "";
  });
  const language = $derived.by(() => {
    const tag = session.settings.find((s) => s.key === "project.language")?.value.trim() ?? "";
    if (!tag) return "";
    const known = languageName(tag);
    return known ? phrases().languageWithTag(known, tag) : tag;
  });
  const included = $derived(session.books.filter((b) => b.included).length);

  const THEMES: readonly Theme[] = ["system", "light", "dark"];
  const themeWord = $derived(
    preferences.theme === "light" ? t("themeLight") : preferences.theme === "dark" ? t("themeDark") : t("themeSystem"),
  );
  function cycleTheme(): void {
    const at = THEMES.indexOf(preferences.theme);
    setTheme(THEMES[(at + 1) % THEMES.length] ?? "system");
  }
</script>

<header class="top">
  <div class="brand">{t("appName")}</div>

  {#if session.project}
    <div class="who">
      <span class="name">{name}</span>
      <span class="meta">
        {#if language}· {language}{/if}
        · {phrases().booksOf(included, session.books.length)}
      </span>
    </div>
  {/if}

  <span class="spacer"></span>

  {#if session.project}
    <button type="button" class="find" data-search-key="action:find" onclick={() => (ui.palette = true)}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.3-4.3"></path>
      </svg>
      <span>{t("findSetting")}</span>
      <kbd>{t("findShortcut")}</kbd>
    </button>

    <button
      type="button"
      class="problems"
      class:bad={session.errorCount > 0}
      data-search-key="action:problems"
      aria-current={session.pane === "build" ? "true" : undefined}
      onclick={() => (session.pane = "build")}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="10"></circle><path d="M12 8v4"></path><path d="M12 16h.01"></path>
      </svg>
      {phrases().problems(session.diagnostics.length)}
    </button>
  {/if}

  <button
    type="button"
    class="btn icon"
    data-search-key="action:theme"
    title={phrases().themeIs(themeWord)}
    aria-label={phrases().themeIs(themeWord)}
    onclick={cycleTheme}
  >
    {#if preferences.theme === "dark"}
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"></path>
      </svg>
    {:else if preferences.theme === "light"}
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="4"></circle><path d="M12 2v2"></path><path d="M12 20v2"></path><path d="m4.93 4.93 1.41 1.41"></path><path d="m17.66 17.66 1.41 1.41"></path><path d="M2 12h2"></path><path d="M20 12h2"></path><path d="m6.34 17.66-1.41 1.41"></path><path d="m19.07 4.93-1.41 1.41"></path>
      </svg>
    {:else}
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9"></circle><path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor" stroke="none"></path>
      </svg>
    {/if}
  </button>

  {#if session.project}
    {#if session.building}
      <button type="button" class="btn" data-search-key="action:cancel" onclick={() => void session.cancel()}>
        {t("cancelBuild")}
      </button>
    {:else}
      <button
        type="button"
        class="btn primary"
        data-search-key="action:generate"
        disabled={!session.canBuild}
        onclick={() => void session.build()}
      >
        {t("generatePdf")}
      </button>
    {/if}
  {/if}
</header>

<style>
  .top {
    flex: none;
    display: flex;
    align-items: center;
    gap: 18px;
    block-size: var(--top-h);
    padding: 0 20px;
    border-block-end: 1px solid var(--line);
    background: var(--panel);
  }
  .brand {
    font: 500 20px var(--serif);
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
    min-inline-size: 0;
    color: var(--mut);
    white-space: nowrap;
  }
  .name {
    color: var(--ink);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer {
    flex: 1;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    inline-size: 340px;
    max-inline-size: 30vw;
    block-size: 34px;
    padding: 0 12px;
    border: 1px solid var(--line2);
    border-radius: var(--radius-lg);
    background: var(--field);
    color: var(--mut);
    font: inherit;
    cursor: text;
    white-space: nowrap;
  }
  .find:hover {
    border-color: var(--acc);
  }
  .find span {
    flex: 1;
    text-align: start;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  kbd {
    padding: 1px 5px;
    border: 1px solid var(--line);
    border-radius: 4px;
    font: 11px var(--mono);
  }
  .problems {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--mut);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
  }
  .problems:hover,
  .problems[aria-current] {
    background: var(--acctint);
    color: var(--acctext);
  }
  .problems.bad {
    color: var(--err-ink);
    font-weight: 600;
  }
</style>
