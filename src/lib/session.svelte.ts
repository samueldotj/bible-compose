/**
 * Everything the window knows, in one place.
 *
 * A single store rather than props threaded through five components, because
 * the panes are not independent: clicking a diagnostic selects a book, and the
 * book list filters the diagnostics. Passing that between siblings is how a
 * selection ends up existing twice and disagreeing with itself.
 *
 * No Tauri here either — it goes through the service like everything else.
 */

import {
  asDiagnostics,
  backend,
  type BuildEvent,
  type BuildState,
  type Defaults,
  type Diagnostic,
  type Project,
  type Severity,
} from "./services/backend";

export interface LogLine {
  readonly stream: string;
  readonly text: string;
}

/** What the diagnostics panel is currently showing (DIA-004). */
export type SeverityFilter = Severity | "all";

/**
 * Lines kept in the log.
 *
 * SILE is chatty — a whole Bible produces tens of thousands of lines — and an
 * unbounded array in a reactive list is a window that slows down as the build
 * goes on, which is exactly when it must not. The oldest are dropped; nothing
 * is lost that matters, because the build directory keeps the real log.
 */
const LOG_LIMIT = 5000;

export class Session {
  versions = $state<{ app: string; contract: string; backend: string } | null>(null);
  project = $state<Project | null>(null);
  /** Shown until a project is open, so the panes are never blank. */
  defaults = $state<Defaults | null>(null);
  /** A failure that is not about the project — the shell itself. */
  fault = $state<string | null>(null);
  opening = $state(false);

  buildState = $state<BuildState>("idle");
  building = $state(false);
  /** Whether a build has run in this session, which decides which diagnostics to show. */
  built = $state(false);
  log = $state<LogLine[]>([]);
  buildDiagnostics = $state<Diagnostic[]>([]);
  output = $state<string | null>(null);
  backendVersion = $state<string | null>(null);
  /** Pages set so far, and what the last build of this project needed. */
  pagesDone = $state(0);
  pagesExpected = $state<number | null>(null);
  /** Where the backend's output is being written (SILE-006). */
  logFile = $state<string | null>(null);
  /**
   * Whether the log pane is showing.
   *
   * Hidden by default: it is the backend's own chatter, it is long, and the
   * page counter beside the Build button now answers the question it used to
   * be watched for. GUI-003 still holds — it is one click away and it is a
   * file — but it no longer takes half the window to say "working".
   */
  showLog = $state(false);

  selectedBook = $state<string | null>(null);
  severity = $state<SeverityFilter>("all");
  bookOnly = $state(false);

  /** Diagnostics from the last edit that was refused, shown against the field. */
  fieldErrors = $state<Record<string, readonly Diagnostic[]>>({});
  /** The same, for style properties, keyed `selector.property`. */
  styleErrors = $state<Record<string, readonly Diagnostic[]>>({});

  /** Which configuration tab is showing. One of `TABS`. */
  pane = $state("settings");

  #stop: (() => void) | null = null;

  /**
   * The list the panel shows.
   *
   * Before a build, what opening the project found; after one, what the build
   * reported — which includes everything opening it found, because the build
   * replays them (DIA-002). Concatenating both would show each of them twice.
   */
  get diagnostics(): readonly Diagnostic[] {
    return this.built ? this.buildDiagnostics : (this.project?.diagnostics ?? []);
  }

  get visibleDiagnostics(): readonly Diagnostic[] {
    const book = this.bookOnly ? this.books.find((b) => b.code === this.selectedBook) : undefined;
    return this.diagnostics.filter((d) => {
      if (this.severity !== "all" && d.severity !== this.severity) return false;
      if (book && d.location?.path !== book.path) return false;
      return true;
    });
  }

  get books() {
    return this.project?.books ?? [];
  }

  /**
   * The settings the panes show: the project's when there is one, the
   * built-in ones before that.
   */
  get settings() {
    return this.project?.settings ?? this.defaults?.settings ?? [];
  }

  get styles() {
    return this.project?.styles ?? this.defaults?.styles ?? [];
  }

  /**
   * Whether a change can be saved. Without a project there is no file to save
   * it to, so the controls are shown filled in and disabled rather than
   * accepting an edit that would go nowhere.
   */
  get editable(): boolean {
    return this.project !== null;
  }

  get errorCount(): number {
    return this.diagnostics.filter((d) => d.severity === "error").length;
  }

  get canBuild(): boolean {
    return this.project !== null && !this.building && !this.opening;
  }

  /**
   * Never rejects.
   *
   * It is called from an effect, where a rejection is an unhandled promise and
   * nothing on screen. Both halves can fail independently — a shell with no
   * backend can still subscribe; one with no event channel can still report
   * versions — so each is caught on its own and the window says what is
   * missing instead of coming up blank.
   */
  async start(): Promise<void> {
    try {
      this.versions = await backend().versions();
    } catch (e: unknown) {
      this.fault = String(e);
    }
    try {
      this.defaults = await backend().defaults();
    } catch (e: unknown) {
      this.fault = String(e);
    }
    try {
      this.#stop = await backend().onBuildEvent((event) => this.#receive(event));
    } catch (e: unknown) {
      this.fault = `no build events: ${String(e)}`;
    }
  }

  stop(): void {
    this.#stop?.();
    this.#stop = null;
  }

  async choose(): Promise<void> {
    const root = await backend().chooseFolder();
    if (root) await this.open(root);
  }

  async open(root: string): Promise<void> {
    this.opening = true;
    this.fault = null;
    try {
      this.project = await backend().openProject(root);
      // A new project's diagnostics are its own; the previous build's are not
      // about this folder and would be read as if they were.
      this.#forgetBuild();
      this.selectedBook = this.project.books[0]?.code ?? null;
    } catch (e: unknown) {
      this.fault = String(e);
    } finally {
      this.opening = false;
    }
  }

  async reopen(): Promise<void> {
    if (this.project) await this.open(this.project.root);
  }

  async setSetting(key: string, value: string): Promise<void> {
    if (!this.project) return;
    try {
      this.project = await backend().setSetting(this.project.root, key, value);
      this.#clearFieldError(key);
    } catch (e: unknown) {
      // The file was not changed, so the form keeps showing the old value with
      // the reason the new one was refused beside it.
      this.fieldErrors = { ...this.fieldErrors, [key]: asDiagnostics(e) };
    }
  }

  async resetSetting(key: string): Promise<void> {
    if (!this.project) return;
    try {
      this.project = await backend().resetSetting(this.project.root, key);
      this.#clearFieldError(key);
    } catch (e: unknown) {
      this.fieldErrors = { ...this.fieldErrors, [key]: asDiagnostics(e) };
    }
  }

  async setStyle(selector: string, property: string, value: string): Promise<void> {
    if (!this.project) return;
    const key = `${selector}.${property}`;
    try {
      this.project = await backend().setStyle(this.project.root, selector, property, value);
      this.styleErrors = without(this.styleErrors, key);
    } catch (e: unknown) {
      // Nothing was written, so the row keeps showing the value in force with
      // the reason the new one was refused beside it.
      this.styleErrors = { ...this.styleErrors, [key]: asDiagnostics(e) };
    }
  }

  async resetStyle(selector: string, property: string): Promise<void> {
    if (!this.project) return;
    const key = `${selector}.${property}`;
    try {
      this.project = await backend().resetStyle(this.project.root, selector, property);
      this.styleErrors = without(this.styleErrors, key);
    } catch (e: unknown) {
      this.styleErrors = { ...this.styleErrors, [key]: asDiagnostics(e) };
    }
  }

  async build(): Promise<void> {
    if (!this.project || this.building) return;
    this.#forgetBuild();
    this.building = true;
    this.built = true;
    this.buildState = "loading";
    try {
      await backend().startBuild(this.project.root);
    } catch (e: unknown) {
      this.building = false;
      this.fault = String(e);
    }
  }

  async cancel(): Promise<void> {
    await backend().cancelBuild();
  }

  #receive(event: BuildEvent): void {
    switch (event.kind) {
      case "state":
        this.buildState = event.state;
        break;
      case "diagnostic":
        this.buildDiagnostics = [...this.buildDiagnostics, event.diagnostic];
        break;
      case "log": {
        const next = [...this.log, { stream: event.stream, text: event.text }];
        this.log = next.length > LOG_LIMIT ? next.slice(next.length - LOG_LIMIT) : next;
        break;
      }
      case "backend":
        this.backendVersion = event.version;
        break;
      case "output":
        this.output = event.path;
        break;
      case "logFile":
        this.logFile = event.path;
        break;
      case "pages":
        this.pagesDone = event.done;
        this.pagesExpected = event.expected ?? null;
        break;
      case "finished":
        this.buildState = event.state;
        this.building = false;
        // The build may have written a settings-independent artefact, but it
        // cannot have changed the project — so nothing is reloaded here. A
        // reload would also throw away the diagnostics just collected.
        break;
    }
  }

  /**
   * How far along, as a fraction, or `null` when nothing can honestly say.
   *
   * Capped just short of full while the build is still running: a bar that
   * sits at 100% for the last thirty seconds of a long document is a bar that
   * has lied, and this estimate is the *previous* build's page count, so
   * overshooting it is normal rather than exceptional.
   */
  get progress(): number | null {
    if (!this.pagesExpected || this.pagesDone === 0) return null;
    return Math.min(this.pagesDone / this.pagesExpected, 0.99);
  }

  #forgetBuild(): void {
    this.log = [];
    this.pagesDone = 0;
    this.pagesExpected = null;
    this.logFile = null;
    this.buildDiagnostics = [];
    this.output = null;
    this.buildState = "idle";
    this.built = false;
  }

  #clearFieldError(key: string): void {
    this.fieldErrors = without(this.fieldErrors, key);
  }
}

/** A copy without one key, since assigning a new object is what runes watch. */
function without<T>(record: Record<string, T>, key: string): Record<string, T> {
  if (!(key in record)) return record;
  const next = { ...record };
  delete next[key];
  return next;
}

export const session = new Session();
