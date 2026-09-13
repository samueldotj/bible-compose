<script lang="ts">
  /**
   * Beside the gallery: the chosen template, what it does, and the button
   * that applies it — behind a question, because there is no undo.
   *
   * No "current template" is ever claimed. A template is *applied* — its
   * settings are written into the project's own file — so after that there
   * is no template, only settings, and a pane still saying "Large print"
   * after the publisher changed the page size would be saying something
   * untrue.
   */
  import Inspector from "./ui/Inspector.svelte";
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { modal } from "../lib/modal";
  import { phrases, t } from "../lib/i18n";

  const chosen = $derived(session.presets?.find((p) => p.id === ui.template) ?? null);
  let confirming = $state(false);

  async function apply(): Promise<void> {
    if (!chosen) return;
    confirming = false;
    await session.applyPreset(chosen.id);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && confirming) {
      event.preventDefault();
      confirming = false;
    }
  }
</script>

<svelte:window {onkeydown} />

<Inspector
  kicker={t("templateKicker")}
  title={chosen?.title ?? t("chooseTemplate")}
  description={chosen ? undefined : t("chooseTemplateDesc")}
>
  {#if chosen}
    <div class="section-title">{t("whatItDoes")}</div>
    <p class="desc">{chosen.description}</p>
    <p class="muted warn">{t("overwriteWarning")}</p>
    <button
      type="button"
      class="btn primary"
      data-search-key="action:apply-template"
      disabled={!session.editable}
      onclick={() => (confirming = true)}
    >
      {t("applyTemplate")}
    </button>
  {/if}
</Inspector>

{#if confirming && chosen}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) confirming = false;
    }}
  >
    <div class="dialog" role="dialog" aria-modal="true" aria-label={phrases().startFromTemplate(chosen.title)} tabindex="-1" use:modal>
      <h2>{phrases().startFromTemplate(chosen.title)}</h2>
      <p>{t("overwriteWarning")}</p>
      <footer>
        <button type="button" class="btn" onclick={() => (confirming = false)}>{t("cancel")}</button>
        <button type="button" class="btn primary" onclick={() => void apply()}>{t("overwriteSettings")}</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .desc {
    margin: 8px 0 0;
    line-height: 1.5;
  }
  .warn {
    margin: 14px 0 16px;
    font-size: 12px;
    line-height: 1.5;
  }
  .dialog {
    inline-size: min(28rem, 92vw);
  }
  .dialog p {
    margin: 0;
    color: var(--mut);
  }
</style>
