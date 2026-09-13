<script lang="ts">
  /**
   * GUI-005, DIA-002, DIA-004: everything wrong, as a table, each row naming
   * the section that fixes it.
   *
   * A blocked build lists every blocking issue at once — which is a property
   * of the orchestrator, not of this table, and this table's job is not to
   * hide any of them behind a "first error" summary. The badges over the
   * table are the filter: click one to see only its kind.
   */
  import { session, type SeverityFilter } from "../lib/session.svelte";
  import { TABS, tabTitle } from "../lib/labels";
  import type { Diagnostic } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";

  /** The section that answers a diagnostic, by the stage its code carries. */
  function fixIn(d: Diagnostic): string | null {
    const stage = d.code.split("-")[0];
    switch (stage) {
      case "PRJ":
      case "USFM":
        return "books";
      case "CFG":
        return "contents";
      case "STY":
      case "FONT":
        return "styles";
      case "ASSET":
        return "figures";
      case "OUT":
        return "metadata";
      default:
        return null;
    }
  }
  const sectionName = (id: string) => {
    const tab = TABS.find((x) => x.id === id);
    return tab ? tabTitle(tab) : id;
  };

  function bookFor(d: Diagnostic): string | null {
    const path = d.location?.path;
    if (!path) return null;
    return session.books.find((b) => b.path === path)?.code ?? null;
  }

  function where(d: Diagnostic): string {
    const at = d.location;
    if (!at) return "";
    const book = bookFor(d);
    const file = book ?? (at.path.split(/[/\\]/).pop() ?? at.path);
    if (at.line === undefined) return file;
    return at.column === undefined ? `${file}:${at.line}` : `${file}:${at.line}:${at.column}`;
  }

  function go(d: Diagnostic): void {
    const book = bookFor(d);
    if (book) session.selectedBook = book;
    const tab = fixIn(d);
    if (tab) session.pane = tab;
  }

  const errors = $derived(session.diagnostics.filter((d) => d.severity === "error").length);
  const warnings = $derived(session.diagnostics.filter((d) => d.severity === "warning").length);
  const notes = $derived(session.diagnostics.filter((d) => d.severity === "info").length);

  function filter(kind: SeverityFilter): void {
    session.severity = session.severity === kind ? "all" : kind;
  }
</script>

<section class="build" aria-label={phrases().problems(session.diagnostics.length)}>
  <div class="head">
    <h2>{phrases().problems(session.diagnostics.length)}</h2>
    <span class="badges">
      <button type="button" class="badge error" class:dim={session.severity !== "all" && session.severity !== "error"} onclick={() => filter("error")}>{phrases().errors(errors)}</button>
      <button type="button" class="badge warning" class:dim={session.severity !== "all" && session.severity !== "warning"} onclick={() => filter("warning")}>{phrases().warnings(warnings)}</button>
      <button type="button" class="badge info" class:dim={session.severity !== "all" && session.severity !== "info"} onclick={() => filter("info")}>{phrases().notes(notes)}</button>
    </span>
    <span class="spacer"></span>
    <span class="muted small">{t("buildNote")}</span>
  </div>

  <div class="table">
    <div class="thead">
      <span>{t("severity")}</span><span>{t("whereColumn")}</span><span>{t("message")}</span><span>{t("fixIn")}</span>
    </div>
    <div class="rows">
      {#each session.visibleDiagnostics as d, i (d.code + i)}
        {@const tab = fixIn(d)}
        <button type="button" class="trow" onclick={() => go(d)}>
          <span><span class="badge {d.severity}">{d.severity === "info" ? phrases().notes(1).replace(/^1 /, "") : d.severity}</span></span>
          <span class="mono where">{where(d)}</span>
          <span class="msg" title={d.help ? `${d.message} — ${d.help}` : d.message}>
            {d.message}
            {#if d.help}<span class="muted"> — {d.help}</span>{/if}
          </span>
          <span class="fix">{tab ? `${sectionName(tab)} →` : d.code}</span>
        </button>
      {:else}
        <p class="empty muted">
          {session.diagnostics.length === 0 ? t("nothingToReport") : t("nothingMatchesFilter")}
        </p>
      {/each}
    </div>
  </div>
</section>

<style>
  .build {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 14px;
    min-inline-size: 0;
    min-block-size: 0;
    padding: 22px 28px;
  }
  .head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 14px;
  }
  h2 {
    font: 500 26px/1.1 var(--serif);
    white-space: nowrap;
  }
  .badges {
    display: inline-flex;
    gap: 6px;
  }
  .badges .badge {
    border: 0;
    cursor: pointer;
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
  }
  .badges .badge.dim {
    opacity: 0.4;
  }
  .spacer {
    flex: 1;
  }
  .small {
    font-size: 12px;
  }
  .table {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-block-size: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
    overflow: hidden;
  }
  .thead,
  .trow {
    display: grid;
    grid-template-columns: 96px 130px 1fr 180px;
    gap: 16px;
    align-items: center;
    padding: 0 16px;
  }
  .thead {
    padding-block: 10px;
    border-block-end: 1px solid var(--line2);
    font: italic 12.5px var(--serif);
    color: var(--mut);
  }
  .rows {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .trow {
    inline-size: 100%;
    min-block-size: 44px;
    padding-block: 6px;
    border: 0;
    border-block-end: 1px solid var(--line);
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .trow:hover {
    background: color-mix(in oklab, var(--acctint) 55%, transparent);
  }
  .badge {
    text-transform: capitalize;
  }
  .where {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .msg {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fix {
    color: var(--acc);
    font-weight: 600;
    white-space: nowrap;
  }
  .empty {
    padding: 16px;
    margin: 0;
  }
</style>
