<script lang="ts">
  /**
   * Which books are in the publication, and in what order — one list.
   *
   * They were two dialogs over the same books, which is one dialog too many:
   * "leave out Ruth" and "put John first" are two things a publisher does to
   * the same list in the same sitting, and making them open two pickers to do
   * it is asking them to hold the list in their head twice.
   *
   * So: a tick for whether a book is in, a position for where it goes, and
   * both written on Apply. They stay two settings in the file — `books.order`
   * and `books.include` are different facts and a project may set one without
   * the other — but they are one question on screen.
   *
   * Neither is written when the answer is the default. Dragging a book back to
   * where the canon puts it clears `books.order` rather than writing an
   * explicit copy of the built-in order; ticking every book clears
   * `books.include` rather than listing all sixty-six. A settings file should
   * record what a publisher decided, not what they left alone.
   */
  import { untrack } from "svelte";
  import { session } from "../lib/session.svelte";

  const {
    order,
    include,
    onapply,
    onclose,
  }: {
    /** `books.order` as it stands — comma-separated codes, empty when unset. */
    order: string;
    /** `books.include`, likewise. Empty means every book found. */
    include: string;
    /** Each is a new value, or `null` to reset that setting to its default. */
    onapply: (next: { order: string | null; include: string | null }) => void;
    onclose: () => void;
  } = $props();

  interface Row {
    readonly code: string;
    readonly name: string;
    readonly present: boolean;
  }

  function codes(text: string): string[] {
    return text
      .split(",")
      .map((s) => s.trim().toUpperCase())
      .filter((s) => s !== "");
  }

  /**
   * The books to show: the project's own, in the order the build would put
   * them, plus any the settings name that are not on disk.
   *
   * The strays are shown rather than dropped because a project may configure a
   * whole Bible and build one Gospel — which the build already reports as
   * "configured but absent" — and a picker that silently deleted them from the
   * settings would turn opening a dialog into an edit.
   */
  const rows: Row[] = untrack(() => {
    const present = session.books.map((b) => ({ code: b.code, name: b.name, present: true }));
    const known = new Set(present.map((b) => b.code));
    const strays = [...codes(order), ...codes(include)]
      .filter((c, i, all) => !known.has(c) && all.indexOf(c) === i)
      .map((code) => ({ code, name: "not in this folder", present: false }));
    return [...present, ...strays];
  });

  /** The order the build would use with `books.order` unset. */
  const canonical = rows.map((r) => r.code);

  /**
   * The working state, as it stood when the dialog opened. From then on it is
   * the person's — a fresh component each time, so nothing stale carries over.
   */
  let arranged = $state<string[]>(
    untrack(() => {
      const named = codes(order).filter((c) => canonical.includes(c));
      return [...named, ...canonical.filter((c) => !named.includes(c))];
    }),
  );
  let ticked = $state<Set<string>>(
    untrack(() => new Set(include.trim() === "" ? canonical : codes(include))),
  );

  let dragging = $state<string | null>(null);

  const nameOf = (code: string) => rows.find((r) => r.code === code)?.name ?? code;
  const isStray = (code: string) => rows.find((r) => r.code === code)?.present === false;

  const isCanonical = $derived(arranged.join(",") === canonical.join(","));
  const allTicked = $derived(ticked.size === rows.length);

  function move(code: string, to: number): void {
    const from = arranged.indexOf(code);
    if (from < 0 || to < 0 || to >= arranged.length || from === to) return;
    const next = [...arranged];
    next.splice(from, 1);
    next.splice(to, 0, code);
    arranged = next;
  }

  function toggle(code: string, on: boolean): void {
    const next = new Set(ticked);
    if (on) next.add(code);
    else next.delete(code);
    ticked = next;
  }

  function apply(): void {
    onapply({
      order: isCanonical ? null : arranged.join(", "),
      // In the arranged order rather than the order they were ticked, so the
      // two settings read as one decision when somebody opens the file.
      include: allTicked ? null : arranged.filter((c) => ticked.has(c)).join(", "),
    });
    onclose();
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
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Books">
    <header>
      <h2>Books</h2>
      <span class="hint">Tick to include, drag to reorder</span>
    </header>

    <div class="tools">
      <button
        type="button"
        class="link"
        onclick={() => (ticked = new Set(allTicked ? [] : canonical))}
      >
        {allTicked ? "Clear all" : "Select all"}
      </button>
      <button type="button" class="link" disabled={isCanonical} onclick={() => (arranged = [...canonical])}>
        Canonical order
      </button>
    </div>

    <div class="list">
      {#if rows.length === 0}
        <p class="note">This folder has no books in it yet.</p>
      {:else}
        {#each arranged as code, i (code)}
          <!--
            Draggable, and also movable with buttons. Drag is what a person
            reaches for; the buttons are what makes it usable with a keyboard,
            on a trackpad, and in a list longer than the dialog.
          -->
          <div
            class="row"
            class:dragging={dragging === code}
            class:out={!ticked.has(code)}
            draggable="true"
            role="listitem"
            ondragstart={() => (dragging = code)}
            ondragend={() => (dragging = null)}
            ondragover={(e) => {
              e.preventDefault();
              if (dragging && dragging !== code) move(dragging, i);
            }}
          >
            <span class="grip" aria-hidden="true">⋮⋮</span>
            <input
              type="checkbox"
              checked={ticked.has(code)}
              aria-label={`Include ${code}`}
              onchange={(e) => toggle(code, e.currentTarget.checked)}
            />
            <span class="position">{ticked.has(code) ? i + 1 : "—"}</span>
            <span class="code">{code}</span>
            <span class="name" class:stray={isStray(code)}>{nameOf(code)}</span>
            <span class="nudge">
              <button
                type="button"
                disabled={i === 0}
                aria-label={`Move ${code} earlier`}
                onclick={() => move(code, i - 1)}>↑</button
              >
              <button
                type="button"
                disabled={i === arranged.length - 1}
                aria-label={`Move ${code} later`}
                onclick={() => move(code, i + 1)}>↓</button
              >
            </span>
          </div>
        {/each}
      {/if}
    </div>

    <footer>
      <span class="summary">
        {allTicked ? `Every book — ${rows.length}` : `${ticked.size} of ${rows.length}`},
        {isCanonical ? "canonical order" : "custom order"}
      </span>
      <button type="button" onclick={onclose}>Cancel</button>
      <button type="button" class="primary" onclick={apply}>Apply</button>
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
    inline-size: min(32rem, 90vw);
    block-size: min(32rem, 85vh);
    padding: 0.9rem 1rem 0.8rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
  }
  header {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  .hint {
    font-size: 0.78rem;
    opacity: 0.6;
  }
  .tools {
    display: flex;
    gap: 0.9rem;
    padding-block: 0.4rem;
  }
  .link {
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.8rem;
    text-decoration: underline;
    cursor: pointer;
    opacity: 0.75;
  }
  .link:disabled {
    text-decoration: none;
    cursor: default;
    opacity: 0.35;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding-inline-end: 0.2rem;
    border-block: 1px solid color-mix(in oklab, currentColor 15%, transparent);
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.25rem 0.35rem;
    border: 1px solid transparent;
    border-radius: 4px;
    font-size: 0.88rem;
  }
  .row:hover {
    background: color-mix(in oklab, currentColor 7%, transparent);
  }
  .row.dragging {
    border-color: color-mix(in oklab, currentColor 35%, transparent);
    opacity: 0.6;
  }
  /* Still in the list and still orderable — just not in the book. */
  .row.out .code,
  .row.out .name,
  .row.out .position {
    opacity: 0.4;
    text-decoration: line-through;
  }
  .grip {
    cursor: grab;
    opacity: 0.4;
    letter-spacing: -0.15em;
  }
  .position {
    inline-size: 1.6rem;
    font-size: 0.75rem;
    text-align: end;
    opacity: 0.5;
  }
  .code {
    inline-size: 3rem;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.8;
  }
  .name.stray {
    font-style: italic;
    opacity: 0.55;
  }
  .nudge {
    display: flex;
    gap: 0.15rem;
  }
  .nudge button {
    inline-size: 1.4rem;
    padding: 0;
    border: 1px solid color-mix(in oklab, currentColor 22%, transparent);
    border-radius: 3px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .nudge button:disabled {
    opacity: 0.25;
    cursor: default;
  }
  footer {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding-block-start: 0.7rem;
  }
  .summary {
    flex: 1;
    font-size: 0.8rem;
    opacity: 0.7;
  }
  footer button {
    padding: 0.28rem 0.7rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  footer button.primary {
    border-color: transparent;
    background: color-mix(in oklab, currentColor 20%, transparent);
    font-weight: 600;
  }
  .note {
    margin: 0.5rem 0;
    font-size: 0.85rem;
    opacity: 0.7;
  }
</style>
