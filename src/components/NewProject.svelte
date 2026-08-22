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

  const { onclose }: { onclose: () => void } = $props();

  let parent = $state("");
  let name = $state("");
  let language = $state("en");
  let refused = $state<Diagnostic[]>([]);
  let working = $state(false);

  /** What the folder will be called, which is the name. */
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
  <div class="dialog" role="dialog" aria-modal="true" aria-label="New project">
    <h2>New project</h2>

    <div class="body">
      <label class="field">
        <span>Where it goes</span>
        <span class="pair">
          <input type="text" readonly value={parent} placeholder="Choose a folder…" />
          <button type="button" onclick={() => void choose()}>Browse…</button>
        </span>
      </label>

      <label class="field">
        <span>Publication name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" autofocus bind:value={name} placeholder="My Bible" spellcheck="false" />
      </label>

      <label class="field">
        <span>Language</span>
        <input
          type="text"
          list="bc-languages"
          bind:value={language}
          placeholder="a BCP-47 tag, such as ta"
          spellcheck="false"
        />
        <datalist id="bc-languages">
          {#each LANGUAGES as l (l.tag)}
            <option value={l.tag}>{l.name}</option>
          {/each}
        </datalist>
      </label>

      {#if folder !== "" && parent !== ""}
        <p class="preview">Creates <code>{parent}/{folder}</code></p>
      {/if}

      {#each refused as problem (problem.code + problem.message)}
        <p class="error">{problem.message}{problem.help ? ` — ${problem.help}` : ""}</p>
      {/each}
    </div>

    <footer>
      <button type="button" onclick={onclose}>Cancel</button>
      <button type="button" class="primary" disabled={!ready} onclick={() => void create()}>
        {working ? "Creating…" : "Create"}
      </button>
    </footer>
  </div>
</div>

<style>
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
    /* Half the window each way. The minimums are what keep it usable on a
       small screen, where half of not much is not enough for three fields
       and a pair of buttons. */
    inline-size: 50vw;
    block-size: 50vh;
    min-inline-size: min(24rem, 92vw);
    min-block-size: min(20rem, 90vh);
    padding: 1.1rem 1.3rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
  }
  /* The fields do not grow with it. A text input the width of half a wide
     screen is harder to read than one the width of what goes in it, so the
     dialog gets bigger and its contents stay where the eye can hold them. */
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    flex: 1;
    inline-size: 100%;
    max-inline-size: 26rem;
    overflow-y: auto;
  }
  h2 {
    margin: 0 0 0.8rem;
    font-size: 1rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.82rem;
  }
  .field > span {
    opacity: 0.8;
  }
  .pair {
    display: flex;
    gap: 0.35rem;
  }
  .pair input {
    flex: 1;
    min-inline-size: 0;
  }
  input {
    padding-block: 0.3rem;
    padding-inline: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.9rem;
  }
  input[readonly] {
    opacity: 0.75;
  }
  .preview {
    margin: 0;
    font-size: 0.78rem;
    opacity: 0.65;
  }
  .preview code {
    overflow-wrap: anywhere;
  }
  .error {
    margin: 0;
    font-size: 0.82rem;
    color: #c0392b;
  }
  footer {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    padding-block-start: 0.3rem;
  }
  button {
    padding: 0.3rem 0.7rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  button.primary {
    border-color: transparent;
    background: color-mix(in oklab, currentColor 20%, transparent);
    font-weight: 600;
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
