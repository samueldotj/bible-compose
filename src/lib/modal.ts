/**
 * What a dialog has to do for a keyboard (NFR-011).
 *
 * Three things, and none of them is `role="dialog"` — the three dialogs in
 * this window already had the role, the `aria-modal` and the Escape key, and
 * were still not operable without a mouse:
 *
 * * **Focus goes in.** A dialog that opens while focus stays on the button
 *   behind it leaves a keyboard user tabbing through a page they can no longer
 *   see, and a screen reader reading it.
 * * **Focus stays in.** `aria-modal` tells assistive technology that the rest
 *   of the page is inert. It does not tell the browser, which will happily
 *   tab out of the dialog and into the form behind it.
 * * **Focus comes back.** Closing a dialog without restoring focus drops it on
 *   `<body>`, and the next Tab starts again from the top of the window.
 *
 * One action rather than three copies, because these are exactly the rules
 * that get two of three dialogs right.
 */

/** Everything that can hold focus, in document order. */
const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

function focusable(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    // `offsetParent` is null for anything `display: none`, which is the cheap
    // and reliable half of "visible". A dialog does not hide its own controls
    // any other way.
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
}

/**
 * Use on the element with `role="dialog"`.
 *
 * ```svelte
 * <div role="dialog" aria-modal="true" use:modal>
 * ```
 */
export function modal(node: HTMLElement): { destroy(): void } {
  // Before anything is moved, so there is something to go back to.
  const opener = document.activeElement as HTMLElement | null;

  // After the first paint: Svelte runs an action before the children it
  // wraps are necessarily laid out, and `autofocus` on an input inside has
  // not fired yet either. Deferring also means a dialog that autofocuses
  // something specific keeps it — this only steps in when nothing else did.
  queueMicrotask(() => {
    if (node.contains(document.activeElement)) return;
    const first = focusable(node)[0];
    (first ?? node).focus();
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== "Tab") return;
    const targets = focusable(node);
    if (targets.length === 0) {
      event.preventDefault();
      return;
    }
    const first = targets.at(0);
    const last = targets.at(-1);
    if (!first || !last) return;
    const here = document.activeElement;

    // Wrap at both ends. Checking `here` rather than trusting the browser,
    // because focus can be outside the dialog entirely — it opened, something
    // else took focus, and Tab would then walk the page behind it.
    if (event.shiftKey && (here === first || !node.contains(here))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (here === last || !node.contains(here))) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener("keydown", onkeydown);
  // Capture on the window as well, so a Tab pressed while focus has escaped
  // still comes back. The listener above only sees keys inside the dialog.
  window.addEventListener("keydown", onkeydown, true);

  return {
    destroy() {
      node.removeEventListener("keydown", onkeydown);
      window.removeEventListener("keydown", onkeydown, true);
      // Only if focus is still somewhere in here or nowhere at all. If the
      // dialog closed *because* something else took focus, taking it back
      // would be the second surprise rather than a fix.
      const here = document.activeElement;
      if (opener?.isConnected && (here === null || here === document.body || node.contains(here))) {
        opener.focus();
      }
    },
  };
}

/** The element a dialog with nothing focusable in it can hold focus on. */
export const DIALOG_TABINDEX = -1;
