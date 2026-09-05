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

/**
 * Which part of the canon a book belongs to, from the canon table.
 *
 * Sent with each book rather than derived here: sixty-six codes divided into
 * testaments is exactly the sort of list that drifts when it is written twice.
 */
export type Testament = "old" | "new" | "deuterocanon";

export interface BookSummary {
  readonly code: string;
  readonly name: string;
  readonly path: string;
  readonly chapters: number;
  readonly errors: number;
  readonly warnings: number;
  /**
   * Whether the book is in the publication (BOOK-003).
   *
   * A book that is out is still listed — it is on disk and it has a place in
   * the order — but it is never parsed, so it has no chapters and no
   * diagnostics.
   */
  readonly included: boolean;
  readonly testament: Testament;
}

/** Which control a setting needs, decided by the schema and not by the form. */
export type SettingKind =
  | "text"
  /** A font family, which the window offers as a list rather than a spelling. */
  | "font"
  /** A BCP-47 language tag, likewise. */
  | "language"
  /** One of a closed set of spellings, which `Setting.choices` lists. */
  | "choice"
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
  /**
   * For `choice`, every spelling the resolver accepts, in the order to offer
   * them.
   *
   * From the schema rather than written here, so a dropdown cannot offer a
   * value the settings file would reject, nor miss one it would accept.
   */
  readonly choices?: readonly string[];
  /** The project file set it, so it can be reset (CFG-007). */
  readonly overridden: boolean;
  readonly location?: SourceLocation;
}

/** Where a build would find a font, which decides whether it travels with the book. */
export type FontSource = "project" | "backend" | "system";

/** One font the picker offers (GUI-003). */
export interface FontChoice {
  readonly family: string;
  readonly source: FontSource;
  /**
   * How many of the open Scripture's distinct characters it cannot draw.
   * Absent when no project is open to check against — not the same as zero.
   */
  readonly missing?: number;
}

/**
 * The page as numbers, in points, for the diagram beside the page settings.
 *
 * Points and not the written units: `biblecompose-config` already parses
 * `0.55in`, `39.6pt` and `13.97mm` into one number, and a second unit parser
 * here would be a second answer to what a margin is.
 */
export interface Geometry {
  readonly pageWidth: number;
  readonly pageHeight: number;
  readonly marginTop: number;
  readonly marginBottom: number;
  readonly marginInner: number;
  readonly marginOuter: number;
  readonly columnGap: number;
  readonly headerGap: number;
  readonly footerGap: number;
  readonly columns: number;
}

/** One of the editions a project can be started from (P6.2). */
export interface Preset {
  readonly id: string;
  readonly title: string;
  readonly description: string;
}

/** A field a head or foot template can name, with its documentation. */
export interface HeadField {
  /** The canonical spelling, without braces: `FirstChapter`. */
  readonly name: string;
  readonly label: string;
  readonly description: string;
  /** What it might read on a page of 1 John. */
  readonly example: string;
}

/** A project the window has opened before (GUI-001). */
export interface Recent {
  readonly root: string;
  /** The publication's name if its settings give one, else the folder's. */
  readonly name: string;
  /** The folder is gone. The row stays, so it can be forgotten deliberately. */
  readonly missing: boolean;
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

/**
 * What the application does before any project says otherwise.
 *
 * CFG-001 and STY-001 both say there is always an answer, so the window can
 * show one before a folder is open.
 */
export interface Defaults {
  readonly settings: readonly Setting[];
  readonly styles: readonly Style[];
  readonly geometry: Geometry;
}

/** What has changed on disk since the window last read the project. */
export interface Changes {
  readonly modified: readonly string[];
  readonly added: readonly string[];
  readonly removed: readonly string[];
}

export interface Project {
  readonly root: string;
  readonly books: readonly BookSummary[];
  readonly diagnostics: readonly Diagnostic[];
  readonly settings: readonly Setting[];
  readonly styles: readonly Style[];
  /** The same books in canonical order, whatever `books.order` says. */
  readonly canonicalOrder: readonly string[];
  readonly geometry: Geometry;
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
  | { kind: "pages"; done: number; expected?: number }
  | { kind: "logFile"; path: string }
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
  /** The built-in settings and styles, with no project involved. */
  defaults(): Promise<Defaults>;
  /** Discover and validate a project folder without building it. */
  openProject(root: string): Promise<Project>;
  /** Stop watching the open project. Nothing on disk is touched. */
  closeProject(): Promise<void>;
  /** Show a folder in the platform's own file manager (GUI-009). */
  openFolder(path: string): Promise<void>;
  /**
   * Open a finished PDF in the platform's own viewer (GUI-009).
   *
   * There is no preview inside the window and this is deliberate
   * (ADR-003): the viewer a publisher already trusts is the one
   * their printer will use.
   */
  openPdf(path: string): Promise<void>;
  /**
   * Open a web address in the machine's browser.
   *
   * Not by following a link: this is a webview showing the application, so a
   * link that navigated would replace the application with a website. The
   * backend refuses anything that is not `https`.
   */
  openUrl(url: string): Promise<void>;
  /** The editions a project can be started from (P6.2). */
  presets(): Promise<readonly Preset[]>;
  /**
   * The fields a head or foot template can name — from the table the
   * backend checks templates against, so what the window documents is what
   * the file accepts.
   */
  headFields(): Promise<readonly HeadField[]>;
  /**
   * Write one into the project's settings file.
   *
   * A preset is written rather than layered, so what comes back is a project
   * whose settings say what the edition is — editable, and visible in the
   * inspector as having come from the publisher's own file.
   */
  applyPreset(root: string, id: string): Promise<Project>;
  /** The projects this machine has opened, most recent first. */
  recentProjects(): Promise<readonly Recent[]>;
  /** Drop one from that list. The folder is not touched. */
  forgetProject(root: string): Promise<readonly Recent[]>;
  /**
   * Make a folder named after the publication inside `parent`, write its
   * settings, and open it. Rejects with the reason, having created nothing.
   */
  createProject(parent: string, name: string, language: string): Promise<Project>;
  /** What has changed on disk since then (FUN-007). */
  changedFiles(): Promise<Changes>;
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
  /**
   * Every font a build could resolve, each with how much of the open
   * Scripture it cannot draw (GUI-003).
   */
  fonts(root: string | null): Promise<readonly FontChoice[]>;
  /**
   * Returns as soon as the build is handed to a thread (GUI-012).
   *
   * `draft` stamps every page and writes beside the finished PDF rather
   * than over it (P5.4). It is an argument and not a setting because it is
   * what this one run is: a project that remembered it was drafting would
   * eventually ship a stamped book.
   */
  startBuild(root: string, draft: boolean, clean: boolean): Promise<void>;
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
  defaults: () => invoke("defaults"),
  openProject: (root) => invoke("open_project", { root }),
  closeProject: () => invoke("close_project"),
  openFolder: (path) => invoke("open_folder", { path }),
  openPdf: (path) => invoke("open_pdf", { path }),
  openUrl: (url) => invoke("open_url", { url }),
  presets: () => invoke("presets"),
  headFields: () => invoke("head_fields"),
  applyPreset: (root, id) => invoke("apply_preset", { root, id }),
  recentProjects: () => invoke("recent_projects"),
  forgetProject: (root) => invoke("forget_project", { root }),
  createProject: (parent, name, language) =>
    invoke("create_project", { parent, name, language }),
  changedFiles: () => invoke("changed_files"),
  setSetting: (root, key, value) => invoke("set_setting", { root, key, value }),
  resetSetting: (root, key) => invoke("reset_setting", { root, key }),
  setStyle: (root, selector, property, value) =>
    invoke("set_style", { root, selector, property, value }),
  resetStyle: (root, selector, property) =>
    invoke("reset_style", { root, selector, property }),
  fonts: (root) => invoke("fonts", { root }),
  startBuild: (root, draft, clean) => invoke("start_build", { root, draft, clean }),
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
