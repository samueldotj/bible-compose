<script lang="ts">
  /**
   * Starting a project: where it goes, what it is called, what language it is
   * in (PRJ-001).
   *
   * Three questions and no more. Everything else about a publication has a
   * built-in answer, so asking about page size or fonts here would be asking
   * somebody to decide, before they have seen a page, things they can change
   * at any time afterwards.
   *
   * The language is a text field with a list behind it rather than a closed
   * dropdown. `project.language` takes any BCP-47 tag, and a publisher setting
   * a language that is not in anybody's list is the ordinary case in this
   * field — a dropdown would be the application telling them their language
   * does not exist.
   */
  import { LANGUAGES } from "../lib/languages";
  import { backend, type Diagnostic } from "../lib/services/backend";
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";
  import { modal } from "../lib/modal";

  const { onclose }: { onclose: () => void } = $props();

  let parent = $state("");
  let name = $state("");
  let language = $state("en");
  let refused = $state<Diagnostic[]>([]);
  let working = $state(false);

  const folder = $derived(name.trim());
  const ready = $derived(parent !== "" && folder !== "" && !working);

  async function choose(): Promise<void> {
    const chosen = await backend().chooseFolder();
    if (chosen) parent = chosen;
  }

  async function create(): Promise<void> {
    if (!ready) return;
    working = true;
    refused = await session.create(parent, folder, language.trim());
    working = false;
    if (refused.length === 0) onclose();
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
  <div class="dialog" role="dialog" aria-modal="true" aria-label={t("newProject")} tabindex="-1" use:modal>
    <h2>{t("newProject")}</h2>

    <div class="body">
      <label class="field">
        <span class="kicker">{t("where")}</span>
        <span class="pair">
          <input type="text" class="input" readonly value={parent} placeholder={t("browse")} />
          <button type="button" class="btn small" onclick={() => void choose()}>{t("browse")}</button>
        </span>
      </label>

      <label class="field">
        <span class="kicker">{t("publicationName")}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" class="input" autofocus dir="auto" bind:value={name} spellcheck="false" />
      </label>

      <label class="field">
        <span class="kicker">{t("language")}</span>
        <input type="text" class="input" list="bc-languages" bind:value={language} placeholder={t("languageTagHint")} spellcheck="false" />
        <datalist id="bc-languages">
          {#each LANGUAGES as l (l.tag)}
            <option value={l.tag}>{l.name}</option>
          {/each}
        </datalist>
      </label>

      {#if folder !== "" && parent !== ""}
        <p class="preview muted">{t("creates")}<span class="mono">{parent}/{folder}</span></p>
      {/if}

      {#each refused as problem (problem.code + problem.message)}
        <p class="error">{problem.message}{problem.help ? ` — ${problem.help}` : ""}</p>
      {/each}
    </div>

    <footer>
      <button type="button" class="btn" onclick={onclose}>{t("cancel")}</button>
      <button type="button" class="btn primary" disabled={!ready} onclick={() => void create()}>
        {working ? t("creating") : t("create")}
      </button>
    </footer>
  </div>
</div>

<style>
  .dialog {
    inline-size: min(30rem, 92vw);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .field .input {
    inline-size: 100%;
  }
  .pair {
    display: flex;
    gap: 6px;
  }
  .pair .input {
    flex: 1;
  }
  .preview {
    margin: 0;
    font-size: 12px;
  }
  .preview .mono {
    overflow-wrap: anywhere;
  }
  .error {
    margin: 0;
    font-size: 12.5px;
    color: var(--err-ink);
  }
</style>
