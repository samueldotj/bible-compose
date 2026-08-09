<script lang="ts">
  /**
   * GUI-006 and GUI-012: what the build is doing, and the two buttons.
   *
   * Build and Cancel are one control in two states rather than two controls,
   * because only one of them is ever meaningful and a disabled Cancel beside
   * an enabled Build is a thing to read before acting.
   */
  import { session } from "../lib/session.svelte";
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

  {#if session.project?.blocked && !session.built}
    <span class="note">
      {session.errorCount} error{session.errorCount === 1 ? "" : "s"} must be fixed first
    </span>
  {/if}

  {#if session.output}
    <span class="output" title={session.output}>wrote {session.output}</span>
  {:else if session.project}
    <span class="output muted" title={session.project.output}>→ {session.project.output}</span>
  {/if}

  {#if session.backendVersion}
    <span class="backend" title={session.backendVersion}>{session.backendVersion}</span>
  {/if}
</div>

<style>
  .bar {
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
  .note {
    font-size: 0.8rem;
    color: #c0392b;
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
