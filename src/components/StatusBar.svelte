<script lang="ts">
  /**
   * One line along the bottom of the window.
   *
   * From the left: what the build is doing, and where the PDF goes or went,
   * with the two links that open them. Under the pointer, the line instead
   * says what the control it is on does — a setting, a section, a style
   * property, a template — so the window explains itself without a manual.
   * At the right, the two questions about the next build.
   */
  import Toggle from "./ui/Toggle.svelte";
  import { hover } from "../lib/hover.svelte";
  import { labelFor } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { labelForSelector } from "../lib/styles";
  import { locale, phrases, t } from "../lib/i18n";

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

  const keep = $derived(session.settings.find((s) => s.key === "output.keep_intermediates"));
  const strict = $derived(session.settings.find((s) => s.key === "strict"));
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
      <span class="mono path" title={session.output}>{phrases().wrote(session.output)}</span>
      <button type="button" class="link" data-search-key="action:open-pdf" onclick={() => void session.showPdf()}>
        {t("openPdf")}
      </button>
    {:else if session.project}
      <span class="mono path" title={session.project.output}>→ {session.project.output}</span>
    {/if}
    {#if session.project}
      <button type="button" class="link" data-search-key="action:open-folder" onclick={() => void session.showFolder()}>
        {t("openFolder")}
      </button>
    {/if}
  {/if}

  <span class="spacer"></span>

  {#if keep}
    <label class="option" data-search-key="output.keep_intermediates">
      <Toggle
        checked={keep.value === "true"}
        disabled={!session.editable}
        label={labelFor("output.keep_intermediates")}
        onchange={(on) => void session.setSetting("output.keep_intermediates", on ? "true" : "false")}
      />
      {labelFor("output.keep_intermediates")}
    </label>
  {/if}
  {#if strict}
    <label class="option" data-search-key="strict">
      <Toggle
        checked={strict.value === "true"}
        disabled={!session.editable}
        label={labelFor("strict")}
        onchange={(on) => void session.setSetting("strict", on ? "true" : "false")}
      />
      {labelFor("strict")}
    </label>
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
  .spacer {
    flex: 1;
  }
  .option {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    cursor: default;
  }
</style>
