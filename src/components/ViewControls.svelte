<script lang="ts">
  /**
   * The theme and the zoom, at the end of the tab strip.
   *
   * A dropdown for the theme — system, light, dark — and the zoom as a
   * figure between a minus and a plus, the figure itself putting it back to
   * 100%. The keyboard and the wheel do the same (`lib/preferences`); this
   * is the part a person can see.
   */
  import {
    preferences,
    resetZoom,
    setTheme,
    zoomIn,
    zoomOut,
    type Theme,
  } from "../lib/preferences.svelte";
  import { t } from "../lib/i18n";

  const THEMES: readonly { value: Theme; label: () => string }[] = [
    { value: "system", label: () => t("themeSystem") },
    { value: "light", label: () => t("themeLight") },
    { value: "dark", label: () => t("themeDark") },
  ];
</script>

<div class="view">
  <label class="theme">
    <span class="hidden">{t("theme")}</span>
    <select
      aria-label={t("theme")}
      title={t("theme")}
      value={preferences.theme}
      onchange={(e) => setTheme(e.currentTarget.value as Theme)}
    >
      {#each THEMES as theme (theme.value)}
        <option value={theme.value}>{theme.label()}</option>
      {/each}
    </select>
  </label>
  <span class="zoom" role="group" aria-label={t("zoom")}>
    <button type="button" aria-label={t("zoomOut")} title={t("zoomOut")} onclick={zoomOut}>−</button>
    <button
      type="button"
      class="level"
      aria-label={t("zoomReset")}
      title={t("zoomReset")}
      onclick={resetZoom}>{Math.round(preferences.zoom * 100)}%</button
    >
    <button type="button" aria-label={t("zoomIn")} title={t("zoomIn")} onclick={zoomIn}>+</button>
  </span>
</div>

<style>
  .view {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    align-self: center;
    margin-inline-start: 0.6rem;
  }
  .hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  select {
    padding: 0.2rem 0.3rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    font: inherit;
    font-size: 0.8rem;
  }
  .zoom {
    display: inline-flex;
    align-items: center;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
  }
  .zoom button {
    padding: 0.15rem 0.45rem;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .zoom button:hover {
    background: color-mix(in oklab, currentColor 10%, transparent);
  }
  .level {
    min-inline-size: 3.2rem;
    font-variant-numeric: tabular-nums;
    border-inline: 1px solid color-mix(in oklab, currentColor 25%, transparent) !important;
  }
</style>
