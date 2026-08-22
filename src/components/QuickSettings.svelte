<script lang="ts">
  /**
   * A few settings, inline, where the thing they describe is.
   *
   * Most settings belong in the tabbed form, grouped and legible. A handful do
   * not: what the publication is called and what language it is in are things
   * you set once, when you open the folder, and "keep intermediates" is a
   * question about the build you are *about to run* — it belongs beside the
   * button, not three tabs away from it.
   *
   * The same session calls as the form, so a value edited here is validated,
   * refused and reset exactly as it would be there. What differs is only the
   * shape: one line, no legend, no origin, and a reset that appears only when
   * there is something to reset.
   */
  import { labelFor, PLACEHOLDERS } from "../lib/labels";
  import { LANGUAGES } from "../lib/languages";
  import { session } from "../lib/session.svelte";
  import type { Setting } from "../lib/services/backend";

  const { keys, width = "8rem" }: { keys: readonly string[]; width?: string } = $props();

  const shown = $derived(
    keys.map((k) => session.settings.find((s) => s.key === k)).filter((s) => s !== undefined),
  );

  function commit(setting: Setting, value: string): void {
    if (value === setting.value) return;
    void session.setSetting(setting.key, value);
  }

  /**
   * The languages to offer, with whatever the project already says.
   *
   * A closed list would otherwise be able to change the answer just by being
   * opened: a project set to a tag this list has never heard of — which is an
   * ordinary thing for a Bible to be — would find no option selected, and the
   * first edit to anything else on the row would quietly replace it. So the
   * project's own tag is an option whenever it is not already one.
   */
  function languages(current: string): { tag: string; label: string }[] {
    const listed = LANGUAGES.map((l) => ({ tag: l.tag, label: `${l.name} (${l.tag})` }));
    const tag = current.trim();
    if (tag !== "" && !LANGUAGES.some((l) => l.tag === tag)) {
      listed.unshift({ tag, label: `${tag} — set in this project` });
    }
    return listed;
  }
</script>

<span class="quick">
  {#each shown as setting (setting.key)}
    {@const errors = session.fieldErrors[setting.key] ?? []}
    <span class="field" class:overridden={setting.overridden}>
      {#if setting.kind === "language"}
        <label for={`quick-${setting.key}`}>{labelFor(setting.key)}</label>
        <select
          id={`quick-${setting.key}`}
          value={setting.value}
          disabled={!session.editable}
          onchange={(e) => commit(setting, e.currentTarget.value)}
        >
          {#each languages(setting.value) as l (l.tag)}
            <option value={l.tag}>{l.label}</option>
          {/each}
        </select>
      {:else if setting.kind === "boolean"}
        <label>
          <input
            type="checkbox"
            checked={setting.value === "true"}
            disabled={!session.editable}
            onchange={(e) =>
              void session.setSetting(setting.key, e.currentTarget.checked ? "true" : "false")}
          />
          {labelFor(setting.key)}
        </label>
      {:else}
        <label for={`quick-${setting.key}`}>{labelFor(setting.key)}</label>
        <input
          id={`quick-${setting.key}`}
          type="text"
          style:inline-size={width}
          class:bad={errors.length > 0}
          value={setting.value}
          placeholder={PLACEHOLDERS[setting.key] ?? ""}
          spellcheck="false"
          disabled={!session.editable}
          title={errors.map((e) => e.message).join(" ") || undefined}
          onchange={(e) => commit(setting, e.currentTarget.value)}
        />
      {/if}

      <!-- Only where there is something to undo: a reset beside every field
           would double the width of a strip whose point is being small. -->
      {#if setting.overridden && session.editable}
        <button
          type="button"
          class="reset"
          title="Restore the built-in value"
          aria-label={`Reset ${labelFor(setting.key)}`}
          onclick={() => void session.resetSetting(setting.key)}
        >
          ×
        </button>
      {/if}
    </span>
  {/each}
</span>

<style>
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.9rem;
    align-items: center;
    font-size: 0.82rem;
  }
  .field {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }
  label {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    white-space: nowrap;
    opacity: 0.85;
  }
  input[type="text"] {
    padding-block: 0.15rem;
    padding-inline: 0.3rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.82rem;
  }
  select {
    max-inline-size: 14rem;
    padding-block: 0.15rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.82rem;
  }
  input.bad {
    border-color: #c0392b;
  }
  input:disabled {
    opacity: 0.6;
  }
  .field.overridden label {
    opacity: 1;
  }
  .reset {
    inline-size: 1.05rem;
    block-size: 1.05rem;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: color-mix(in oklab, currentColor 14%, transparent);
    color: inherit;
    font: inherit;
    font-size: 0.75rem;
    line-height: 1;
    cursor: pointer;
  }
</style>
