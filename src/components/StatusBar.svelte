<script lang="ts">
  /**
   * One line along the bottom of the window.
   *
   * From the left: what the build is doing, and where the PDF goes or went —
   * the path itself opens it. Under the pointer, the line instead
   * says what the control it is on does — a setting, a section, a style
   * property, a template — so the window explains itself without a manual.
   *
   * **And nothing else.** Keep intermediates and Strict settings used to sit
   * at the right of this line as well as on the Build section, which put two
   * switches that change what a build *does* in the one place a person reads
   * rather than operates — and put them a few pixels from the path, where the
   * help text swapping in and out moves everything around them. They live on
   * Build now, where the rest of the build's settings are.
   */
  import { hover } from "../lib/hover.svelte";
  import { labelFor } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { labelForSelector } from "../lib/styles";
  import { locale, phrases } from "../lib/i18n";

  function explain(key: string | null): string | null {
    if (key === null) return null;
    const help = locale().help;
    if (key in help) return help[key]!;
    if (key.startsWith("preset:")) {
      const preset = session.presets?.find((p) => p.id === key.slice("preset:".length));
      return preset ? `${preset.title}: ${preset.description}` : null;
    }
    if (key.startsWith("style:")) {
      const rest = key.slice("style:".length);
      const dot = rest.lastIndexOf(".");
      const property = dot > 0 ? rest.slice(dot + 1) : "";
      const selector = dot > 0 && `property:${property}` in help ? rest.slice(0, dot) : rest;
      const name = labelForSelector(selector) ?? selector;
      if (`property:${property}` in help) return `${name} — ${help[`property:${property}`]}`;
      return phrases().styleRow(name);
    }
    return labelFor(key);
  }

  const help = $derived(explain(hover.key));

  const tone = $derived(
    session.buildState === "succeeded"
      ? "good"
      : session.buildState === "failed" || session.buildState === "blocked" || session.buildState === "cancelled"
        ? "bad"
        : session.building
          ? "busy"
          : "idle",
  );

  const state = $derived.by(() => {
    const word = locale().states[session.buildState];
    if (session.building && session.progress !== null) return `${word} · ${Math.round(session.progress * 100)}%`;
    return word;
  });

</script>

<footer class="status">
  <span class="state {tone}" role="status" aria-live="polite">
    <span class="dot" aria-hidden="true"></span>
    {state}
  </span>

  {#if help}
    <span class="help">{help}</span>
  {:else}
    {#if session.output}
      <!--
        **The path is the control, and it carries no `data-search-key` on
        purpose.** The help above replaces this whole branch, so anything
        tagged here unmounts the moment the pointer arrives — which is what
        made the two buttons that used to sit along this line impossible to
        click, rather than merely hard to. The actions keep their keys on the
        Build inspector's copies, which is where the search looks for them.
      -->
      <button type="button" class="link mono path" title={session.output} onclick={() => void session.showPdf()}>
        {phrases().wrote(session.output)}
      </button>
    {:else if session.project}
      <span class="mono path" title={session.project.output}>→ {session.project.output}</span>
    {/if}
  {/if}

</footer>

<style>
  .status {
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    block-size: var(--status-h);
    padding: 0 20px;
    border-block-start: 1px solid var(--line);
    background: var(--panel);
    font-size: 12px;
    color: var(--mut);
    white-space: nowrap;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .dot {
    inline-size: 7px;
    block-size: 7px;
    border-radius: 4px;
    background: var(--line2);
  }
  .state.idle .dot {
    background: var(--ok);
  }
  .state.good .dot {
    background: var(--ok);
  }
  .state.bad .dot {
    background: var(--err);
  }
  .state.busy .dot {
    background: var(--acc);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .state.busy .dot {
      animation: none;
    }
  }
  .path,
  .help {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .help {
    color: var(--ink);
  }
</style>
