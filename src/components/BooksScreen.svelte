<script lang="ts">
  /**
   * GUI-001 and BOOK-003/004: the books in the project, each with a switch
   * saying whether it is in the publication and a handle to move it.
   *
   * Two columns, because the two testaments are two lists and a reader
   * looking for Habakkuk should not have to scroll past the Gospels. Each
   * column scrolls on its own so a long Old Testament does not decide how
   * much of the New is visible. A filter box, because sixty-six rows is a
   * list to search rather than to read.
   */
  import Toggle from "./ui/Toggle.svelte";
  import { columns, drag, commitOrder, included, move, selectGroup, toggle } from "../lib/books.svelte";
  import { session } from "../lib/session.svelte";
  import type { Testament } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";

  let filter = $state("");
  const needle = $derived(filter.trim().toLowerCase());
  const matches = (b: { name: string; code: string }) =>
    needle === "" || b.name.toLowerCase().includes(needle) || b.code.toLowerCase().includes(needle);

  const titleOf = (id: Testament) =>
    id === "old" ? t("oldTestament") : id === "new" ? t("newTestament") : t("deuterocanon");

  function status(book: { errors: number; warnings: number }): "error" | "warning" | "ok" {
    if (book.errors > 0) return "error";
    if (book.warnings > 0) return "warning";
    return "ok";
  }
</script>

<section class="books" aria-label={t("booksRegion")}>
  <div class="head">
    <h2>{t("booksRegion")}</h2>
    <span class="muted">
      {phrases().inPublicationOf(included().size, session.books.length)}
    </span>
    <span class="spacer"></span>
    <input
      type="search"
      class="input filter"
      placeholder={t("filterBooks")}
      aria-label={t("filterBooks")}
      spellcheck="false"
      bind:value={filter}
    />
  </div>

  {#if session.books.length === 0}
    <p class="muted">{t("noUsfmHere")}</p>
  {:else}
    <div class="testaments">
      {#each columns() as column (column.id)}
        {@const codes = column.books.map((b) => b.code)}
        {@const chosen = codes.filter((c) => included().has(c)).length}
        <section class="testament" aria-label={titleOf(column.id)}>
          <div class="colhead">
            <span class="title">{titleOf(column.id)}</span>
            <span class="muted tally">{chosen} / {column.books.length}</span>
            <span class="spacer"></span>
            <button
              type="button"
              class="link"
              data-search-key="action:include-all"
              disabled={!session.editable || chosen === codes.length}
              onclick={() => selectGroup(codes, true)}>{t("includeAll")}</button
            >
            <button
              type="button"
              class="link"
              data-search-key="action:clear"
              disabled={!session.editable || chosen === 0}
              onclick={() => selectGroup(codes, false)}>{t("clear")}</button
            >
          </div>
          <ul>
            {#each column.books as book, i (book.code)}
              <li
                class:dragging={drag.dragging === book.code}
                class:out={!book.included}
                class:selected={session.selectedBook === book.code}
                class:hidden={!matches(book)}
                draggable={session.editable}
                ondragstart={(e) => {
                  drag.dragging = book.code;
                  e.dataTransfer?.setData("text/plain", book.code);
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
                }}
                ondragend={commitOrder}
                ondragover={(e) => {
                  e.preventDefault();
                  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
                }}
                ondragenter={() => {
                  if (drag.dragging && drag.dragging !== book.code) move(drag.dragging, i, codes);
                }}
                ondrop={(e) => {
                  e.preventDefault();
                  commitOrder();
                }}
              >
                <span class="grip" aria-hidden="true">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.8"></circle><circle cx="9" cy="12" r="1.8"></circle><circle cx="9" cy="19" r="1.8"></circle><circle cx="15" cy="5" r="1.8"></circle><circle cx="15" cy="12" r="1.8"></circle><circle cx="15" cy="19" r="1.8"></circle></svg>
                </span>
                <Toggle
                  checked={book.included}
                  disabled={!session.editable}
                  label={phrases().includeBook(book.code)}
                  onchange={(on) => toggle(book.code, on)}
                />
                <button
                  type="button"
                  class="row"
                  aria-current={session.selectedBook === book.code ? "true" : undefined}
                  onclick={() => (session.selectedBook = book.code)}
                >
                  <span class="dot {status(book)}" aria-hidden="true"></span>
                  <span class="name">{book.name}</span>
                  <span class="mono code">{book.code}</span>
                  <span class="muted chapters">
                    {book.included ? phrases().chapters(book.chapters) : "—"}
                  </span>
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
  .books {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 14px;
    min-inline-size: 0;
    min-block-size: 0;
    padding: 22px 28px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 14px;
  }
  h2 {
    font: 500 26px/1.1 var(--serif);
  }
  .spacer {
    flex: 1;
  }
  .filter {
    inline-size: 240px;
    block-size: 32px;
  }
  .testaments {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 0 28px;
    min-block-size: 0;
  }
  .testament {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
    min-inline-size: 0;
  }
  .colhead {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding-block-end: 8px;
    border-block-end: 1px solid var(--line2);
    font-size: 12px;
  }
  .title {
    font: italic 500 16px var(--serif);
    color: var(--ink);
  }
  .tally {
    font-variant-numeric: tabular-nums;
  }
  ul {
    flex: 1;
    min-block-size: 0;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  li {
    display: grid;
    grid-template-columns: 14px 32px 1fr auto;
    gap: 12px;
    align-items: center;
    block-size: 40px;
    border-block-end: 1px solid var(--line);
  }
  li.hidden {
    display: none;
  }
  li.dragging {
    opacity: 0.5;
  }
  li.selected {
    background: color-mix(in oklab, var(--acctint) 60%, transparent);
  }
  .grip {
    display: flex;
    color: var(--line2);
    cursor: grab;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr 44px 92px;
    gap: 10px;
    align-items: center;
    min-inline-size: 0;
    padding: 0 4px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .dot {
    inline-size: 6px;
    block-size: 6px;
    border-radius: 50%;
    background: var(--line2);
  }
  .dot.error {
    background: var(--err);
  }
  .dot.warning {
    background: var(--warn);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  li.out .name {
    font-weight: 400;
    color: var(--mut);
  }
  .code {
    color: var(--mut);
  }
  .chapters {
    font-size: 12px;
    text-align: end;
    white-space: nowrap;
  }
  .nudge {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .nudge button {
    inline-size: 20px;
    block-size: 14px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 3px;
    background: transparent;
    color: var(--mut);
    font: 9px/1 var(--sans);
    cursor: pointer;
  }
  .nudge button:disabled {
    opacity: 0.25;
    cursor: default;
  }
</style>
