/**
 * The twelve head and foot slots, and the one decision that ties six of them
 * to the other six.
 *
 * A left-hand page and a right-hand page are mirror images: the outer slot
 * of one is its left, of the other its right. Most books want the two sides
 * to say the same things in the same places, so the right page can *follow*
 * the left, mirrored — and while it does, writing a left slot writes its
 * twin on the right, swapped so outer stays outer.
 */
import { session } from "./session.svelte";
import { ui } from "./ui.svelte";

export const LINES = ["header", "footer"] as const;
export const POSITIONS = ["left", "center", "right"] as const;
export type Line = (typeof LINES)[number];
export type Position = (typeof POSITIONS)[number];

export function slotKey(side: "left" | "right", line: Line, position: Position): string {
  return `headers.${side}_page.${line}_${position}`;
}

/** The right-page twin of a left-page slot: the same line, mirrored. */
export function twin(key: string): string | null {
  const m = /^headers\.left_page\.(header|footer)_(left|center|right)$/.exec(key);
  if (!m) return null;
  const line = m[1] as Line;
  const position = m[2] as Position;
  const mirrored: Position = position === "left" ? "right" : position === "right" ? "left" : "center";
  return slotKey("right", line, mirrored);
}

function valueOf(key: string): string {
  return session.settings.find((s) => s.key === key)?.value ?? "";
}

/** Whether every right slot already reads as the mirror of its left twin. */
export function readsMirrored(): boolean {
  for (const line of LINES) {
    for (const position of POSITIONS) {
      const left = slotKey("left", line, position);
      const right = twin(left);
      if (right && valueOf(left) !== valueOf(right)) return false;
    }
  }
  return true;
}

/** What the mirror switch shows: what somebody chose, else what the slots say. */
export function mirrored(): boolean {
  return ui.mirror ?? readsMirrored();
}

/** Write one slot — and its twin, while the right page follows the left. */
export async function setSlot(key: string, value: string): Promise<void> {
  // Decided before the write: afterwards the two sides disagree by
  // construction, and the answer would always be no.
  const follow = mirrored();
  await session.setSetting(key, value);
  const other = twin(key);
  if (other && follow && valueOf(other) !== value) {
    await session.setSetting(other, value);
  }
}

/** Turn following on, copying the left page across; or off, changing nothing. */
export async function setMirrored(on: boolean): Promise<void> {
  ui.mirror = on;
  if (!on) return;
  for (const line of LINES) {
    for (const position of POSITIONS) {
      const left = slotKey("left", line, position);
      const right = twin(left);
      if (right && valueOf(left) !== valueOf(right)) {
        await session.setSetting(right, valueOf(left));
      }
    }
  }
}
