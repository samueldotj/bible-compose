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
  import type { Testament } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";

  let dragging = $state<string | null>(null);
  /** The arrangement being dragged, before it is committed on drop. */
  let pending = $state<string[] | null>(null);

  const order = $derived(pending ?? session.books.map((b) => b.code));
  const rows = $derived(
    order.map((code) => session.books.find((b) => b.code === code)).filter((b) => b !== undefined),
  );
  const included = $derived(new Set(session.books.filter((b) => b.included).map((b) => b.code)));

  /**
   * The columns, in canonical order of testament, and only the ones this
   * project has books in.
   *
   * A New Testament on its own is an ordinary publication and so is a Bible
   * with deuterocanonical books; an empty column headed "Old Testament" would
   * be furniture in the first case, and a book with nowhere to go would be a
   * book you could not reach in the second.
   */
  const TESTAMENTS: readonly { id: Testament; title: string }[] = [
    { id: "old", title: "Old Testament" },
    { id: "new", title: "New Testament" },
    { id: "deuterocanon", title: "Deuterocanonical" },
  ];

  const columns = $derived(
    TESTAMENTS.map((t) => ({
      ...t,
      books: rows.filter((b) => b.testament === t.id),
    })).filter((c) => c.books.length > 0),
  );

  /**
   * One testament's books, permuted, back into the whole order.
   *
   * **The other testaments do not move.** The two columns are a view of a
   * single `books.order`, and the honest way to show any order in two columns
   * is to keep each book's *slot* in the full list and change only which book
   * sits in which of its own testament's slots. A project that prints the New
   * Testament first, or interleaves, keeps that arrangement while its Gospels
   * are being reordered.
   */
  function withGroupReordered(full: string[], group: readonly string[], next: string[]): string[] {
    const inGroup = new Set(group);
    const queue = [...next];
    return full.map((code) => (inGroup.has(code) ? (queue.shift() ?? code) : code));
  }

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

  /**
   * Move a book to a position *within its own column*.
   *
   * `column` is the codes that column shows, so a book dragged from one
   * testament to another finds itself absent from the target's list and
   * nothing happens — which is the right answer: a book's testament is the
   * canon's to say and not a publisher's.
   */
  function move(code: string, to: number, column: readonly string[]): void {
    const from = column.indexOf(code);
    if (from < 0 || to < 0 || to >= column.length || from === to) return;
    const next = [...column];
    next.splice(from, 1);
    next.splice(to, 0, code);
    pending = withGroupReordered(order, column, next);
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
   * The same, for one testament.
   *
   * A New Testament edition is one of the commonest things anybody makes with
   * this, and making one from a whole Bible was thirty-nine clicks. It leaves
   * the other columns exactly as they are — which is the point, and is why
   * this is not the pair above with a filter on it.
   */
  function selectGroup(codes: readonly string[], on: boolean): void {
    const next = new Set(included);
    for (const code of codes) {
      if (on) next.add(code);
      else next.delete(code);
    }
    void session.setBooks(order, next);
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

<!-- No heading of its own: the tab above it says Scripture, and each column
     says which testament it is. A third title between them would be a label
     for a thing already labelled twice. -->
<section class="pane" aria-label={t("booksRegion")}>
  {#if !session.project}
    <p class="empty">{t("noProjectOpen")}</p>
  {:else if session.books.length === 0}
    <p class="empty">{t("noUsfmHere")}</p>
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
          onclick={() => selectAll(true)}>{t("selectAll")}</button
        >
        <button
          type="button"
          disabled={!session.editable || included.size === 0}
          onclick={() => selectAll(false)}>{t("clearAll")}</button
        >
        <button
          type="button"
          disabled={!session.editable || isCanonical}
          title={t("canonicalOrderHint")}
          onclick={restoreCanonical}>{t("canonicalOrder")}</button
        >
      </span>
    </p>
    <!-- Side by side, because the two testaments are two lists and a reader
         looking for Habakkuk should not have to scroll past the Gospels. One
         column each, each scrolling on its own so a long Old Testament does
         not decide how much of the New is visible. -->
    <div class="testaments">
      {#each columns as column (column.id)}
        {@const codes = column.books.map((b) => b.code)}
        {@const chosen = codes.filter((c) => included.has(c)).length}
        <section class="testament" aria-label={column.title}>
          <h3>
            <span class="what">
              {column.title}
              <span class="tally">{chosen} of {column.books.length}</span>
            </span>
            <!-- Only where there is more than one column. With a single
                 testament these would be the pair above it, twice. -->
            {#if columns.length > 1}
              <span class="tools">
                <button
                  type="button"
                  disabled={!session.editable || chosen === codes.length}
                  title={`Put every book of the ${column.title} in the publication`}
                  onclick={() => selectGroup(codes, true)}>{t("selectAll")}</button
                >
                <button
                  type="button"
                  disabled={!session.editable || chosen === 0}
                  title={`Take every book of the ${column.title} out`}
                  onclick={() => selectGroup(codes, false)}>{t("clearAll")}</button
                >
              </span>
            {/if}
          </h3>
          <ul>
            {#each column.books as book, i (book.code)}
              <li
                class:dragging={dragging === book.code}
                class:out={!book.included}
                draggable={session.editable}
                ondragstart={(e) => {
                  dragging = book.code;
                  // Firefox starts no drag at all without payload, and the move
                  // cursor is the difference between "this will reorder" and
                  // "this will copy something somewhere".
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
                  // The reorder happens on enter rather than on over:
                  // `dragover` fires continuously while the pointer sits still,
                  // and moving the row under it each time makes the list
                  // flicker between two arrangements.
                  if (dragging && dragging !== book.code) move(dragging, i, codes);
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
                  aria-label={phrases().includeBook(book.code)}
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
                  <span class="chapters">
                    {book.included ? `${book.chapters} ch` : "not included"}
                  </span>
                  {#if book.errors > 0}
                    <span class="count error">
                      {book.errors} error{book.errors === 1 ? "" : "s"}
                    </span>
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
                    aria-label={phrases().moveEarlier(book.code)}
                    onclick={() => {
                      move(book.code, i - 1, codes);
                      commitOrder();
                    }}>↑</button
                  >
                  <button
                    type="button"
                    disabled={!session.editable || i === column.books.length - 1}
                    aria-label={phrases().moveLater(book.code)}
                    onclick={() => {
                      move(book.code, i + 1, codes);
                      commitOrder();
                    }}>↓</button
                  >
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</section>

<style>
  /* The pane fills its column so the list can. */
  .pane {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
  }
  .hint {
    flex: none;
  }
  /* Equal columns whatever their length, so the two testaments read as two
     lists of the same kind of thing rather than as a big one and a small one.
     They wrap to a single column when the window cannot give each a usable
     measure. */
  .testaments {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(19rem, 1fr));
    gap: 0 1.4rem;
    flex: 1;
    min-block-size: 0;
  }
  .testament {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
    min-inline-size: 0;
  }
  h3 {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    justify-content: space-between;
    flex: none;
    margin: 0 0 0.25rem;
    padding-block-end: 0.2rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    font-size: 0.78rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.7;
  }
  h3 .what {
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    min-inline-size: 0;
  }
  h3 .tally {
    font-weight: 400;
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
  }
  /* Lower case and un-tracked, against the heading they sit on: they are
     things to do, not part of its name. */
  h3 .tools {
    text-transform: none;
    letter-spacing: normal;
    font-weight: 400;
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
