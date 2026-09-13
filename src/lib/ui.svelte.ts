/**
 * How the window is being looked at, as distinct from what it holds.
 *
 * The session is the project; this is the view of it — which setting the
 * pointer is on so the page can show what it governs, whether the spread
 * shows one page or two, whether the palette is open. None of it is saved:
 * a window reopens with the spread fitted and nothing lit, which is the
 * right first sight of any project.
 */
import { TABS } from "./labels";
import { homeOf } from "./search";
import { session } from "./session.svelte";
import { reveal } from "./spotlight.svelte";
import { STYLE_GROUPS } from "./styles";

export type Unit = "in" | "mm" | "pt";

export const ui = $state<{
  /** The setting the pointer is on — a row, or the thing on the page it governs. */
  lit: string | null;
  /** Both pages of the spread, or the right-hand one alone. */
  spread: boolean;
  /** Fitted to the room it has, or at a fixed size with room to scroll. */
  fit: boolean;
  /** Whether the palette is open. */
  palette: boolean;
  /** The style the Styles section is showing, as its selector. */
  style: string | null;
  /** The template a click chose, awaiting Apply. */
  template: string | null;
  /** The unit Trim & margins shows its measurements in. */
  unit: Unit;
  /**
   * Whether the right page's slots follow the left page's, mirrored.
   * `null` until somebody decides: the answer is then read off the slots.
   */
  mirror: boolean | null;
}>({
  lit: null,
  spread: true,
  fit: true,
  palette: false,
  style: null,
  template: null,
  unit: "in",
  mirror: null,
});

/**
 * Go to a setting: the section that owns it, its entry of Styles when it
 * has one, and then the row itself, lit for a moment.
 *
 * What a click on the page does, and what the palette does: the two ways
 * of arriving at a setting share one road.
 */
export function goToSetting(key: string): void {
  const home = homeOf(key);
  if (home?.tab && TABS.some((x) => x.id === home.tab)) session.pane = home.tab;
  if (home?.subtab) session.stylePane = home.subtab;
  void reveal(key);
}

/**
 * The style the Styles section is showing: the chosen one when it belongs
 * to the open group, else the group's first — and nothing when the open
 * entry is Typography or Inspect, which have no one element.
 */
export function shownStyle(): string | null {
  if (session.pane !== "styles") return null;
  const group = STYLE_GROUPS.find((g) => g.id === session.stylePane);
  if (!group) return null;
  if (ui.style && group.rows.some((r) => r.selector === ui.style)) return ui.style;
  return group.rows[0]?.selector ?? null;
}

/** Go to a style: the Styles section, its group, and the style itself. */
export function goToStyle(selector: string, group: string): void {
  session.pane = "styles";
  session.stylePane = group;
  ui.style = selector;
  void reveal(`style:${selector}`);
}
