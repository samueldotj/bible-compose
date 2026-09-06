/**
 * What the pointer is on, for the status bar.
 *
 * Every control the search can land on carries a `data-search-key`, and
 * that is the key the status bar explains — one listener on the document,
 * so a control added later is explained the moment it carries the tag.
 */
export const hover = $state<{ key: string | null }>({ key: null });

/** Watch the pointer. Returns the function that stops watching. */
export function installHoverHelp(): () => void {
  const over = (event: PointerEvent) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const el = target.closest<HTMLElement>("[data-search-key]");
    hover.key = el?.dataset.searchKey ?? null;
  };
  const gone = () => {
    hover.key = null;
  };
  document.addEventListener("pointerover", over);
  document.addEventListener("pointerleave", gone);
  return () => {
    document.removeEventListener("pointerover", over);
    document.removeEventListener("pointerleave", gone);
  };
}
