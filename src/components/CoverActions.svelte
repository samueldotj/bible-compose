<script lang="ts">
  /**
   * The head of the Cover inspector: make `book_cover.pdf`, and say what was
   * made — the sheet, the spine, and where the page count came from, because
   * an estimated spine is the one thing about a cover that must not go to a
   * printer unremarked.
   */
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";

  const made = $derived(session.cover);
</script>

<div class="go">
  <button
    type="button"
    class="btn primary"
    data-search-key="action:make-cover"
    disabled={!session.project || session.makingCover}
    onclick={() => void session.makeCover()}
  >
    {session.makingCover ? t("makingCover") : t("makeCover")}
  </button>
</div>

{#if made}
  <p class="made">
    {phrases().coverMade(made.binding, made.width_in, made.height_in, made.spine_in, made.pages, made.pages_from === "estimate")}
  </p>
  {#each made.diagnostics as d (d.code + d.message)}
    <p class="warn">{d.message}{d.help ? ` — ${d.help}` : ""}</p>
  {/each}
  <div class="links">
    <span class="mono muted path" title={made.path}>{made.path}</span>
    <button type="button" class="link" data-search-key="action:open-cover" onclick={() => void session.showCover()}>{t("openCover")}</button>
  </div>
{:else}
  <p class="muted small">{t("coverIdle")}</p>
{/if}

<style>
  .go {
    margin: 14px 0 10px;
  }
  .made {
    margin: 6px 0;
    font-size: 12.5px;
  }
  .warn {
    margin: 4px 0;
    font-size: 12px;
    color: var(--warn, #9a6a00);
  }
  .links {
    display: flex;
    gap: 10px;
    align-items: baseline;
    margin-block-end: 8px;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-inline-size: 0;
    flex: 1;
    font-size: 11.5px;
  }
  .small {
    font-size: 12px;
  }
</style>
