<script lang="ts">
  /**
   * Starting a project: which folder, and what language it is in (PRJ-001).
   *
   * Two questions and no more. The folder chosen *is* the project — usually it
   * already holds the Scripture — and the publication is named after it.
   * Everything else has a built-in answer, so asking about page size or fonts
   * here would be asking somebody to decide, before they have seen a page,
   * things they can change at any time afterwards.
   *
   * The folder is looked at as soon as it is chosen. One that already has a
   * `biblecompose.toml` is a project, so it is refused with an offer to open
   * it; one with files other than USFM is warned about and still allowed.
   *
   * The language is a text field with a list behind it rather than a closed
   * dropdown. `project.language` takes any BCP-47 tag, and a publisher setting
   * a language that is not in anybody's list is the ordinary case in this
   * field — a dropdown would be the application telling them their language
   * does not exist. The list offers names and `resolveLanguage` turns the one
   * that was picked back into its tag, so the field can be searched the way a
   * person thinks of their language rather than the way the file stores it.
   */
  import { LANGUAGES, resolveLanguage } from "../lib/languages";
  import { backend, type Diagnostic, type FolderCheck } from "../lib/services/backend";
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";
  import { modal } from "../lib/modal";

  const { onclose }: { onclose: () => void } = $props();

  let folder = $state("");
  let check = $state<FolderCheck | null>(null);
  let language = $state("English");
  let refused = $state<Diagnostic[]>([]);
  let working = $state(false);

  const ready = $derived(folder !== "" && check !== null && !check.isProject && !working);

  async function choose(): Promise<void> {
    const chosen = await backend().chooseFolder();
    if (!chosen) return;
    folder = chosen;
    check = null;
    refused = [];
    check = await backend().checkProjectFolder(chosen);
  }

  async function create(): Promise<void> {
    if (!ready) return;
    working = true;
    refused = await session.create(folder, resolveLanguage(language));
    working = false;
    if (refused.length === 0) onclose();
  }

  async function openInstead(): Promise<void> {
    onclose();
    await session.open(folder);
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
        <span class="kicker">{t("projectFolder")}</span>
        <span class="pair">
          <input type="text" class="input" readonly value={folder} placeholder={t("browse")} />
          <!-- svelte-ignore a11y_autofocus -->
          <button type="button" class="btn small" autofocus onclick={() => void choose()}>{t("browse")}</button>
        </span>
      </label>

      {#if check?.isProject}
        <p class="error">{t("alreadyAProject")}</p>
      {:else if check && check.otherFiles > 0}
        <p class="warning">{t("otherFilesWarning").replace("{n}", String(check.otherFiles))}</p>
      {/if}

      <label class="field">
        <span class="kicker">{t("language")}</span>
        <input type="text" class="input" list="bc-languages" bind:value={language} placeholder={t("languageTagHint")} spellcheck="false" />
        <!--
          The name is the option's *value*, because that is what the browser
          filters its suggestions on: with the tag there, typing "Hindi"
          matched nothing and the only way to reach a language was to scroll
          the whole list. `resolveLanguage` turns the name back into the tag on
          the way out, so what reaches `biblecompose.toml` is still BCP-47 and
          a publisher who knows their tag can still type it.
        -->
        <datalist id="bc-languages">
          {#each LANGUAGES as l (l.tag)}
            <option value={l.name} label={l.tag}></option>
          {/each}
        </datalist>
      </label>

      {#each refused as problem (problem.code + problem.message)}
        <p class="error">{problem.message}{problem.help ? ` — ${problem.help}` : ""}</p>
      {/each}
    </div>

    <footer>
      <button type="button" class="btn" onclick={onclose}>{t("cancel")}</button>
      {#if check?.isProject}
        <button type="button" class="btn primary" onclick={() => void openInstead()}>{t("openInstead")}</button>
      {:else}
        <button type="button" class="btn primary" disabled={!ready} onclick={() => void create()}>
          {working ? t("creating") : t("create")}
        </button>
      {/if}
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
  .warning {
    margin: 0;
    font-size: 12.5px;
    color: var(--warn-ink);
  }
  .error {
    margin: 0;
    font-size: 12.5px;
    color: var(--err-ink);
  }
</style>
