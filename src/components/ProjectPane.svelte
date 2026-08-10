<script lang="ts">
  /**
   * GUI-001: the books in the project, each with its own validation status.
   *
   * The counts come from the shell, not from filtering the diagnostics here,
   * so this pane and the diagnostics panel cannot disagree about which book
   * owns a problem.
   */
  import { session } from "../lib/session.svelte";

  function status(book: { errors: number; warnings: number }): "error" | "warning" | "ok" {
    if (book.errors > 0) return "error";
    if (book.warnings > 0) return "warning";
    return "ok";
  }
</script>

<section class="pane" aria-labelledby="books-heading">
  <h2 id="books-heading">Books</h2>

  {#if !session.project}
    <p class="empty">No project open.</p>
  {:else if session.books.length === 0}
    <p class="empty">This folder has no USFM in it.</p>
  {:else}
    <ul>
      {#each session.books as book (book.code)}
        <li>
          <button
            type="button"
            class="row"
            class:selected={session.selectedBook === book.code}
            aria-current={session.selectedBook === book.code ? "true" : undefined}
            onclick={() => (session.selectedBook = book.code)}
          >
            <span class="dot {status(book)}" aria-hidden="true"></span>
            <span class="name">{book.name}</span>
            <span class="code">{book.code}</span>
            <span class="chapters">{book.chapters} ch</span>
            {#if book.errors > 0}
              <span class="count error">{book.errors} error{book.errors === 1 ? "" : "s"}</span>
            {:else if book.warnings > 0}
              <span class="count warning">
                {book.warnings} warning{book.warnings === 1 ? "" : "s"}
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  /* A whole Bible is sixty-six rows, and the pane sits above the diagnostics
     panel — so the list scrolls rather than pushing everything else off the
     window. Tall enough that a Gospel-sized project never scrolls at all. */
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-block-size: 22rem;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 0.5rem;
    align-items: baseline;
    inline-size: 100%;
    padding-block: 0.3rem;
    padding-inline: 0.4rem;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .row.selected {
    background: color-mix(in oklab, currentColor 14%, transparent);
  }
  .dot {
    inline-size: 0.5rem;
    block-size: 0.5rem;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.35;
  }
  .dot.error {
    background: #c0392b;
    opacity: 1;
  }
  .dot.warning {
    background: #b8860b;
    opacity: 1;
  }
  .code,
  .chapters {
    font-size: 0.8rem;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .count {
    grid-column: 2 / -1;
    font-size: 0.78rem;
  }
  .count.error {
    color: #c0392b;
  }
  .count.warning {
    color: #8a6100;
  }
  .empty {
    opacity: 0.65;
  }
</style>
