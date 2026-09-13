/**
 * The book list's own state and actions, shared by the table in the centre
 * and the inspector beside it (BOOK-003, BOOK-004).
 *
 * "Is Ruth in" and "does John come first" are questions asked *about the
 * book list*, so the answers are written from one place — and the drag in
 * progress lives here too, so the table can show the arrangement being
 * dragged while the inspector keeps describing the one on disk.
 */
import { session } from "./session.svelte";
import type { BookSummary, Testament } from "./services/backend";

export const drag = $state<{ dragging: string | null; pending: string[] | null }>({
  dragging: null,
  pending: null,
});

/** The order shown: the one being dragged, else the project's. */
export function order(): string[] {
  return drag.pending ?? session.books.map((b) => b.code);
}

export function rows(): BookSummary[] {
  return order()
    .map((code) => session.books.find((b) => b.code === code))
    .filter((b): b is BookSummary => b !== undefined);
}

export function included(): Set<string> {
  return new Set(session.books.filter((b) => b.included).map((b) => b.code));
}

export function isCanonical(): boolean {
  const canonical = session.project?.canonicalOrder ?? [];
  return order().join(",") === canonical.join(",");
}

/**
 * The columns, in canonical order of testament, and only the ones this
 * project has books in: an empty column headed "Old Testament" would be
 * furniture for a New Testament edition.
 */
export const TESTAMENTS: readonly Testament[] = ["old", "new", "deuterocanon"];

export function columns(): { id: Testament; books: BookSummary[] }[] {
  const all = rows();
  return TESTAMENTS.map((id) => ({ id, books: all.filter((b) => b.testament === id) })).filter(
    (c) => c.books.length > 0,
  );
}

/**
 * One testament's books, permuted, back into the whole order. The other
 * testaments do not move: each book keeps its *slot* in the full list and
 * only which book sits in which of its own testament's slots changes.
 */
function withGroupReordered(full: string[], group: readonly string[], next: string[]): string[] {
  const inGroup = new Set(group);
  const queue = [...next];
  return full.map((code) => (inGroup.has(code) ? (queue.shift() ?? code) : code));
}

/** Move a book to a position within its own column. */
export function move(code: string, to: number, column: readonly string[]): void {
  const from = column.indexOf(code);
  if (from < 0 || to < 0 || to >= column.length || from === to) return;
  const next = [...column];
  next.splice(from, 1);
  next.splice(to, 0, code);
  drag.pending = withGroupReordered(order(), column, next);
}

/** Written on drop rather than on every hover: each write reopens the project. */
export function commitOrder(): void {
  drag.dragging = null;
  const next = drag.pending;
  drag.pending = null;
  if (next) void session.setBooks(next, included());
}

export function toggle(code: string, on: boolean): void {
  const next = new Set(included());
  if (on) next.add(code);
  else next.delete(code);
  void session.setBooks(order(), next);
}

/** All or nothing, for one testament or the whole list. */
export function selectGroup(codes: readonly string[], on: boolean): void {
  const next = new Set(included());
  for (const code of codes) {
    if (on) next.add(code);
    else next.delete(code);
  }
  void session.setBooks(order(), next);
}

/** Back to the canon's order, leaving the selection alone. */
export function restoreCanonical(): void {
  void session.setBooks([...(session.project?.canonicalOrder ?? [])], included());
}
