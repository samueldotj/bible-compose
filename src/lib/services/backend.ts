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

/** Mirrors `biblecompose_diagnostics::Diagnostic`. */
export interface Diagnostic {
  readonly code: string;
  readonly severity: Severity;
  readonly message: string;
  readonly location?: { path: string; line?: number; column?: number };
  readonly help?: string;
  readonly detail?: string;
}

export interface BookSummary {
  readonly code: string;
  readonly path: string;
  readonly chapters: number;
}

/**
 * What the shell can do for the interface.
 *
 * An interface rather than a set of functions so a test can supply a fake
 * without a module mock, and so the surface stays small enough to read.
 */
export interface Backend {
  /** Application and typesetting-backend versions, for the about box and the log. */
  versions(): Promise<{ app: string; contract: string; backend: string }>;
  /** Discover and validate a project folder without building it. */
  openProject(root: string): Promise<{
    books: readonly BookSummary[];
    diagnostics: readonly Diagnostic[];
  }>;
}

/** The real one, talking to the Rust side. */
export const tauriBackend: Backend = {
  versions: () => invoke("versions"),
  openProject: (root) => invoke("open_project", { root }),
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
