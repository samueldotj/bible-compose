<script lang="ts">
  /**
   * The centre of the window: the two pages a reader meets together, with a
   * strip of controls over them saying which pages these are and how they
   * are shown.
   *
   * A spread rather than a page, because inner and outer margins, and the
   * left and right running heads, only mean anything across a fold. One
   * page can be asked for, and the page can be fitted to the room or shown
   * at its printed size with room to scroll.
   */
  import ProofPage from "./ProofPage.svelte";
  import Segmented from "./ui/Segmented.svelte";
  import { SAMPLE_BOOK } from "../lib/sample";
  import { mirrored, setMirrored } from "../lib/heads";
  import { session } from "../lib/session.svelte";
  import { goToSetting, goToStyle, ui, type Unit } from "../lib/ui.svelte";
  import { pair } from "../lib/units";
  import { STYLE_GROUPS } from "../lib/styles";
  import { phrases, t } from "../lib/i18n";

  const {
    guides = false,
    slots = false,
    outline = null,
  }: {
    guides?: boolean;
    slots?: boolean;
    outline?: string | null;
  } = $props();

  const g = $derived(session.geometry);
  /** On the Styles section a click on the page chooses a style; elsewhere, a setting. */
  const styling = $derived(session.pane === "styles");

  /** The page's printed height in CSS pixels, for the 100% view. */
  const actualHeight = $derived(g ? (g.pageHeight * 96) / 72 : 960);
  /** Width over height, which decides how tall two pages can be side by side. */
  const ratio = $derived(g ? g.pageWidth / g.pageHeight : 0.7);
  const fitted = $derived(
    ui.spread
      ? `min(100cqb, calc(100cqi / ${2 * ratio} - 2px))`
      : `min(100cqb, calc(100cqi / ${ratio}))`,
  );

  function pickStyle(selector: string): void {
    const group = STYLE_GROUPS.find((x) => x.rows.some((r) => r.selector === selector));
    if (group) goToStyle(selector, group.id);
  }

  const UNITS: readonly { value: Unit; label: string }[] = [
    { value: "in", label: t("unitIn") },
    { value: "mm", label: t("unitMm") },
    { value: "pt", label: t("unitPt") },
  ];
</script>

<div class="canvas">
  <div class="toolbar">
    <span class="which">
      <strong>{SAMPLE_BOOK} 1–2</strong>
      <span class="muted">· {phrases().pages("412", "413")}</span>
    </span>
    {#if guides && g}
      <span class="muted measure">
        {phrases().textBlock(
          pair(g.pageWidth, g.pageHeight, ui.unit),
          pair(g.pageWidth - g.marginInner - g.marginOuter, g.pageHeight - g.marginTop - g.marginBottom, ui.unit),
        )}
      </span>
    {/if}
    {#if outline}
      <span class="muted">{t("showing")} <span class="mono ink">{outline}</span> {t("onThePage")}</span>
    {/if}
    <span class="spacer"></span>
    {#if guides}
      <span data-search-key="action:units">
        <Segmented small options={UNITS} value={ui.unit} label={t("unitIn")} onchange={(u) => (ui.unit = u as Unit)} />
      </span>
    {/if}
    {#if slots}
      <button
        type="button"
        class="btn small"
        data-search-key="action:mirror"
        disabled={!session.editable || mirrored()}
        onclick={() => void setMirrored(true)}
      >
        {t("mirrorButton")}
      </button>
    {/if}
    <span data-search-key="action:spread">
      <Segmented
        small
        options={[
          { value: "spread", label: t("spreadView") },
          { value: "page", label: t("pageView") },
        ]}
        value={ui.spread ? "spread" : "page"}
        onchange={(v) => (ui.spread = v === "spread")}
      />
    </span>
    <span data-search-key="action:fit">
      <Segmented
        small
        options={[
          { value: "fit", label: t("fit") },
          { value: "actual", label: t("actualSize") },
        ]}
        value={ui.fit ? "fit" : "actual"}
        onchange={(v) => (ui.fit = v === "fit")}
      />
    </span>
  </div>

  <div class="room" class:scrolls={!ui.fit}>
    <div class="spread" style:block-size={ui.fit ? fitted : `${actualHeight}px`}>
      {#if ui.spread}
        <ProofPage side="left" {guides} {slots} {outline} onpick={goToSetting} onpickstyle={styling ? pickStyle : undefined} />
      {/if}
      <ProofPage side="right" {guides} {slots} {outline} onpick={goToSetting} onpickstyle={styling ? pickStyle : undefined} />
    </div>
  </div>
</div>

<style>
  .canvas {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-inline-size: 0;
    min-block-size: 0;
  }
  .toolbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    padding: 12px 24px 8px;
    color: var(--mut);
  }
  .which strong {
    color: var(--ink);
    font-weight: 600;
  }
  .ink {
    color: var(--ink);
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .room {
    flex: 1;
    min-block-size: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 24px 20px;
    /* A size container: the spread below is sized to it. */
    container-type: size;
  }
  .room.scrolls {
    display: block;
    overflow: auto;
    overscroll-behavior: contain;
  }
  /* Two pages side by side, as tall as the room allows, or as wide — the
     smaller of the two, so both pages are always whole. Width per page is
     height times the trim's ratio; we bound by height and let each page's
     aspect ratio decide its width, then cap the block size so two of them
     fit across. */
  .spread {
    display: flex;
    gap: 2px;
    box-shadow: var(--shadow-page);
  }
  .room.scrolls .spread {
    inline-size: max-content;
    margin: 0 auto;
  }
</style>
