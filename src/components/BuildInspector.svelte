<script lang="ts">
  /**
   * GUI-006 and GUI-012: what the build is doing, stage by stage, and the
   * two questions about the next one.
   *
   * The stages are the states the orchestrator reports (GUI-006), each with
   * a tick once passed and a spinner while running. The bar is honest: it
   * has an end only when the previous build's page count can give it one,
   * and it stops short of full while the build is still running.
   */
  import Inspector from "./ui/Inspector.svelte";
  import SettingRow from "./ui/SettingRow.svelte";
  import { navTitle, TABS, tabTitle } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import type { BuildState } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";

  const tab = TABS.find((x) => x.id === "build")!;

  /** The stages in order, and the states that mean each is under way. */
  const STAGES: readonly { label: () => string; states: readonly BuildState[] }[] = [
    { label: () => t("stageRead"), states: ["loading", "loaded"] },
    { label: () => t("stageCheck"), states: ["validating", "blocked"] },
    { label: () => t("stageEmit"), states: ["emitting"] },
    { label: () => t("stageLayout"), states: ["typesetting"] },
    { label: () => t("stageWrite"), states: ["publishing"] },
  ];
  const ORDER: readonly BuildState[] = [
    "idle",
    "loading",
    "loaded",
    "validating",
    "blocked",
    "emitting",
    "typesetting",
    "publishing",
    "succeeded",
    "failed",
    "cancelled",
  ];
  const rank = (s: BuildState) => ORDER.indexOf(s);

  function stageState(stage: (typeof STAGES)[number]): "done" | "running" | "pending" | "stopped" {
    const now = session.buildState;
    if (now === "idle") return "pending";
    if (stage.states.includes(now)) return session.building ? "running" : "stopped";
    const last = stage.states[stage.states.length - 1]!;
    if (now === "succeeded") return "done";
    if (now === "failed" || now === "cancelled" || now === "blocked") {
      // Everything before the point it stopped was passed.
      return rank(last) < rank(now) && now !== "blocked" ? "done" : rank(last) < rank("validating") ? "done" : "pending";
    }
    return rank(last) < rank(now) ? "done" : "pending";
  }

  const title = $derived.by(() => {
    switch (session.buildState) {
      case "idle":
        return t("buildIdleTitle");
      case "succeeded":
        return t("completed");
      case "failed":
        return t("failed");
      case "blocked":
        return t("blocked");
      case "cancelled":
        return t("cancelled");
      default:
        return t("building");
    }
  });

  const description = $derived.by(() => {
    if (session.building) {
      if (session.pagesDone === 0) return t("starting");
      return phrases().pagesSoFar(session.pagesDone, session.pagesExpected);
    }
    if (session.project?.blocked && !session.built) return phrases().errorsMustBeFixed(session.errorCount);
    if (session.output) return phrases().wrote(session.output);
    return t("buildIdleDesc");
  });
</script>

<Inspector kicker={`${navTitle(tab.nav)} › ${tabTitle(tab)}`} {title} {description}>
  {#if session.building}
    <div class="track" role="progressbar" aria-valuemin={0} aria-valuemax={session.pagesExpected ?? undefined} aria-valuenow={session.progress === null ? undefined : session.pagesDone} aria-label={t("typesettingProgress")}>
      {#if session.progress !== null}
        <div class="fill" style:inline-size="{session.progress * 100}%"></div>
      {:else}
        <div class="fill sweeping"></div>
      {/if}
    </div>
  {/if}

  <ol class="stages">
    {#each STAGES as stage (stage.label())}
      {@const s = stageState(stage)}
      <li class={s}>
        <span class="mark" aria-hidden="true">
          {#if s === "done"}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"></path></svg>
          {:else if s === "running"}
            <span class="spin"></span>
          {:else if s === "stopped"}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M18 6 6 18"></path><path d="m6 6 12 12"></path></svg>
          {:else}
            <span class="ring"></span>
          {/if}
        </span>
        <span class="name">{stage.label()}</span>
        {#if s === "running" && stage.states.includes("typesetting") && session.progress !== null}
          <span class="mono muted">{Math.round(session.progress * 100)}%</span>
        {/if}
      </li>
    {/each}
  </ol>

  <div class="go">
    {#if session.building}
      <button type="button" class="btn" data-search-key="action:cancel" onclick={() => void session.cancel()}>{t("cancelBuild")}</button>
    {:else}
      <button type="button" class="btn primary" data-search-key="action:generate" disabled={!session.canBuild} onclick={() => void session.build()}>
        {t("generatePdf")}
      </button>
    {/if}
  </div>

  <div class="section-title">{t("options")}</div>
  <SettingRow key="output.keep_intermediates" hint={t("keepHint")} />
  <SettingRow key="strict" hint={t("strictHint")} />

  <div class="section-title">{t("outputFolder")}</div>
  <div class="row">
    <span class="mono muted path" title={session.output ?? session.project?.output}>{session.output ?? session.project?.output}</span>
  </div>
  <div class="links">
    <button type="button" class="link" data-search-key="action:open-folder" onclick={() => void session.showFolder()}>{t("openFolder")}</button>
    {#if session.output}
      <button type="button" class="link" data-search-key="action:open-pdf" onclick={() => void session.showPdf()}>{t("openPdf")}</button>
    {/if}
  </div>
  {#if session.logFile}
    <p class="muted small">{t("backendLog")}: <span class="mono">{session.logFile}</span></p>
  {/if}
</Inspector>

<style>
  .track {
    block-size: 6px;
    margin: 18px 0 6px;
    border-radius: 3px;
    background: var(--line);
    overflow: hidden;
  }
  .fill {
    block-size: 100%;
    background: var(--acc);
    transition: inline-size 200ms linear;
  }
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
      opacity: 0.4;
    }
  }
  .stages {
    margin: 14px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
    line-height: 2;
    color: var(--mut);
  }
  .stages li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stages li.done .name,
  .stages li.running .name {
    color: var(--ink);
  }
  .stages li.running .name {
    font-weight: 600;
  }
  .stages li.pending {
    opacity: 0.6;
  }
  .mark {
    display: inline-flex;
    inline-size: 13px;
    block-size: 13px;
    color: var(--acc);
  }
  .stages li.stopped .mark {
    color: var(--err);
  }
  .ring {
    inline-size: 13px;
    block-size: 13px;
    border: 1.5px solid var(--line2);
    border-radius: 50%;
  }
  .spin {
    inline-size: 13px;
    block-size: 13px;
    border: 2px solid var(--acc);
    border-inline-end-color: transparent;
    border-radius: 50%;
    animation: turn 0.9s linear infinite;
  }
  @keyframes turn {
    to {
      rotate: 360deg;
    }
  }
  .stages .mono {
    margin-inline-start: auto;
  }
  .go {
    padding: 16px 0 6px;
  }
  .row {
    padding: 9px 0 4px;
  }
  .path {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .links {
    display: flex;
    gap: 14px;
    padding-block-end: 9px;
    border-block-end: 1px solid var(--line);
  }
  .small {
    font-size: 12px;
    overflow-wrap: anywhere;
  }
</style>
