<script lang="ts">
  /**
   * The table of contents down the left: every section of the window,
   * grouped the way a publisher thinks about a book — which Scripture, then
   * the text, then the page, then the type, then the output.
   *
   * Styles unfolds into its own entries when it is open: eight groups of
   * elements and an inspector over all of them are nouns rather than steps,
   * and a column leaves them left-aligned, reading as a list of what can be
   * styled.
   */
  import { NAV, STYLE_TABS, TABS, navTitle, subTabTitle, tabTitle } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";
</script>

<nav class="rail" aria-label={t("settingsRegion")}>
  {#each NAV as group (group.id)}
    <div class="kicker group">{navTitle(group.id)}</div>
    {#each TABS.filter((x) => x.nav === group.id) as tab (tab.id)}
      <button
        type="button"
        class="entry"
        class:active={session.pane === tab.id}
        data-search-key={`tab:${tab.id}`}
        aria-current={session.pane === tab.id ? "page" : undefined}
        onclick={() => (session.pane = tab.id)}
      >
        <span>{tabTitle(tab)}</span>
        {#if tab.id === "build" && session.diagnostics.length > 0}
          <span class="badge count" class:bad={session.errorCount > 0}>{session.diagnostics.length}</span>
        {/if}
      </button>
      {#if tab.styles && session.pane === "styles"}
        <div class="sub">
          {#each STYLE_TABS as sub (sub.id)}
            <button
              type="button"
              class="entry small"
              class:active={session.stylePane === sub.id}
              data-search-key={`subtab:${sub.id}`}
              aria-current={session.stylePane === sub.id ? "true" : undefined}
              onclick={() => (session.stylePane = sub.id)}
            >
              {subTabTitle(sub)}
            </button>
          {/each}
        </div>
      {/if}
    {/each}
  {/each}

  <div class="foot">
    <button type="button" class="entry" data-search-key="action:close" onclick={() => void session.close()}>
      {t("closeProject")}
    </button>
    <p class="hint">{t("railHint")}</p>
  </div>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    flex: none;
    gap: 1px;
    inline-size: var(--rail-w);
    min-block-size: 0;
    padding: 14px 12px;
    border-inline-end: 1px solid var(--line);
    background: var(--panel);
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .group {
    padding: 14px 10px 5px;
  }
  .group:first-child {
    padding-block-start: 6px;
  }
  .entry {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    inline-size: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--ink);
    font: inherit;
    text-align: start;
    white-space: nowrap;
    cursor: pointer;
  }
  .entry:hover {
    background: color-mix(in oklab, var(--acctint) 55%, transparent);
  }
  .entry.active {
    background: var(--acctint);
    color: var(--acctext);
    font-weight: 600;
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 2px 0 0 14px;
  }
  .entry.small {
    padding: 5px 10px;
    font-size: 12.5px;
  }
  .badge.bad {
    background: var(--err-bg);
    color: var(--err-ink);
  }
  .foot {
    margin-block-start: auto;
    padding-block-start: 12px;
  }
  .hint {
    margin: 4px 0 0;
    padding: 6px 10px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--mut);
    white-space: normal;
  }
</style>
