<script lang="ts">
  /**
   * GUI-006 and GUI-012: what the build is doing, and the two buttons.
   *
   * Build and Cancel are one control in two states rather than two controls,
   * because only one of them is ever meaningful and a disabled Cancel beside
   * an enabled Build is a thing to read before acting.
   */
  import QuickSettings from "./QuickSettings.svelte";
  import { session } from "../lib/session.svelte";

  /**
   * What hovering Open folder says.
   *
   * The backend's full output is still written to a file every build
   * (SILE-006); with the log pane gone, this is where the window says so.
   */
  function folderHint(): string {
    const where = session.folderToOpen;
    if (!where) return "No project open";
    return session.logFile ? `${where}
Backend log: ${session.logFile}` : where;
  }
  import type { BuildState } from "../lib/services/backend";

  /** GUI-006's wording, which is also what the CLI prints. */
  const LABELS: Readonly<Record<BuildState, string>> = {
    idle: "idle",
    loading: "loading",
    loaded: "loaded",
    blocked: "blocked",
    validating: "validating",
    emitting: "generating",
    typesetting: "running SILE",
    publishing: "publishing",
    succeeded: "completed",
    failed: "failed",
    cancelled: "canceled",
  };

  const tone = $derived(
    session.buildState === "succeeded"
      ? "good"
      : session.buildState === "failed" ||
          session.buildState === "blocked" ||
          session.buildState === "cancelled"
        ? "bad"
        : session.building
          ? "busy"
          : "idle",
  );
</script>

<div class="bar">
  {#if session.building}
    <button type="button" class="primary" onclick={() => void session.cancel()}>Cancel</button>
  {:else}
    <button
      type="button"
      class="primary"
      disabled={!session.canBuild}
      onclick={() => void session.build()}
    >
      Build
    </button>
  {/if}

  <span class="state {tone}">{LABELS[session.buildState]}</span>

  {#if session.building}
    <!-- A real count, always; a bar with an end only when there is an honest
         one to give. The estimate is the previous build's page count, which is
         the only thing that knows how long this document is — a typesetter
         does not, until it has set it. -->
    <div class="progress" role="status" aria-live="polite">
      <div
        class="track"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={session.pagesExpected ?? undefined}
        aria-valuenow={session.progress === null ? undefined : session.pagesDone}
        aria-label="Typesetting progress"
      >
        {#if session.progress !== null}
          <div class="fill" style:inline-size="{session.progress * 100}%"></div>
        {:else}
          <div class="fill sweeping"></div>
        {/if}
      </div>
      <span class="pages">
        {#if session.pagesDone === 0}
          starting…
        {:else if session.pagesExpected}
          page {session.pagesDone} of about {session.pagesExpected}
        {:else}
          page {session.pagesDone}
        {/if}
      </span>
    </div>
  {/if}

  {#if session.project?.blocked && !session.built}
    <span class="note">
      {session.errorCount} error{session.errorCount === 1 ? "" : "s"} must be fixed first
    </span>
  {/if}

  <!--
    Here rather than under the book list, which is where it was until the books
    became a tab of their own. The count is a standing fact about the project
    and the reason to open the panel at all; behind a tab it would be invisible
    from the four tabs where a publisher is most likely to introduce a problem.
    Beside the build state, because that is what most problems are about.
  -->
  <button
    type="button"
    class="problems"
    class:bad={session.errorCount > 0}
    disabled={!session.project}
    onclick={() => (session.showProblems = true)}
  >
    Problems
    <span class="tally">{session.problemCount}</span>
  </button>


  {#if session.output}
    <span class="output" title={session.output}>wrote {session.output}</span>
  {:else if session.project}
    <span class="output muted" title={session.project.output}>→ {session.project.output}</span>
  {/if}

  <!-- GUI-009. The PDF's folder once there is one, the project's before
       that: opening the output folder before anything has been built points
       at a folder that does not exist. -->
  <button
    type="button"
    class="open-folder"
    disabled={!session.project}
    title={folderHint()}
    onclick={() => void session.showFolder()}
  >
    Open folder
  </button>

  <!-- Questions about the build that is about to run: whether it keeps what
       it wrote on the way, and whether a setting this release does not
       recognise stops it. Beside the button rather than behind a tab, because
       they are decided while looking at this one. -->
  <QuickSettings keys={["output.keep_intermediates", "strict"]} />

  {#if session.backendVersion}
    <span class="backend" title={session.backendVersion}>{session.backendVersion}</span>
  {/if}
</div>

<style>
  .bar {
    /* The bottom edge of the window. It never shrinks — the Build button is
       the control you reach for after changing something, and it should not be
       somewhere you have to go looking.
       
       A tenth of the window exactly, rather than as much as its contents want.
       Everything in here wraps, and a bar that grew to three lines on a narrow
       window would take those lines from the panes above — or, once the column
       adds up to more than the window, put a scrollbar on the whole layout. A
       fixed share makes the arithmetic hold: this tenth, the panes the other
       nine, and nothing left over to scroll. Content past it scrolls in here,
       which is the right trade for a strip of controls against the list of
       books being published. */
    flex: none;
    block-size: 10%;
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.9rem;
    align-items: center;
    padding-block: 0.5rem;
    border-block-start: 1px solid color-mix(in oklab, currentColor 15%, transparent);
  }
  .primary {
    padding-block: 0.3rem;
    padding-inline: 1rem;
    border: 1px solid color-mix(in oklab, currentColor 35%, transparent);
    border-radius: 5px;
    background: color-mix(in oklab, currentColor 10%, transparent);
    color: inherit;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .state {
    font-size: 0.85rem;
    font-variant-caps: all-small-caps;
    letter-spacing: 0.04em;
  }
  .state.good {
    color: #1d7a45;
  }
  .state.bad {
    color: #c0392b;
  }
  .state.busy {
    opacity: 0.85;
  }
  .state.idle {
    opacity: 0.55;
  }
  .progress {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    min-inline-size: 16rem;
  }
  .track {
    position: relative;
    flex: 1;
    block-size: 0.4rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in oklab, currentColor 15%, transparent);
  }
  .fill {
    block-size: 100%;
    border-radius: 999px;
    background: currentColor;
    opacity: 0.55;
    transition: inline-size 200ms linear;
  }
  /* No estimate, so the bar says "working" rather than a fraction it would be
     making up. */
  .fill.sweeping {
    inline-size: 35%;
    animation: sweep 1.4s ease-in-out infinite;
  }
  @keyframes sweep {
    0% {
      margin-inline-start: -35%;
    }
    100% {
      margin-inline-start: 100%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill.sweeping {
      animation: none;
      inline-size: 100%;
      opacity: 0.3;
    }
  }
  .pages {
    font-size: 0.78rem;
    opacity: 0.7;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .note {
    font-size: 0.8rem;
    color: #c0392b;
  }
  .open-folder,
  .problems {
    padding-block: 0.15rem;
    padding-inline: 0.5rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.75rem;
    opacity: 0.7;
    cursor: pointer;
  }
  .problems {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    flex: none;
  }
  .problems:hover:not(:disabled) {
    background: color-mix(in oklab, currentColor 8%, transparent);
    opacity: 1;
  }
  /* An error is the one thing here that stops a build, so it is the one thing
     that stops being quiet. */
  .problems.bad {
    border-color: #c0392b;
    color: #c0392b;
    font-weight: 600;
    opacity: 1;
  }
  .problems .tally {
    padding-inline: 0.35rem;
    border-radius: 999px;
    background: color-mix(in oklab, currentColor 15%, transparent);
    font-variant-numeric: tabular-nums;
  }
  .problems:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .output,
  .backend {
    font-size: 0.78rem;
    opacity: 0.7;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-inline-size: 28rem;
  }
  .output.muted {
    opacity: 0.45;
  }
  .backend {
    margin-inline-start: auto;
    opacity: 0.45;
  }
</style>
