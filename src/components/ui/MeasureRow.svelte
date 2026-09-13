<script lang="ts">
  /**
   * One page measurement, shown in the unit the person chose.
   *
   * The backend sends the page in points; this shows the number in inches,
   * millimetres or points and writes back in that unit, so what is typed is
   * what the file keeps.
   */
  import { labelFor } from "../../lib/labels";
  import { session } from "../../lib/session.svelte";
  import { ui } from "../../lib/ui.svelte";
  import { fromPoints, withUnit } from "../../lib/units";
  import { t } from "../../lib/i18n";

  const {
    key,
    points,
    label,
    hint,
    idle = false,
    compact = false,
  }: {
    key: string;
    /** The value in force, in points, from the geometry. */
    points: number;
    label?: string;
    hint?: string;
    /** Nothing for it to act on — one column has no gutter. */
    idle?: boolean;
    /** Half a row: label and field only, for a grid of four. */
    compact?: boolean;
  } = $props();

  const setting = $derived(session.settings.find((s) => s.key === key));
  const errors = $derived(session.fieldErrors[key] ?? []);
  const lit = $derived(ui.lit === key);
  const id = $derived(`measure-${key.replace(/\W/g, "-")}`);

  function commit(typed: string): void {
    const value = withUnit(typed, ui.unit);
    if (!setting || value === "" || value === setting.value) return;
    void session.setSetting(key, value);
  }

  const origin = $derived.by(() => {
    if (!setting?.overridden) return null;
    const at = setting.location;
    if (!at) return t("setInProject");
    const file = at.path.split(/[/\\]/).pop() ?? at.path;
    return at.line ? `${file}:${at.line}` : file;
  });
</script>

<div
  class="row"
  class:lit
  class:idle
  class:compact
  role="group"
  data-search-key={key}
  onpointerenter={() => (ui.lit = key)}
  onpointerleave={() => (ui.lit = null)}
  onfocusin={() => (ui.lit = key)}
  onfocusout={() => (ui.lit = null)}
>
  <div class="main">
    <label for={id}>
      <span class="name">{label ?? labelFor(key)}</span>
      {#if hint}<span class="hint">{hint}</span>{/if}
    </label>
    <span class="field">
      <input
        {id}
        class="input"
        class:bad={errors.length > 0}
        type="text"
        inputmode="decimal"
        value={fromPoints(points, ui.unit)}
        disabled={!session.editable || setting === undefined}
        onchange={(e) => commit(e.currentTarget.value)}
      />
      <span class="unit">{ui.unit}</span>
    </span>
  </div>
  {#if setting?.overridden && session.editable}
    <div class="origin">
      <span class="mono">{origin}</span>
      <button type="button" class="link" onclick={() => void session.resetSetting(key)}>{t("reset")}</button>
    </div>
  {/if}
  {#each errors as error (error.code + error.message)}
    <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
  {/each}
</div>

<style>
  .row {
    padding: 9px 10px;
    margin-inline: -10px;
    border-block-end: 1px solid var(--line);
    border-radius: var(--radius);
  }
  .row.compact {
    padding-block: 5px;
    border-block-end: 0;
  }
  .row.lit {
    background: var(--acctint);
  }
  .row.lit .name {
    color: var(--acctext);
    font-weight: 600;
  }
  .row.idle label {
    color: var(--mut);
  }
  .main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }
  .hint {
    font-size: 12px;
    color: var(--mut);
  }
  .field {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  .input {
    inline-size: 84px;
    padding-inline-end: 28px;
    text-align: end;
  }
  .unit {
    position: absolute;
    inset-inline-end: 9px;
    font-size: 11px;
    color: var(--mut);
    pointer-events: none;
  }
  .origin {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-block-start: 5px;
    font-size: 12px;
    color: var(--mut);
  }
  .origin .mono {
    font-size: 11px;
  }
  .error {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--err-ink);
  }
</style>
