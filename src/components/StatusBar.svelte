<script lang="ts">
  /**
   * One line above the build bar: what the control under the pointer does.
   *
   * The words come from the help catalogue by the control's search key —
   * a setting, a tab, a section of Styles, a style property — with the
   * template's own description for a template and the style's name for a
   * style row. Nothing under the pointer, and it says how it works.
   */
  import { hover } from "../lib/hover.svelte";
  import { session } from "../lib/session.svelte";
  import { labelFor } from "../lib/labels";
  import { labelForSelector } from "../lib/styles";
  import { locale, phrases, t } from "../lib/i18n";

  function explain(key: string | null): string {
    if (key === null) return t("statusIdle");
    const help = locale().help;
    if (key in help) return help[key]!;
    if (key.startsWith("preset:")) {
      const preset = session.presets?.find((p) => p.id === key.slice("preset:".length));
      return preset ? `${preset.title}: ${preset.description}` : t("statusIdle");
    }
    if (key.startsWith("style:")) {
      const rest = key.slice("style:".length);
      const dot = rest.lastIndexOf(".");
      // `style:heading.s1.font_size` is a property; `style:chapter` a row.
      const property = dot > 0 ? rest.slice(dot + 1) : "";
      const selector = dot > 0 && `property:${property}` in help ? rest.slice(0, dot) : rest;
      const name = labelForSelector(selector) ?? selector;
      if (`property:${property}` in help) return `${name} — ${help[`property:${property}`]}`;
      return phrases().styleRow(name);
    }
    return labelFor(key);
  }

  const text = $derived(explain(hover.key));
</script>

<div class="status" role="status" aria-live="polite">{text}</div>

<style>
  .status {
    flex: none;
    padding: 0.25rem 0.8rem;
    border-block-start: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    background: color-mix(in oklab, CanvasText 4%, Canvas);
    font-size: 0.82rem;
    line-height: 1.3;
    opacity: 0.85;
    min-block-size: 1.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
