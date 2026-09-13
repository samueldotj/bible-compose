<script lang="ts">
  /**
   * The editions a project can be started from (P6.2), as a gallery.
   *
   * Each card is a page schematic — one column or two — with the name and a
   * sentence. Clicking a card chooses it; the inspector beside the gallery
   * says what it does and holds the button that applies it, because
   * applying one overwrites a dozen settings at once and there is no undo
   * for a settings file.
   */
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n";

  $effect(() => {
    void session.loadPresets();
  });

  /** Whether the schematic shows two columns — read off the name, which is all a preset tells us. */
  const twoColumn = (title: string) => /two|reference|study/i.test(title);
</script>

<section class="templates" aria-label={t("editionsRegion")}>
  <div class="head">
    <h2>{t("startFrom")}</h2>
    <span class="muted">{t("presetNote")}</span>
  </div>
  {#if session.presets === null}
    <p class="muted">{t("loading")}</p>
  {:else}
    <div class="grid">
      {#each session.presets as preset (preset.id)}
        <button
          type="button"
          class="card"
          class:chosen={ui.template === preset.id}
          data-search-key={`preset:${preset.id}`}
          aria-pressed={ui.template === preset.id}
          onclick={() => (ui.template = preset.id)}
        >
          <span class="sheet" class:two={twoColumn(preset.title)} aria-hidden="true"><span></span><span></span></span>
          <span class="title">{preset.title}</span>
          <span class="desc">{preset.description}</span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .templates {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 16px;
    min-inline-size: 0;
    min-block-size: 0;
    padding: 22px 28px;
    overflow-y: auto;
  }
  .head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 14px;
  }
  h2 {
    font: 500 26px/1.1 var(--serif);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 14px;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
    min-block-size: 230px;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .card:hover {
    border-color: var(--line2);
  }
  .card.chosen {
    border: 2px solid var(--acc);
    background: var(--acctint);
    padding: 15px;
  }
  .sheet {
    display: grid;
    grid-template-columns: 1fr;
    gap: 4px;
    inline-size: 62px;
    block-size: 84px;
    padding: 10px 8px 14px;
    border: 1px solid var(--line2);
    border-radius: 2px;
    background: var(--paper);
    box-shadow: 0 2px 6px rgba(40, 30, 10, 0.1);
  }
  .sheet span {
    background: var(--line);
  }
  .sheet span:last-child {
    display: none;
  }
  .sheet.two {
    grid-template-columns: 1fr 1fr;
  }
  .sheet.two span:last-child {
    display: block;
  }
  .title {
    font: 500 17px/1.2 var(--serif);
  }
  .desc {
    flex: 1;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--mut);
  }
</style>
