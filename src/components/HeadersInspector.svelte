<script lang="ts">
  /**
   * Headers & footers: three slots on each edge of each page — outer,
   * centre, inner — each a template of fields. The same six controls sit on
   * the page itself; these are the same slots in a list, with the one
   * decision the page cannot show: whether the right page follows the left.
   */
  import Inspector from "./ui/Inspector.svelte";
  import SlotControl from "./ui/SlotControl.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import { LINES, POSITIONS, mirrored, setMirrored, slotKey, type Line, type Position } from "../lib/heads";
  import { labelFor, navTitle, tabTitle, TABS } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { locale, phrases, t } from "../lib/i18n";

  const tab = TABS.find((x) => x.id === "headers")!;

  const lineTitle = (side: "left" | "right", line: Line) =>
    side === "left"
      ? line === "header"
        ? t("leftHeader")
        : t("leftFooter")
      : line === "header"
        ? t("rightHeader")
        : t("rightFooter");

  /** Outer first on both sides, which is the inner-most slot's opposite. */
  const ordered = (side: "left" | "right"): readonly Position[] =>
    side === "left" ? POSITIONS : [...POSITIONS].reverse();

  const name = (side: "left" | "right", line: Line, position: Position) =>
    line === "header"
      ? phrases().headerSlot(side === "left" ? t("leftPage") : t("rightPage"), labelFor(slotKey(side, line, position)))
      : phrases().footerSlot(side === "left" ? t("leftPage") : t("rightPage"), labelFor(slotKey(side, line, position)));
</script>

{#snippet slots(side: "left" | "right")}
  {#each LINES as line (line)}
    <div class="section-title">{lineTitle(side, line)}</div>
    {#each ordered(side) as position (position)}
      {@const key = slotKey(side, line, position)}
      <div class="row">
        <span class="label">{labelFor(key)}</span>
        <SlotControl {key} name={name(side, line, position)} />
      </div>
    {/each}
  {/each}
{/snippet}

<Inspector kicker={navTitle(tab.nav)} title={tabTitle(tab)} description={locale().help["tab:headers"]}>
  {@render slots("left")}

  <div class="section-title">{t("rightPageTitle")}</div>
  <div class="row" data-search-key="action:mirror">
    <span class="label">{t("mirrorRow")}</span>
    <Toggle checked={mirrored()} disabled={!session.editable} label={t("mirrorRow")} onchange={(on) => void setMirrored(on)} />
  </div>
  <p class="muted note">{t("mirrorHint")}</p>

  {#if !mirrored()}
    {@render slots("right")}
  {/if}
</Inspector>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 0;
    border-block-end: 1px solid var(--line);
  }
  .row :global(.slot) {
    flex: 1;
    max-inline-size: 62%;
  }
  .label {
    white-space: nowrap;
  }
  .note {
    margin: 0;
    padding: 8px 0;
    font-size: 12px;
  }
</style>
