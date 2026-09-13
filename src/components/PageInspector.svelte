<script lang="ts">
  /**
   * Trim & margins: the sheet, the text block and the space around it.
   *
   * Nine numbers in a column do not say which margin is against the spine,
   * and that is the one thing about them a publisher has to get right — so
   * the spread beside this draws every one of them as a guide, and pointing
   * at a row lights its guide. The trim keeps a free text field under its
   * list of usual sizes, because a publisher entering what their press
   * quoted them is the ordinary case.
   */
  import Inspector from "./ui/Inspector.svelte";
  import MeasureRow from "./ui/MeasureRow.svelte";
  import Segmented from "./ui/Segmented.svelte";
  import { navTitle, tabTitle, TABS } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { TRIMS } from "../lib/trims";
  import { ui } from "../lib/ui.svelte";
  import { locale, t } from "../lib/i18n";

  const tab = TABS.find((x) => x.id === "page")!;
  const g = $derived(session.geometry);
  const size = $derived(session.settings.find((s) => s.key === "page.size"));
  const columnsSetting = $derived(session.settings.find((s) => s.key === "page.columns"));

  /** The trim list's entry for the value in force, or `custom`. */
  const trimEntry = $derived(TRIMS.some((x) => x.value === size?.value) ? (size?.value ?? "") : "custom");
  let customising = $state(false);

  function pickTrim(entry: string): void {
    if (entry === "custom") {
      customising = true;
      return;
    }
    customising = false;
    if (size && entry !== size.value) void session.setSetting("page.size", entry);
  }

  const columns = $derived(String(Math.max(1, Math.min(3, g?.columns ?? 2))));

  function light(key: string, on: boolean): void {
    ui.lit = on ? key : null;
  }
</script>

<Inspector kicker={navTitle(tab.nav)} title={tabTitle(tab)} description={locale().help["tab:page"]}>
  {#if g}
    <div class="section-title">{t("trimTitle")}</div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="row"
      class:lit={ui.lit === "page.size"}
      data-search-key="page.size"
      onpointerenter={() => light("page.size", true)}
      onpointerleave={() => light("page.size", false)}
    >
      <div class="main">
        <span>{t("sizeRow")}</span>
        <span class="select-wrap wide">
          <select
            class="input"
            aria-label={t("sizeRow")}
            value={customising ? "custom" : trimEntry}
            disabled={!session.editable}
            onchange={(e) => pickTrim(e.currentTarget.value)}
          >
            {#each TRIMS as trim (trim.value)}
              <option value={trim.value}>{trim.value} — {trim.name}</option>
            {/each}
            <option value="custom">{t("customSize")}</option>
          </select>
        </span>
      </div>
      {#if customising || trimEntry === "custom"}
        <input
          type="text"
          class="input custom"
          aria-label={t("sizeRow")}
          value={size?.value ?? ""}
          spellcheck="false"
          disabled={!session.editable}
          onchange={(e) => void session.setSetting("page.size", e.currentTarget.value)}
        />
      {/if}
      {#if size?.overridden && session.editable}
        <div class="origin">
          <span class="mono">{size.location?.line ? `${size.location.path.split(/[/\\]/).pop()}:${size.location.line}` : t("setInProject")}</span>
          <button type="button" class="link" onclick={() => void session.resetSetting("page.size")}>{t("reset")}</button>
        </div>
      {/if}
      {#each session.fieldErrors["page.size"] ?? [] as error (error.message)}
        <p class="error">{error.message}</p>
      {/each}
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="row"
      class:lit={ui.lit === "page.columns"}
      data-search-key="page.columns"
      onpointerenter={() => light("page.columns", true)}
      onpointerleave={() => light("page.columns", false)}
    >
      <div class="main">
        <span>{t("columnsRow")}</span>
        <Segmented
          options={[
            { value: "1", label: "1" },
            { value: "2", label: "2" },
            { value: "3", label: "3" },
          ]}
          value={columns}
          label={t("columnsRow")}
          disabled={!session.editable}
          onchange={(n) => void session.setSetting("page.columns", n)}
        />
      </div>
      {#if columnsSetting?.overridden && session.editable}
        <div class="origin">
          <span class="mono">{t("setInProject")}</span>
          <button type="button" class="link" onclick={() => void session.resetSetting("page.columns")}>{t("reset")}</button>
        </div>
      {/if}
    </div>

    <MeasureRow key="page.column_gap" points={g.columnGap} label={t("gutterRow")} hint={t("gutterHint")} idle={g.columns < 2} />

    <div class="section-title">{t("marginsTitle")}</div>
    <div class="grid">
      <MeasureRow key="page.margin_top" points={g.marginTop} label={t("top")} compact />
      <MeasureRow key="page.margin_bottom" points={g.marginBottom} label={t("bottom")} compact />
      <MeasureRow key="page.margin_inner" points={g.marginInner} label={t("inner")} compact />
      <MeasureRow key="page.margin_outer" points={g.marginOuter} label={t("outer")} compact />
    </div>
    <p class="muted note">{t("marginsHint")}</p>

    <div class="section-title">{t("furnitureTitle")}</div>
    <MeasureRow key="page.header_gap" points={g.headerGap} label={t("headRow")} hint={t("headHint")} />
    <MeasureRow key="page.footer_gap" points={g.footerGap} label={t("footRow")} hint={t("footHint")} />
  {:else}
    <p class="muted">{t("loading")}</p>
  {/if}
</Inspector>

<style>
  .row {
    padding: 9px 10px;
    margin-inline: -10px;
    border-block-end: 1px solid var(--line);
    border-radius: var(--radius);
  }
  .row.lit {
    background: var(--acctint);
  }
  .main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .wide {
    max-inline-size: 65%;
  }
  .custom {
    inline-size: 100%;
    margin-block-start: 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 16px;
    padding: 5px 0;
    border-block-end: 1px solid var(--line);
  }
  .note {
    margin: 0;
    padding: 6px 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .origin {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-block-start: 5px;
    font-size: 12px;
    color: var(--mut);
  }
  .error {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--err-ink);
  }
</style>
