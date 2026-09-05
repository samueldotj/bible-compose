<script lang="ts">
  /**
   * The editions a project can be started from (P6.2).
   *
   * Each one is a button the width of the row, and no "current preset"
   * anywhere. A preset is *applied* — its settings are written into the
   * project's own file — so after that there is no preset, only settings. A
   * control that kept saying "Large print" after the publisher changed the
   * page size would be claiming something untrue, and this is the tab where
   * they would change it.
   *
   * Choosing one asks first, in a dialog that names the template and says
   * what it will do: applying a preset overwrites a dozen settings at once
   * and there is no undo for a settings file.
   *
   * It has a tab of its own — Template — rather than a place above the page
   * diagram, where it started. A template rewrites a dozen settings across
   * every other tab at once, and a control that does that sitting above the
   * margin fields read as one more margin field.
   */
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";
  import { modal } from "../lib/modal";

  /** The template awaiting confirmation, if any. */
  let confirming = $state<{ id: string; title: string } | null>(null);

  $effect(() => {
    void session.loadPresets();
  });

  async function apply(id: string): Promise<void> {
    confirming = null;
    await session.applyPreset(id);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && confirming !== null) {
      event.preventDefault();
      confirming = null;
    }
  }
</script>

<svelte:window {onkeydown} />

<section class="presets" aria-label={t("editionsRegion")}>
  <h3>{t("startFrom")}</h3>
  <p class="note">{t("presetNote")}</p>

  {#if session.presets === null}
    <p class="note">{t("loading")}</p>
  {:else}
    <ul>
      {#each session.presets as preset (preset.id)}
        <li>
          <button
            type="button"
            class="template"
            disabled={!session.editable}
            onclick={() => (confirming = { id: preset.id, title: preset.title })}
          >
            <strong>{preset.title}</strong>
            <span class="note">{preset.description}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if confirming}
  {@const chosen = confirming}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) confirming = null;
    }}
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label={phrases().startFromTemplate(chosen.title)}
      tabindex="-1"
      use:modal
    >
      <h2>{phrases().startFromTemplate(chosen.title)}</h2>
      <p>{t("overwriteWarning")}</p>
      <footer>
        <button type="button" onclick={() => (confirming = null)}>{t("cancel")}</button>
        <button type="button" class="primary" onclick={() => void apply(chosen.id)}>
          {t("overwriteSettings")}
        </button>
      </footer>
    </div>
  </div>
{/if}

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
    gap: 0.3rem;
  }
  /* The row is the control. A button that looks like a row until the pointer
     is on it, and then like a button: the whole of the title and the sentence
     is the target, since the sentence is what a person reads before choosing. */
  .template {
    display: flex;
    flex-direction: column;
    align-items: start;
    inline-size: 100%;
    padding: 0.5rem 0.7rem;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .template:hover:not(:disabled),
  .template:focus-visible {
    border-color: color-mix(in oklab, currentColor 25%, transparent);
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .template:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .note {
    opacity: 0.7;
    font-size: 0.85em;
    margin: 0;
  }

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
    gap: 0.6rem;
    inline-size: min(28rem, 92vw);
    padding: 1.1rem 1.3rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  .dialog p {
    margin: 0;
    font-size: 0.88rem;
    opacity: 0.85;
  }
  footer {
    display: flex;
    justify-content: end;
    gap: 0.5rem;
    margin-block-start: 0.4rem;
  }
</style>
