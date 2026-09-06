/**
 * How the window looks to this person: the colour theme and the zoom.
 *
 * Preferences of the machine rather than of the project — a publisher who
 * likes a dark window likes it for every Bible — so they live in the
 * browser's storage and not in `biblecompose.toml`, and a project opened on
 * another machine looks the way that machine's owner likes.
 *
 * Both are applied to the document root: the theme as `color-scheme`, which
 * is what every colour in the stylesheets is written against (`Canvas`,
 * `CanvasText`, and mixes of the two), and the zoom as CSS `zoom`, which
 * scales the whole page the way a browser's own zoom does.
 */

export type Theme = "system" | "light" | "dark";

const THEME_KEY = "biblecompose.theme";
const ZOOM_KEY = "biblecompose.zoom";

const ZOOM_STEP = 1.1;
export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 2.5;

function stored(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Storage may be unavailable; the preference then lasts the session.
  }
}

function readTheme(): Theme {
  const t = stored(THEME_KEY);
  return t === "light" || t === "dark" ? t : "system";
}

function readZoom(): number {
  const z = Number(stored(ZOOM_KEY));
  return Number.isFinite(z) && z >= ZOOM_MIN && z <= ZOOM_MAX ? z : 1;
}

export const preferences = $state<{ theme: Theme; zoom: number }>({
  theme: readTheme(),
  zoom: readZoom(),
});

/** Put the preferences on the document. Called once, and on every change. */
export function applyPreferences(): void {
  const root = document.documentElement;
  root.dataset.theme = preferences.theme;
  root.style.colorScheme = preferences.theme === "system" ? "light dark" : preferences.theme;
  root.style.zoom = String(preferences.zoom);
}

export function setTheme(theme: Theme): void {
  preferences.theme = theme;
  store(THEME_KEY, theme);
  applyPreferences();
}

export function setZoom(zoom: number): void {
  const clamped = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom));
  // Two decimals, so ten steps in and ten out land back on exactly 1.
  preferences.zoom = Math.round(clamped * 100) / 100;
  store(ZOOM_KEY, String(preferences.zoom));
  applyPreferences();
}

export function zoomIn(): void {
  setZoom(preferences.zoom * ZOOM_STEP);
}

export function zoomOut(): void {
  setZoom(preferences.zoom / ZOOM_STEP);
}

export function resetZoom(): void {
  setZoom(1);
}

/**
 * The shortcuts every browser teaches: Ctrl (or Cmd) with `+`, `-` and `0`,
 * and Ctrl with the mouse wheel. Installed once on the window; returns the
 * function that takes them off again.
 */
export function installViewShortcuts(): () => void {
  const onkeydown = (event: KeyboardEvent) => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
    switch (event.key) {
      case "+":
      case "=":
      case "Add":
        event.preventDefault();
        zoomIn();
        break;
      case "-":
      case "_":
      case "Subtract":
        event.preventDefault();
        zoomOut();
        break;
      case "0":
        event.preventDefault();
        resetZoom();
        break;
      default:
        break;
    }
  };
  const onwheel = (event: WheelEvent) => {
    if (!event.ctrlKey) return;
    event.preventDefault();
    if (event.deltaY < 0) zoomIn();
    else if (event.deltaY > 0) zoomOut();
  };
  window.addEventListener("keydown", onkeydown);
  // Not passive: a wheel the page zooms with must not also scroll it.
  window.addEventListener("wheel", onwheel, { passive: false });
  return () => {
    window.removeEventListener("keydown", onkeydown);
    window.removeEventListener("wheel", onwheel);
  };
}
