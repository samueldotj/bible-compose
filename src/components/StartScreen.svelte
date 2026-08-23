<script lang="ts">
  /**
   * What the window shows before a project is open (PRJ-001).
   *
   * It used to be a sentence explaining that a folder of USFM will build. True,
   * and no help at all to somebody who has done this before: the folder they
   * want is one they opened yesterday, and finding it again meant a folder
   * picker and a memory of where they put it.
   *
   * So: the projects this machine has opened, and the two ways to arrive at
   * one that is not on the list.
   */
  import NewProject from "./NewProject.svelte";
  import { session } from "./../lib/session.svelte";

  let starting = $state(false);

  /** The folder, for the line under the name. */
  function where(root: string): string {
    const parts = root.split(/[/\\]/).filter(Boolean);
    parts.pop();
    return parts.join("/");
  }
</script>

<section class="start">
  <div class="actions">
    <button type="button" class="primary" onclick={() => void session.choose()}>
      Open a project…
    </button>
    <button type="button" onclick={() => (starting = true)}>New project…</button>
  </div>

  <p class="lede">
    A project is a folder of USFM. Anything it does not configure uses the built-in defaults, so a
    folder with nothing but Scripture in it will build.
  </p>

  {#if session.recent.length > 0}
    <h2>Recent</h2>
    <ul>
      {#each session.recent as item (item.root)}
        <li class:missing={item.missing}>
          <button
            type="button"
            class="row"
            disabled={item.missing}
            title={item.root}
            onclick={() => void session.open(item.root)}
          >
            <span class="name">{item.name}</span>
            <span class="where">{item.missing ? "no longer there" : where(item.root)}</span>
          </button>
          <!-- Forgetting is deliberate and never deletes anything. A row that
               removed itself when the folder went missing would leave somebody
               wondering whether they had imagined it. -->
          <button
            type="button"
            class="forget"
            title="Remove from this list — the folder is not touched"
            aria-label={`Forget ${item.name}`}
            onclick={() => void session.forget(item.root)}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if session.versions}
    <p class="versions">{session.versions.app} · contract {session.versions.contract}</p>
  {/if}
</section>

{#if starting}
  <NewProject onclose={() => (starting = false)} />
{/if}

<style>
  .start {
    /* The only thing on screen, so it takes what there is and scrolls a long
       list inside it rather than growing the window. */
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    inline-size: 100%;
    max-inline-size: 34rem;
    margin-inline: auto;
    padding-block: 2rem;
    padding-inline: 1rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .lede {
    margin-block: 0.9rem 1.6rem;
    font-size: 0.9rem;
    opacity: 0.7;
  }
  h2 {
    margin: 0 0 0.4rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.6;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    border-block-end: 1px solid color-mix(in oklab, currentColor 10%, transparent);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 1;
    min-inline-size: 0;
    padding-block: 0.5rem;
    padding-inline: 0.4rem;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row:hover:not(:disabled) {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .row:disabled {
    cursor: default;
  }
  li.missing .name {
    text-decoration: line-through;
    opacity: 0.55;
  }
  .name {
    font-size: 0.95rem;
  }
  .where {
    overflow: hidden;
    font-size: 0.76rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.55;
  }
  .forget {
    flex: none;
    inline-size: 1.4rem;
    block-size: 1.4rem;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    opacity: 0.4;
  }
  .versions {
    margin-block-start: 2rem;
    font-size: 0.72rem;
    opacity: 0.45;
  }
  .forget:hover {
    background: color-mix(in oklab, currentColor 12%, transparent);
    opacity: 1;
  }
  button.primary {
    border-color: transparent;
    background: color-mix(in oklab, currentColor 20%, transparent);
    font-weight: 600;
  }
  .actions button {
    padding: 0.4rem 0.9rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.9rem;
    cursor: pointer;
  }
</style>
