/**
 * Points to the unit a person is looking at, and back.
 *
 * The backend parses `0.55in`, `39.6pt` and `13.97mm` into points and sends
 * the page as numbers; the Trim & margins inspector shows them in whichever
 * unit is chosen and writes back in that unit, so what is typed is what the
 * file keeps. No second unit parser here: this only formats a number the
 * backend already resolved, and appends a unit to one a person typed.
 */
import type { Unit } from "./ui.svelte";

const PER_POINT: Record<Unit, number> = { pt: 1, in: 1 / 72, mm: 25.4 / 72 };

/** Points, as a number in `unit`, to two decimals with the noise dropped. */
export function fromPoints(points: number, unit: Unit): string {
  const value = points * PER_POINT[unit];
  const digits = unit === "pt" ? 1 : 2;
  return String(Number(value.toFixed(digits)));
}

/** What to write for a number typed in `unit`. A bare number gets the unit. */
export function withUnit(typed: string, unit: Unit): string {
  const text = typed.trim();
  if (text === "") return text;
  return /^[0-9.]+$/.test(text) ? `${text}${unit}` : text;
}

/** `7 × 10 in`, from two lengths in points. */
export function pair(width: number, height: number, unit: Unit): string {
  return `${fromPoints(width, unit)} × ${fromPoints(height, unit)} ${unit}`;
}
