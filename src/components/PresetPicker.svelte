<script lang="ts">
  /**
   * The editions a project can be started from (P6.2).
   *
   * Buttons rather than a dropdown, and no "current preset" anywhere. A preset
   * is *applied* — its settings are written into the project's own file — so
   * after that there is no preset, only settings. A control that kept saying
   * "Large print" after the publisher changed the page size would be claiming
   * something untrue, and this is the tab where they would change it.
   *
   * It has a tab of its own — Template — rather than a place above the page
   * diagram, where it started. A template rewrites a dozen settings across
   * every other tab at once, and a control that does that sitting above the
   * margin fields read as one more margin field.
   */
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";

  let confirming = $state<string | null>(null);

  $effect(() => {
    void session.loadPresets();
  });

  async function apply(id: string): Promise<void> {
    confirming = null;
    await session.applyPreset(id);
  }
</script>

<section class="presets" aria-label={t("editionsRegion")}>
  <h3>{t("startFrom")}</h3>
  <p class="note">{t("presetNote")}</p>

  {#if session.presets === null}
    <p class="note">{t("loading")}</p>
  {:else}
    <ul>
      {#each session.presets as preset (preset.id)}
        <li>
          <div class="what">
            <strong>{preset.title}</strong>
            <span class="note">{preset.description}</span>
          </div>
          {#if confirming === preset.id}
            <!--
              One confirmation, because applying a preset overwrites a dozen
              settings at once and there is no undo for a settings file. The
              button says what it will do rather than "OK".
            -->
            <span class="confirm">
              <button type="button" class="primary" onclick={() => void apply(preset.id)}>
                {t("overwriteSettings")}
              </button>
              <button type="button" onclick={() => (confirming = null)}>{t("cancel")}</button>
            </span>
          {:else}
            <button
              type="button"
              disabled={!session.editable}
              onclick={() => (confirming = preset.id)}
            >
              {t("use")}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .presets {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-block-end: 1rem;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  li {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
  }
  .what {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }
  .note {
    opacity: 0.7;
    font-size: 0.85em;
    margin: 0;
  }
  .confirm {
    display: flex;
    gap: 0.4rem;
    flex: none;
  }
  button {
    flex: none;
  }
</style>
