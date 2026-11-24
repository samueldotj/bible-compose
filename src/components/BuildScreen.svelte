<script lang="ts">
  /**
   * GUI-005, DIA-002, DIA-004: everything wrong, as a table, each row naming
   * the section that fixes it.
   *
   * A blocked build lists every blocking issue at once — which is a property
   * of the orchestrator, not of this table, and this table's job is not to
   * hide any of them behind a "first error" summary. The badges over the
   * table are the filter: click one to see only its kind.
   *
   * The text of a row can be selected and copied; only "Fix in" moves to the
   * section. "Copy" puts what the filter shows on the clipboard as one report,
   * each distinct problem once with its count and places, because a project
   * with one systematic fault reports it thousands of times.
   *
   * Once a build has written a PDF the same space can show its pages instead
   * (GUI-008); the switch over the table chooses, and a build that succeeds
   * chooses the pages for you.
   */
  import { session, type SeverityFilter } from "../lib/session.svelte";
  import { TABS, tabTitle } from "../lib/labels";
  import type { Diagnostic } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";
  import { problemReport } from "../lib/problems";
  import PdfPreview from "./PdfPreview.svelte";
  import Segmented from "./ui/Segmented.svelte";

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

  let copied = $state<"idle" | "done" | "failed">("idle");
  let reset: ReturnType<typeof setTimeout> | undefined;

  function fileLabel(path: string): string {
    return session.books.find((b) => b.path === path)?.code ?? (path.split(/[/\\]/).pop() ?? path);
  }

  async function copy(): Promise<void> {
    const text = problemReport(session.visibleDiagnostics, fileLabel, phrases());
    try {
      await navigator.clipboard.writeText(text);
      copied = "done";
    } catch {
      copied = "failed";
    }
    clearTimeout(reset);
    reset = setTimeout(() => (copied = "idle"), 2000);
  }
</script>

<section class="build" aria-label={phrases().problems(session.diagnostics.length)}>
  {#if session.output}
    <div class="views">
      <Segmented
        options={[
          { value: "preview", label: t("previewView") },
          { value: "problems", label: `${t("problemsView")} · ${session.diagnostics.length}` },
        ]}
        value={session.buildView}
        onchange={(v) => (session.buildView = v === "preview" ? "preview" : "problems")}
      />
    </div>
  {/if}

  {#if session.output && session.buildView === "preview"}
    <PdfPreview path={session.output} />
  {:else}
  <div class="head">
    <h2>{phrases().problems(session.diagnostics.length)}</h2>
    <span class="badges">
      <button type="button" class="badge error" class:dim={session.severity !== "all" && session.severity !== "error"} onclick={() => filter("error")}>{phrases().errors(errors)}</button>
      <button type="button" class="badge warning" class:dim={session.severity !== "all" && session.severity !== "warning"} onclick={() => filter("warning")}>{phrases().warnings(warnings)}</button>
      <button type="button" class="badge info" class:dim={session.severity !== "all" && session.severity !== "info"} onclick={() => filter("info")}>{phrases().notes(notes)}</button>
    </span>
    <span class="spacer"></span>
    <span class="muted small">{t("buildNote")}</span>
    <button
      type="button"
      class="copy"
      title={t("copyProblemsHint")}
      disabled={session.visibleDiagnostics.length === 0}
      onclick={copy}
    >{copied === "done" ? t("copiedProblems") : copied === "failed" ? t("copyFailed") : t("copyProblems")}</button>
  </div>

  {#if (session.project?.invalidKeys.length ?? 0) > 0}
    {@const invalid = session.project?.invalidKeys ?? []}
    <div class="invalid" role="status">
      <span>
        {phrases().invalidKeys(invalid.length)}
        {#each invalid as k, i (k.file + k.key)}
          <code class="mono" title={k.file}>{k.key}</code>{i < invalid.length - 1 ? ", " : ""}
        {/each}
      </span>
      <button
        type="button"
        class="remove"
        title={t("removeInvalidHint")}
        onclick={() => session.removeInvalidSettings()}
      >{t("removeInvalid")}</button>
    </div>
  {/if}

  <div class="table">
    <div class="thead">
      <span>{t("severity")}</span><span>{t("whereColumn")}</span><span>{t("message")}</span><span>{t("fixIn")}</span>
    </div>
    <div class="rows">
      {#each session.visibleDiagnostics as d, i (d.code + i)}
        {@const tab = fixIn(d)}
        <div class="trow">
          <span><span class="badge {d.severity}">{d.severity === "info" ? phrases().notes(1).replace(/^1 /, "") : d.severity}</span></span>
          <span class="mono where">{where(d)}</span>
          <span class="msg" title={d.help ? `${d.message} — ${d.help}` : d.message}>
            {d.message}
            {#if d.help}<span class="muted"> — {d.help}</span>{/if}
          </span>
          {#if tab}
            <button type="button" class="fix" title={d.code} onclick={() => go(d)}>{`${sectionName(tab)} →`}</button>
          {:else}
            <span class="code mono">{d.code}</span>
          {/if}
        </div>
      {:else}
        <p class="empty muted">
          {session.diagnostics.length === 0 ? t("nothingToReport") : t("nothingMatchesFilter")}
        </p>
      {/each}
    </div>
  </div>
  {/if}
</section>

<style>
  .invalid {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--warn);
    background: var(--warn-bg);
    border-radius: 6px;
    font-size: 13px;
  }
  .invalid span {
    flex: 1;
    min-inline-size: 0;
  }
  .invalid code {
    font-size: 12px;
  }
  .invalid .remove {
    flex: none;
  }
  .build {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 14px;
    min-inline-size: 0;
    min-block-size: 0;
    padding: 22px 28px;
  }
  .views {
    display: flex;
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
    user-select: text;
  }
  .trow:hover {
    background: color-mix(in oklab, var(--acctint) 55%, transparent);
  }
  .copy {
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: inherit;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    padding: 4px 12px;
    cursor: pointer;
    min-inline-size: 7em;
  }
  .copy:disabled {
    opacity: 0.4;
    cursor: default;
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
    justify-self: start;
    border: 0;
    padding: 0;
    background: none;
    color: var(--acc);
    font: inherit;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  .fix:hover {
    text-decoration: underline;
  }
  .code {
    color: var(--mut);
  }
  .empty {
    padding: 16px;
    margin: 0;
  }
</style>
