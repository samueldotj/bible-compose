/**
 * Light up the thing a search found.
 *
 * Every control the search can land on carries a `data-search-key`, and
 * this finds it once the tab it is on has rendered, scrolls it into view,
 * gives it the focus if it can take one, and marks it for a few seconds so
 * the eye lands where the pointer would have.
 *
 * The lookup is by attribute rather than by a registry the components sign
 * into, so a control added later is reachable the moment it carries the
 * attribute — the same bargain the index makes with the schema.
 */
import { tick } from "svelte";

/** The key most recently lit, so a test or a page can ask. */
export const spotlight = $state<{ key: string | null }>({ key: null });

const LINGER_MS = 3000;
const ATTEMPTS = 20;
const BETWEEN_MS = 100;

/**
 * Find the element for `key` and light it. Waits for it, briefly: a tab
 * switch renders on the next tick, and the settings a form shows may still
 * be on their way from the backend.
 */
export async function reveal(key: string): Promise<boolean> {
  spotlight.key = key;
  await tick();
  for (let attempt = 0; attempt < ATTEMPTS; attempt++) {
    const el = document.querySelector<HTMLElement>(`[data-search-key="${CSS.escape(key)}"]`);
    if (el) {
      el.scrollIntoView({ block: "center", behavior: "smooth" });
      el.classList.add("spotlit");
      window.setTimeout(() => el.classList.remove("spotlit"), LINGER_MS);
      const control = el.matches("input, select, button, textarea")
        ? el
        : el.querySelector<HTMLElement>("input:not([disabled]), select:not([disabled]), button:not([disabled])");
      control?.focus({ preventScroll: true });
      return true;
    }
    await new Promise((resolve) => window.setTimeout(resolve, BETWEEN_MS));
  }
  return false;
}
