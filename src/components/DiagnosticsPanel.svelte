<script lang="ts">
  /**
   * GUI-005, DIA-002, DIA-004: everything wrong, filterable, and clickable
   * back to the book it is about.
   *
   * A blocked build lists every blocking issue at once — which is a property
   * of the orchestrator, not of this panel, and this panel's job is not to
   * hide any of them behind a "first error" summary.
   *
   * A dialog, opened by the button under the book list, rather than a pane
   * standing under it: most of the time there is nothing wrong, and a reserved
   * corner of the left column reading "0" is space taken from the thing being
   * read. When something *is* wrong, a blocked build reports everything at
   * once and that list wants more room than the corner ever had.
   */
  import { session } from "../lib/session.svelte";
  import type { Diagnostic, Severity } from "../lib/services/backend";

  const { onclose }: { onclose: () => void } = $props();

  const FILTERS: readonly (Severity | "all")[] = ["all", "error", "warning", "info"];

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onclose();
    }
  }

  function bookFor(diagnostic: Diagnostic): string | null {
    const path = diagnostic.location?.path;
    if (!path) return null;
    return session.books.find((b) => b.path === path)?.code ?? null;
  }

  function select(diagnostic: Diagnostic): void {
    const code = bookFor(diagnostic);
    if (code) session.selectedBook = code;
  }

  function where(diagnostic: Diagnostic): string {
    const at = diagnostic.location;
    if (!at) return "";
    const file = at.path.split(/[/\\]/).pop() ?? at.path;
    if (at.line === undefined) return file;
    return at.column === undefined ? `${file}:${at.line}` : `${file}:${at.line}:${at.column}`;
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
  <div class="pane" role="dialog" aria-modal="true" aria-labelledby="diagnostics-heading">
    <header>
      <h2 id="diagnostics-heading">
        Problems
        <span class="tally">{session.visibleDiagnostics.length}</span>
      </h2>

      <div class="filters">
        {#each FILTERS as filter (filter)}
          <button
            type="button"
            class:active={session.severity === filter}
            onclick={() => (session.severity = filter)}
          >
            {filter}
          </button>
        {/each}

        <label class="book-only">
          <input type="checkbox" bind:checked={session.bookOnly} />
          selected book only
        </label>
      </div>

    </header>

    {#if session.visibleDiagnostics.length === 0}
      <p class="empty">
        {session.diagnostics.length === 0 ? "Nothing to report." : "Nothing matches the filter."}
      </p>
    {:else}
      <ul>
        {#each session.visibleDiagnostics as diagnostic, i (diagnostic.code + i)}
          <li>
            <button type="button" class="row {diagnostic.severity}" onclick={() => select(diagnostic)}>
              <span class="code">{diagnostic.code}</span>
              <span class="message">{diagnostic.message}</span>
              {#if diagnostic.location}
                <span class="where">{where(diagnostic)}</span>
              {/if}
              {#if diagnostic.help}
                <span class="help">{diagnostic.help}</span>
              {/if}
              {#if diagnostic.detail}
                <!-- DIA-005: backend text is collapsed until it is asked for. -->
                <span class="detail">{diagnostic.detail}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
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
  /* Room for the list, which is why this is a dialog and not the corner it
     used to live in: a blocked build reports everything at once. */
  .pane {
    display: flex;
    flex-direction: column;
    inline-size: min(52rem, 92vw);
    block-size: min(38rem, 85vh);
    padding: 1rem 1.2rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
  }
  .close {
    inline-size: 1.6rem;
    block-size: 1.6rem;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
    opacity: 0.5;
  }
  .close:hover {
    background: color-mix(in oklab, currentColor 12%, transparent);
    opacity: 1;
  }

  header {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    align-items: baseline;
    justify-content: space-between;
  }
  .tally {
    font-weight: 400;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .filters {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    font-size: 0.8rem;
  }
  .filters button {
    padding-block: 0.1rem;
    padding-inline: 0.4rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .filters button.active {
    border-color: color-mix(in oklab, currentColor 30%, transparent);
    background: color-mix(in oklab, currentColor 10%, transparent);
  }
  .book-only {
    display: flex;
    gap: 0.25rem;
    align-items: center;
    opacity: 0.75;
  }
  ul {
    list-style: none;
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    margin: 0.5rem 0 0;
    padding: 0;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.15rem 0.5rem;
    inline-size: 100%;
    padding-block: 0.35rem;
    padding-inline: 0.4rem;
    border: 0;
    border-inline-start: 3px solid transparent;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row:hover {
    background: color-mix(in oklab, currentColor 7%, transparent);
  }
  .row.error {
    border-inline-start-color: #c0392b;
  }
  .row.warning {
    border-inline-start-color: #b8860b;
  }
  .row.info {
    border-inline-start-color: color-mix(in oklab, currentColor 35%, transparent);
  }
  .code {
    font-size: 0.75rem;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .message {
    font-size: 0.88rem;
  }
  .where {
    font-size: 0.75rem;
    opacity: 0.6;
  }
  .help,
  .detail {
    grid-column: 2 / -1;
    font-size: 0.78rem;
    opacity: 0.7;
  }
  .detail {
    white-space: pre-wrap;
    font-family: ui-monospace, monospace;
    opacity: 0.55;
  }
  .empty {
    opacity: 0.65;
  }
</style>
