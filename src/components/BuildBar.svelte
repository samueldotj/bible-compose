<script lang="ts">
  /**
   * GUI-006 and GUI-012: what the build is doing, and the two buttons.
   *
   * Generate PDF and Cancel are one control in two states rather than two
   * controls, because only one of them is ever meaningful and a disabled
   * Cancel beside an enabled Generate PDF is a thing to read before acting.
   *
   * The strip reads left to right as report then action: what the last build
   * did, where it put it, what is wrong with the project — and, at the far
   * end, the two questions about the next build and the button that starts it.
   */
  import QuickSettings from "./QuickSettings.svelte";
  import { session } from "../lib/session.svelte";
  import { locale, t } from "../lib/i18n";

  /**
   * What hovering Open folder says.
   *
   * The backend's full output is still written to a file every build
   * (SILE-006); with the log pane gone, this is where the window says so.
   */
  function folderHint(): string {
    const where = session.folderToOpen;
    if (!where) return t("noProjectOpen");
    return session.logFile ? `${where}
Backend log: ${session.logFile}` : where;
  }


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
  <!--
    The state is the announcement, and the page count is deliberately not.
    A live region that fires once per page would read a thousand times
    through a Bible; the state changes eight times a build and is the thing
    a person waiting for the PDF is waiting to hear. The bar keeps its
    `progressbar` role, so the count is there for anyone who asks for it.
  -->
  <span class="state {tone}" role="status" aria-live="polite">
    {locale().states[session.buildState]}
  </span>

  {#if session.building}
    <!-- A real count, always; a bar with an end only when there is an honest
         one to give. The estimate is the previous build's page count, which is
         the only thing that knows how long this document is — a typesetter
         does not, until it has set it. -->
    <div class="progress" aria-live="off">
      <div
        class="track"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={session.pagesExpected ?? undefined}
        aria-valuenow={session.progress === null ? undefined : session.pagesDone}
        aria-label={t("typesettingProgress")}
      >
        {#if session.progress !== null}
          <div class="fill" style:inline-size="{session.progress * 100}%"></div>
        {:else}
          <div class="fill sweeping"></div>
        {/if}
      </div>
      <span class="pages">
        {#if session.pagesDone === 0}
          {t("starting")}
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
    {t("problems")}
    <span class="tally">{session.problemCount}</span>
  </button>

  {#if session.output}
    <!-- The path is the report; the button beside it is the action. Only once
         there is a file: a viewer opened on nothing is worse than no button. -->
    <button
      type="button"
      class="output-link"
      title={session.output}
      onclick={() => void session.showPdf()}
    >
      wrote {session.output}
    </button>
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
    {t("openFolder")}
  </button>

  {#if session.backendVersion}
    <span class="backend" title={session.backendVersion}>{session.backendVersion}</span>
  {/if}

  <!--
    The right-hand end, and everything that belongs to the act of running a
    build comes with it.

    Questions about the build that is *about to run* — whether it keeps what it
    wrote on the way, and whether a setting this release does not recognise
    stops it — are decided while looking at the button, so they travel with it
    rather than staying where the button used to be. Everything to the left is
    a report on the build that already ran.
  -->
  <div class="go">
    <QuickSettings keys={["output.keep_intermediates", "strict"]} />
    <!--
      A proof rather than the publication. Beside the button and not in the
      settings form, because it describes the run you are about to start and
      not the project — and because the button's own label changes with it,
      which is the clearest possible statement of what pressing it will do.
    -->
    <label class="draft">
      <input type="checkbox" bind:checked={session.draft} disabled={session.building} />
      {t("draft")}
    </label>
    <!--
      A build with nothing to do is skipped. This is how you make it do the work
      anyway, which is the answer when something outside the project changed —
      a system font, artwork on another disk — because the fingerprint is a
      promise about the project's own files and says so (BLD-007).
    -->
    <label class="draft" title={t("cleanHint")}>
      <input type="checkbox" bind:checked={session.clean} disabled={session.building} />
      {t("clean")}
    </label>
    {#if session.building}
      <button type="button" class="primary" onclick={() => void session.cancel()}>{t("cancel")}</button>
    {:else}
      <button
        type="button"
        class="primary"
        disabled={!session.canBuild}
        onclick={() => void session.build()}
      >
        {session.draft ? t("generateDraft") : t("generatePdf")}
      </button>
    {/if}
  </div>
</div>

<style>
  .bar {
    /* The bottom edge of the window. It never shrinks — Generate PDF is the
       control you reach for after changing something, and it should not be
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
  /* The build controls, held at the right-hand end. `auto` rather than
     `justify-content: space-between` on the bar, because the bar wraps: with
     space-between, a bar that fits on one line would also spread the six
     things on its left across the whole width. */
  /* The written path, as a button. Deliberately not styled as one: it is the
     report first and the action second, and a row of buttons all shouting is
     a row nobody reads. */
  .output-link {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    text-align: start;
    cursor: pointer;
    text-decoration: underline;
    text-decoration-style: dotted;
    text-underline-offset: 0.2em;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .draft {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    white-space: nowrap;
  }
  .go {
    display: flex;
    gap: 0.9rem;
    align-items: center;
    flex: none;
    margin-inline-start: auto;
  }
  .primary {
    /* Wide enough for the longer of its two labels, so the strip does not
       shift sideways the moment a build starts. */
    min-inline-size: 8.5rem;
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
