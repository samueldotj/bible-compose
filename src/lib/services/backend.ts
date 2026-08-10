/**
 * The only place in the frontend that knows Tauri exists.
 *
 * ADR-003 requires that no component import a Tauri API. Everything
 * privileged — the filesystem, dialogs, process control, the build — arrives
 * through an interface declared here, so a component can be rendered in a test
 * against a plain object and the frontend stays portable if the shell is ever
 * revisited (which ADR-003 now says is possible).
 *
 * `scripts/lint-frontend.mjs` fails the build if anything outside this
 * directory reaches for `@tauri-apps`.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

/** Mirrors `biblecompose_app::BuildState`. */
export type BuildState =
  | "idle"
  | "loading"
  | "loaded"
  | "validating"
  | "blocked"
  | "emitting"
  | "typesetting"
  | "publishing"
  | "succeeded"
  | "failed"
  | "cancelled";

export type Severity = "error" | "warning" | "info";

export interface SourceLocation {
  readonly path: string;
  readonly line?: number;
  readonly column?: number;
}

/** Mirrors `biblecompose_diagnostics::Diagnostic`. */
export interface Diagnostic {
  readonly code: string;
  readonly severity: Severity;
  readonly message: string;
  readonly location?: SourceLocation;
  readonly help?: string;
  readonly detail?: string;
}

export interface BookSummary {
  readonly code: string;
  readonly name: string;
  readonly path: string;
  readonly chapters: number;
  readonly errors: number;
  readonly warnings: number;
}

/** Which control a setting needs, decided by the schema and not by the form. */
export type SettingKind =
  | "text"
  | "length"
  | "page_size"
  | "integer"
  | "boolean"
  | "path"
  | "list";

export interface Setting {
  readonly key: string;
  readonly kind: SettingKind;
  readonly value: string;
  /** The project file set it, so it can be reset (CFG-007). */
  readonly overridden: boolean;
  readonly location?: SourceLocation;
}

export type StyleOrigin = "builtin" | "file" | "inherited";

export interface StyleProperty {
  readonly name: string;
  readonly value: string;
  readonly origin: StyleOrigin;
  /** The selector it was inherited from, when it was. */
  readonly from?: string;
  readonly location?: SourceLocation;
}

export interface Style {
  readonly selector: string;
  readonly properties: readonly StyleProperty[];
}

export interface Project {
  readonly root: string;
  readonly books: readonly BookSummary[];
  readonly diagnostics: readonly Diagnostic[];
  readonly settings: readonly Setting[];
  readonly styles: readonly Style[];
  readonly output: string;
  readonly blocked: boolean;
}

/**
 * One stream rather than one event per kind, so a state change cannot arrive
 * before the diagnostic that explains it.
 */
export type BuildEvent =
  | { kind: "state"; state: BuildState }
  | { kind: "diagnostic"; diagnostic: Diagnostic }
  | { kind: "log"; stream: string; text: string }
  | { kind: "backend"; version: string }
  | { kind: "output"; path: string }
  | { kind: "finished"; state: BuildState };

/** Stops delivering events. */
export type Unsubscribe = () => void;

/**
 * What the shell can do for the interface.
 *
 * An interface rather than a set of functions so a test can supply a fake
 * without a module mock, and so the surface stays small enough to read.
 */
export interface Backend {
  /** Application and typesetting-backend versions, for the about box and the log. */
  versions(): Promise<{ app: string; contract: string; backend: string }>;
  /** Ask the operating system for a folder. `null` if the person cancelled. */
  chooseFolder(): Promise<string | null>;
  /** Discover and validate a project folder without building it. */
  openProject(root: string): Promise<Project>;
  /**
   * Write one setting. Rejects with the diagnostics the new value would cause,
   * having changed nothing.
   */
  setSetting(root: string, key: string, value: string): Promise<Project>;
  /** Remove one setting, so the built-in value applies again (CFG-007). */
  resetSetting(root: string, key: string): Promise<Project>;
  /** Write one style property (STY-005). Rejects with the reason, unchanged. */
  setStyle(root: string, selector: string, property: string, value: string): Promise<Project>;
  /** Remove one, so the cascade decides it again. */
  resetStyle(root: string, selector: string, property: string): Promise<Project>;
  /** Returns as soon as the build is handed to a thread (GUI-012). */
  startBuild(root: string): Promise<void>;
  /** Ask the running build to stop. `false` if there was not one. */
  cancelBuild(): Promise<boolean>;
  /** Everything the build has to say, in order. */
  onBuildEvent(handler: (event: BuildEvent) => void): Promise<Unsubscribe>;
}

/** The real one, talking to the Rust side. */
export const tauriBackend: Backend = {
  versions: () => invoke("versions"),
  chooseFolder: async () => {
    const chosen = await open({ directory: true, multiple: false });
    // The plugin's type allows an array; `multiple: false` means it never is.
    return typeof chosen === "string" ? chosen : null;
  },
  openProject: (root) => invoke("open_project", { root }),
  setSetting: (root, key, value) => invoke("set_setting", { root, key, value }),
  resetSetting: (root, key) => invoke("reset_setting", { root, key }),
  setStyle: (root, selector, property, value) =>
    invoke("set_style", { root, selector, property, value }),
  resetStyle: (root, selector, property) =>
    invoke("reset_style", { root, selector, property }),
  startBuild: (root) => invoke("start_build", { root }),
  cancelBuild: () => invoke("cancel_build"),
  onBuildEvent: async (handler) => {
    const stop = await listen<BuildEvent>("build", (event) => handler(event.payload));
    return stop;
  },
};

/**
 * Swappable so components never reach for the real one directly. A test calls
 * `setBackend` with a stub; nothing else changes.
 */
let current: Backend = tauriBackend;

export function backend(): Backend {
  return current;
}

export function setBackend(next: Backend): void {
  current = next;
}

/**
 * What a rejected command carries.
 *
 * Tauri rejects with whatever the command returned as its error, which for
 * `set_setting` and `reset_setting` is a list of diagnostics. Anything else —
 * a panic, a plugin failure — arrives as a string, and is worth showing rather
 * than swallowing.
 */
export function asDiagnostics(error: unknown): Diagnostic[] {
  if (Array.isArray(error)) return error as Diagnostic[];
  return [
    {
      code: "GUI-000",
      severity: "error",
      message: String(error),
    },
  ];
}
