<script lang="ts">
  /**
   * GUI-001 and BOOK-003/004: the books in the project, each with its own
   * validation status — and, on the same rows, whether it is in the
   * publication and where it goes.
   *
   * There was a dialog for that. It listed the same books in the same order,
   * a click away from the list already on screen, which is one list too many:
   * "is Ruth in" and "does John come first" are things you ask *about the book
   * list*, so they belong on it.
   *
   * The counts come from the shell, not from filtering the diagnostics here,
   * so this pane and the diagnostics panel cannot disagree about which book
   * owns a problem.
   */
  import { session } from "../lib/session.svelte";

  let dragging = $state<string | null>(null);
  /** The arrangement being dragged, before it is committed on drop. */
  let pending = $state<string[] | null>(null);

  const order = $derived(pending ?? session.books.map((b) => b.code));
  const rows = $derived(
    order.map((code) => session.books.find((b) => b.code === code)).filter((b) => b !== undefined),
  );
  const included = $derived(new Set(session.books.filter((b) => b.included).map((b) => b.code)));

  /**
   * The canon's own order, from the shell — the canon table is Rust's and
   * mirroring sixty-six rows into TypeScript would be a second copy of it,
   * kept where it cannot be checked against the first.
   */
  const canonical = $derived(session.project?.canonicalOrder ?? []);
  const isCanonical = $derived(order.join(",") === canonical.join(","));

  function status(book: { errors: number; warnings: number }): "error" | "warning" | "ok" {
    if (book.errors > 0) return "error";
    if (book.warnings > 0) return "warning";
    return "ok";
  }

  function move(code: string, to: number): void {
    const from = order.indexOf(code);
    if (from < 0 || to < 0 || to >= order.length || from === to) return;
    const next = [...order];
    next.splice(from, 1);
    next.splice(to, 0, code);
    pending = next;
  }

  /**
   * Written on drop rather than on every hover: each write reopens the
   * project, and doing that for every row a book is dragged past would
   * rewrite the settings file a dozen times to record one move.
   */
  function commitOrder(): void {
    dragging = null;
    const next = pending;
    pending = null;
    if (next) void session.setBooks(next, included);
  }

  function toggle(code: string, on: boolean): void {
    const next = new Set(included);
    if (on) next.add(code);
    else next.delete(code);
    void session.setBooks(order, next);
  }

  /**
   * All or nothing.
   *
   * Clearing everything is allowed and is not the same as the default: an
   * empty `books.include` is a project that has deliberately selected nothing,
   * which the build reports as having no books to compose. Refusing the click
   * would be the window deciding a publisher cannot be in a half-finished
   * state on the way to picking three books.
   */
  function selectAll(on: boolean): void {
    void session.setBooks(order, on ? new Set(order) : new Set());
  }

  /**
   * Back to the canon's order.
   *
   * This *clears* `books.order` rather than writing the canonical sequence
   * out, because `setBooks` recognises the arrangement as the default one —
   * so the button undoes the setting rather than pinning it to a value that
   * happens to match today. It leaves the selection alone: which books are in
   * and what order they come in are two decisions, and a control that reset
   * both would be one nobody could use for either.
   */
  function restoreCanonical(): void {
    void session.setBooks([...canonical], included);
  }
</script>

<section class="pane" aria-labelledby="books-heading">
  <h2 id="books-heading">Books</h2>

  {#if !session.project}
    <p class="empty">No project open.</p>
  {:else if session.books.length === 0}
    <p class="empty">This folder has no USFM in it.</p>
  {:else}
    <p class="hint">
      <span>
        {included.size === session.books.length
          ? `All ${session.books.length} in the publication`
          : `${included.size} of ${session.books.length} in the publication`}
      </span>
      <span class="tools">
        <button
          type="button"
          disabled={!session.editable || included.size === session.books.length}
          onclick={() => selectAll(true)}>Select all</button
        >
        <button
          type="button"
          disabled={!session.editable || included.size === 0}
          onclick={() => selectAll(false)}>Clear all</button
        >
        <button
          type="button"
          disabled={!session.editable || isCanonical}
          title="Put the books back in the order the canon gives them"
          onclick={restoreCanonical}>Canonical order</button
        >
      </span>
    </p>
    <ul>
      {#each rows as book, i (book.code)}
        <li
          class:dragging={dragging === book.code}
          class:out={!book.included}
          draggable={session.editable}
          ondragstart={(e) => {
            dragging = book.code;
            // Firefox starts no drag at all without payload, and the move
            // cursor is the difference between "this will reorder" and "this
            // will copy something somewhere".
            e.dataTransfer?.setData("text/plain", book.code);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
          }}
          ondragend={commitOrder}
          ondragover={(e) => {
            // Without this the drop is refused and `dragend` reports a
            // cancelled drag, whatever the rest of the handlers do.
            e.preventDefault();
            if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
          }}
          ondragenter={() => {
            // The reorder happens on enter rather than on over: `dragover`
            // fires continuously while the pointer sits still, and moving the
            // row under it each time makes the list flicker between two
            // arrangements.
            if (dragging && dragging !== book.code) move(dragging, i);
          }}
          ondrop={(e) => {
            e.preventDefault();
            commitOrder();
          }}
        >
          <span class="grip" aria-hidden="true">⋮⋮</span>
          <input
            type="checkbox"
            checked={book.included}
            disabled={!session.editable}
            aria-label={`Include ${book.code}`}
            onchange={(e) => toggle(book.code, e.currentTarget.checked)}
          />
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
            <span class="chapters">{book.included ? `${book.chapters} ch` : "not included"}</span>
            {#if book.errors > 0}
              <span class="count error">{book.errors} error{book.errors === 1 ? "" : "s"}</span>
            {:else if book.warnings > 0}
              <span class="count warning">
                {book.warnings} warning{book.warnings === 1 ? "" : "s"}
              </span>
            {/if}
          </button>
          <span class="nudge">
            <button
              type="button"
              disabled={!session.editable || i === 0}
              aria-label={`Move ${book.code} earlier`}
              onclick={() => {
                move(book.code, i - 1);
                commitOrder();
              }}>↑</button
            >
            <button
              type="button"
              disabled={!session.editable || i === rows.length - 1}
              aria-label={`Move ${book.code} later`}
              onclick={() => {
                move(book.code, i + 1);
                commitOrder();
              }}>↓</button
            >
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  /* The pane fills its column so the list can. */
  .pane {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
  }
  h2,
  .hint {
    flex: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    /* Whatever the column has left after the heading and the summary. A
       whole Bible is sixty-six rows and a Gospel is four; a fixed height was
       either too much for one or too little for the other. */
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  li {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    border: 1px solid transparent;
    border-radius: 4px;
  }
  li.dragging {
    border-color: color-mix(in oklab, currentColor 35%, transparent);
    opacity: 0.6;
  }
  .hint {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    justify-content: space-between;
    margin: 0 0 0.3rem;
    font-size: 0.78rem;
    opacity: 0.75;
  }
  .tools {
    display: flex;
    gap: 0.5rem;
    flex: none;
  }
  .tools button {
    border: 0;
    padding: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .tools button:disabled {
    text-decoration: none;
    cursor: default;
    opacity: 0.35;
  }
  .grip {
    cursor: grab;
    opacity: 0.35;
    font-size: 0.8rem;
    letter-spacing: -0.15em;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 0.5rem;
    align-items: baseline;
    flex: 1;
    min-inline-size: 0;
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
  /* Still listed, still orderable — just not in the book. */
  li.out .name,
  li.out .code,
  li.out .chapters {
    opacity: 0.45;
    text-decoration: line-through;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .nudge {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
  }
  .nudge button {
    inline-size: 1.2rem;
    block-size: 0.85rem;
    padding: 0;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 3px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.6rem;
    line-height: 1;
    cursor: pointer;
  }
  .nudge button:disabled {
    opacity: 0.2;
    cursor: default;
  }
  .empty {
    opacity: 0.65;
  }
</style>
