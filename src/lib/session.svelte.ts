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
  type Recent,
  type BuildEvent,
  type Changes,
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

/**
 * How often the project is compared against what is on disk (FUN-007).
 *
 * Two seconds: fast enough that a publisher who saved in another editor and
 * switched back finds the notice already there, slow enough that statting a
 * project's files is nothing. Nobody is timing this — the notice offers a
 * reload rather than performing one, so it only has to be true by the time it
 * is acted on.
 */
const WATCH_INTERVAL = 2000;

export class Session {
  versions = $state<{ app: string; contract: string; backend: string } | null>(null);
  project = $state<Project | null>(null);
  /** Shown until a project is open, so the panes are never blank. */
  defaults = $state<Defaults | null>(null);
  /** A failure that is not about the project — the shell itself. */
  fault = $state<string | null>(null);
  opening = $state(false);
  /**
   * The folder being opened, while it is being opened.
   *
   * Reading a whole Bible is seconds of parsing, and for those seconds the
   * window has a folder but no project. Held so the wait can name what it is
   * waiting for: "Loading" over a start screen offering the same folder again
   * is the application looking like it did not hear.
   */
  openingWhat = $state<string | null>(null);

  /** The projects this machine has opened, most recent first (PRJ-001). */
  recent = $state<readonly Recent[]>([]);
  /**
   * The folder a new project was just made in.
   *
   * Held so the window can say what to do next. A project that has just been
   * created has a settings file and no Scripture, and the one thing standing
   * between it and a book is copying the USFM in — which nothing else on
   * screen would tell anybody.
   */
  created = $state<string | null>(null);

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
  /**
   * Where the backend's own output is being written (SILE-006).
   *
   * There is no log pane any more — the button that opened it is now the one
   * that opens the folder. The file is still written, in full, and this is how
   * the window can say where: it is on the Open folder button, which is the
   * thing that gets you to it.
   */
  logFile = $state<string | null>(null);

  selectedBook = $state<string | null>(null);
  severity = $state<SeverityFilter>("all");
  bookOnly = $state(false);

  /** Diagnostics from the last edit that was refused, shown against the field. */
  fieldErrors = $state<Record<string, readonly Diagnostic[]>>({});
  /** The same, for style properties, keyed `selector.property`. */
  styleErrors = $state<Record<string, readonly Diagnostic[]>>({});

  /**
   * Whether the problems dialog is open.
   *
   * A dialog rather than a pane under the book list: most of the time there is
   * nothing wrong, and a permanently reserved corner of the left column
   * reading "0" is space taken from the thing being read. When something *is*
   * wrong, a blocked build reports everything at once and that list wants more
   * room than the corner ever had.
   */
  showProblems = $state(false);

  /** Which configuration tab is showing. One of `TABS`. */
  pane = $state("page");
  /** And which section within the Styles tab. One of `STYLE_TABS`. */
  stylePane = $state("typography");
  /** The selector the inspector is showing, and what is filtering the list. */
  inspected = $state<string | null>(null);
  inspectFilter = $state("");

  /** Files changed on disk since the project was read (FUN-007). */
  changes = $state<Changes | null>(null);

  #stop: (() => void) | null = null;
  #watch: ReturnType<typeof setInterval> | null = null;

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

  /** The page as numbers: the project's, or the built-in ones before that. */
  get geometry() {
    return this.project?.geometry ?? this.defaults?.geometry ?? null;
  }

  /**
   * Whether a change can be saved. Without a project there is no file to save
   * it to, so the controls are shown filled in and disabled rather than
   * accepting an edit that would go nowhere.
   */
  get editable(): boolean {
    return this.project !== null;
  }

  /** Errors and warnings, which is what the Problems button counts. */
  get problemCount(): number {
    return this.diagnostics.filter((d) => d.severity !== "info").length;
  }

  get errorCount(): number {
    return this.diagnostics.filter((d) => d.severity === "error").length;
  }

  /** Whether anything on disk differs from what the window is showing. */
  get changedCount(): number {
    const c = this.changes;
    if (!c) return 0;
    return c.modified.length + c.added.length + c.removed.length;
  }

  /** The names, for a one-line notice. */
  get changedNames(): string[] {
    const c = this.changes;
    if (!c) return [];
    return [...c.modified, ...c.added, ...c.removed];
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
    await this.loadRecent();
  }

  /** Never rejects: a start screen with no list is still a start screen. */
  async loadRecent(): Promise<void> {
    try {
      this.recent = await backend().recentProjects();
    } catch {
      this.recent = [];
    }
  }

  async forget(root: string): Promise<void> {
    try {
      this.recent = await backend().forgetProject(root);
    } catch {
      /* The list is a convenience; failing to shorten it is not a fault. */
    }
  }

  /**
   * Start a project and open it.
   *
   * Returns the diagnostics that refused it, or nothing when it was made.
   * Refusals come back rather than landing in the fault banner because they
   * belong beside the field that caused them — a name with a slash in it is
   * something to correct, not a failure of the application.
   */
  async create(parent: string, name: string, language: string): Promise<Diagnostic[]> {
    this.opening = true;
    try {
      this.project = await backend().createProject(parent, name, language);
      this.changes = null;
      this.#startWatching();
      this.#forgetBuild();
      this.selectedBook = null;
      this.created = this.project.root;
      await this.loadRecent();
      return [];
    } catch (e: unknown) {
      return asDiagnostics(e);
    } finally {
      this.opening = false;
    }
  }

  stop(): void {
    this.#stop?.();
    this.#stop = null;
    if (this.#watch !== null) {
      clearInterval(this.#watch);
      this.#watch = null;
    }
  }

  /**
   * Start comparing the project against disk, once there is one.
   *
   * Skipped while a build is running: the build writes into the project's own
   * cache directory, and although those files are not watched, a publisher
   * watching a progress bar does not need a second thing competing for
   * attention.
   */
  #startWatching(): void {
    if (this.#watch !== null) return;
    this.#watch = setInterval(() => {
      if (!this.project || this.building) return;
      void backend()
        .changedFiles()
        .then((changes) => (this.changes = changes))
        // A failed comparison is not worth a fault banner — the next tick
        // tries again, and the window is still showing a correct project.
        .catch(() => {});
    }, WATCH_INTERVAL);
  }

  async choose(): Promise<void> {
    const root = await backend().chooseFolder();
    if (root) await this.open(root);
  }

  async open(root: string): Promise<void> {
    this.opening = true;
    this.openingWhat = root;
    this.fault = null;
    try {
      this.project = await backend().openProject(root);
      this.changes = null;
      this.#startWatching();
      // A new project's diagnostics are its own; the previous build's are not
      // about this folder and would be read as if they were.
      this.#forgetBuild();
      this.selectedBook = this.project.books[0]?.code ?? null;
      // A folder opened deliberately is no longer the one just created, even
      // when it is the same folder: the instruction has been read.
      this.created = null;
      await this.loadRecent();
    } catch (e: unknown) {
      this.fault = String(e);
    } finally {
      this.opening = false;
      this.openingWhat = null;
    }
  }

  /**
   * Put the project down and go back to the start screen.
   *
   * Nothing on disk is touched — this closes a view of a folder that was there
   * before the window opened it. The build log and diagnostics go with it:
   * they are about a publication that is no longer on screen, and leaving them
   * would have the next project inherit the last one's problems.
   */
  async close(): Promise<void> {
    try {
      await backend().closeProject();
    } catch {
      /* The window can let go of a project the shell failed to forget. */
    }
    this.project = null;
    this.changes = null;
    this.created = null;
    this.selectedBook = null;
    this.#forgetBuild();
    await this.loadRecent();
  }

  /**
   * The folder worth showing right now (GUI-009).
   *
   * Where the PDF landed once there is one, and the project itself before
   * that. One button rather than two, because "open the output folder" before
   * anything has been built points at a folder that does not exist yet.
   */
  get folderToOpen(): string | null {
    if (!this.project) return null;
    const pdf = this.output;
    if (pdf) {
      const at = Math.max(pdf.lastIndexOf("/"), pdf.lastIndexOf("\\"));
      if (at > 0) return pdf.slice(0, at);
    }
    return this.project.root;
  }

  async showFolder(): Promise<void> {
    const path = this.folderToOpen;
    if (!path) return;
    try {
      await backend().openFolder(path);
    } catch (e: unknown) {
      this.fault = asDiagnostics(e)
        .map((d) => d.message)
        .join(" ");
    }
  }

  async reopen(): Promise<void> {
    if (this.project) await this.open(this.project.root);
  }

  /**
   * Which books are in the publication, and in what order (BOOK-003, BOOK-004).
   *
   * Both settings written from one place, because the book list asks one
   * question: `books.include` when a tick changes, `books.order` when a row
   * moves. In sequence rather than in parallel — each write reopens the
   * project, and two reopenings racing would leave the window showing
   * whichever finished last.
   *
   * Neither is written when the answer is the default. Every book ticked
   * clears `books.include` rather than listing all sixty-six; the canonical
   * arrangement clears `books.order` rather than writing an explicit copy of
   * it. A settings file should record what a publisher decided.
   */
  async setBooks(order: string[], included: Set<string>): Promise<void> {
    if (!this.project) return;

    const canonical = this.project.canonicalOrder;

    const writes: [string, string | null][] = [
      ["books.order", order.join(",") === canonical.join(",") ? null : order.join(", ")],
      [
        "books.include",
        included.size === this.books.length ? null : order.filter((c) => included.has(c)).join(", "),
      ],
    ];

    for (const [key, value] of writes) {
      const current = this.settings.find((s) => s.key === key);
      if (!current) continue;
      if (value === null) {
        if (current.overridden) await this.resetSetting(key);
      } else if (value !== current.value) {
        await this.setSetting(key, value);
      }
    }
  }

  async setSetting(key: string, value: string): Promise<void> {
    if (!this.project) return;
    try {
      this.project = await backend().setSetting(this.project.root, key, value);
      this.changes = null;
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
      this.changes = null;
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
      this.changes = null;
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
      this.changes = null;
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
