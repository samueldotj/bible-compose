<script lang="ts">
  /**
   * GUI-003: the backend's own output, as it happens, and copyable.
   *
   * Copyable because SILE-006 keeps every line for exactly one purpose —
   * someone pasting it into a support question. A log you have to retype is a
   * log nobody sends.
   *
   * Follows the tail unless the reader has scrolled up. Scrolling away from
   * the bottom is a deliberate act, and yanking the view back is how a log
   * becomes unreadable during the build it is describing.
   */
  import { session } from "../lib/session.svelte";

  let box = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let copied = $state(false);

  $effect(() => {
    // Touch the length so this re-runs when a line arrives.
    void session.log.length;
    if (follow && box) box.scrollTop = box.scrollHeight;
  });

  function onScroll(): void {
    if (!box) return;
    const distance = box.scrollHeight - box.scrollTop - box.clientHeight;
    follow = distance < 24;
  }

  async function copy(): Promise<void> {
    const text = session.log.map((l) => l.text).join("\n");
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<section class="pane" aria-labelledby="log-heading">
  <header>
    <h2 id="log-heading">Build log <span class="tally">{session.log.length}</span></h2>
    <div class="actions">
      {#if session.logFile}
        <span class="path" title={session.logFile}>{session.logFile}</span>
      {/if}
      {#if !follow}
        <button type="button" onclick={() => ((follow = true), box && (box.scrollTop = box.scrollHeight))}>
          Follow
        </button>
      {/if}
      <button type="button" onclick={() => void copy()} disabled={session.log.length === 0}>
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  </header>

  <div class="box" bind:this={box} onscroll={onScroll} role="log" aria-live="off">
    {#if session.log.length === 0}
      <p class="empty">The backend has not said anything yet.</p>
    {:else}
      {#each session.log as line, i (i)}
        <div class="line" class:stderr={line.stream === "stderr"}>{line.text}</div>
      {/each}
    {/if}
  </div>
</section>

<style>
  header {
    display: flex;
    gap: 1rem;
    align-items: baseline;
    justify-content: space-between;
  }
  .tally {
    font-weight: 400;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
  }
  /* The file is the copy that outlives the window; the pane is the one you can
     watch. Showing the path is what makes the first discoverable. */
  .path {
    font-size: 0.7rem;
    opacity: 0.5;
    direction: rtl;
    text-align: start;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-inline-size: 22rem;
  }
  .actions button {
    padding-block: 0.1rem;
    padding-inline: 0.45rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .actions button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .box {
    margin-block-start: 0.5rem;
    block-size: 100%;
    min-block-size: 8rem;
    overflow: auto;
    padding: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 18%, transparent);
    border-radius: 6px;
    background: color-mix(in oklab, currentColor 4%, transparent);
    font-family: ui-monospace, monospace;
    font-size: 0.78rem;
    line-height: 1.45;
  }
  .line {
    white-space: pre-wrap;
    /* The stream is what distinguishes a line, and colour alone would not —
       stderr is dimmed *and* marked, for the same reason severity is not
       colour-only elsewhere. */
  }
  .line.stderr::before {
    content: "! ";
    opacity: 0.7;
  }
  .line.stderr {
    color: #b8860b;
  }
  .empty {
    margin: 0;
    opacity: 0.6;
    font-family: system-ui, sans-serif;
  }
</style>
