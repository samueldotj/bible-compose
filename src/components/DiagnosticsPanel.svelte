<script lang="ts">
  /**
   * GUI-005, DIA-002, DIA-004: everything wrong, filterable, and clickable
   * back to the book it is about.
   *
   * A blocked build lists every blocking issue at once — which is a property
   * of the orchestrator, not of this panel, and this panel's job is not to
   * hide any of them behind a "first error" summary.
   */
  import { session } from "../lib/session.svelte";
  import type { Diagnostic, Severity } from "../lib/services/backend";

  const FILTERS: readonly (Severity | "all")[] = ["all", "error", "warning", "info"];

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

<section class="pane" aria-labelledby="diagnostics-heading">
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
</section>

<style>
  header {
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
