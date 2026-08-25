<script lang="ts">
  /**
   * GUI-002: every supported setting, editable without touching TOML.
   *
   * The rows come from the schema (`Settings::fields`), so this file decides
   * what to *call* a setting and never what settings exist. A key added to the
   * schema appears here on the next build, in the "Other" group if nobody has
   * given it a home yet — which is a visible prompt rather than a silent
   * omission.
   *
   * Edits commit on change rather than on a Save button. There is no unsaved
   * state to lose that way, and CFG-006's write-back is cheap enough that
   * batching buys nothing.
   */
  import FontPicker from "./FontPicker.svelte";
  import { EDITED_ELSEWHERE, GROUPS, labelFor, placeholderFor, wordsFor } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import type { Setting } from "../lib/services/backend";
  import { t } from "../lib/i18n";

  /**
   * Which groups to show, and whether to sweep up the settings that belong to
   * none. The tab decides — this renders whatever it is handed, so adding a
   * tab is a change to one list rather than to a component.
   */
  const { groups: show, orphans = false }: { groups: readonly string[]; orphans?: boolean } =
    $props();

  const grouped = $derived.by(() => {
    const settings = session.settings;
    const byKey = new Map(settings.map((s) => [s.key, s]));
    const placed = new Set<string>();

    const groups = GROUPS.filter((g) => show.includes(g.id)).map((g) => {
      const rows: Setting[] = [];
      for (const key of g.keys) {
        const row = byKey.get(key);
        if (row) {
          rows.push(row);
          placed.add(key);
        }
      }
      return { id: g.id, title: g.title, rows };
    }).filter((g) => g.rows.length > 0);

    if (orphans) {
      // Against every group, not only the ones on this tab: a key that has a
      // home elsewhere is not an orphan.
      const homed = new Set(GROUPS.flatMap((g) => g.keys));
      const stray = settings.filter((s) => !homed.has(s.key) && !EDITED_ELSEWHERE.has(s.key));
      if (stray.length > 0) {
        groups.push({ id: "other", title: "Other", rows: stray });
      }
    }
    return groups;
  });

  /** Which font setting has the picker open, if any. */
  let picking = $state<string | null>(null);
  function commit(setting: Setting, value: string): void {
    if (value === setting.value) return;
    void session.setSetting(setting.key, value);
  }

  function toggle(setting: Setting, on: boolean): void {
    void session.setSetting(setting.key, on ? "true" : "false");
  }

  function origin(setting: Setting): string {
    if (!setting.overridden) return "built-in default";
    const at = setting.location;
    if (!at) return "set in this project";
    return at.line ? `${at.path}:${at.line}` : at.path;
  }
</script>

<section class="pane" aria-label={t("settingsRegion")}>
  {#if session.settings.length === 0}
    <p class="empty">{t("loading")}</p>
  {:else}
    {#each grouped as group (group.id)}
      <fieldset>
        <legend>{group.title}</legend>

        {#each group.rows as setting (setting.key)}
          {@const errors = session.fieldErrors[setting.key] ?? []}
          <div class="row" class:overridden={setting.overridden}>
            <label for={`set-${setting.key}`}>{labelFor(setting.key)}</label>

            {#if setting.kind === "boolean"}
              <input
                id={`set-${setting.key}`}
                type="checkbox"
                checked={setting.value === "true"}
                disabled={!session.editable}
                onchange={(e) => toggle(setting, e.currentTarget.checked)}
              />
            {:else if setting.kind === "integer"}
              <input
                id={`set-${setting.key}`}
                type="number"
                min="1"
                value={setting.value}
                disabled={!session.editable}
                onchange={(e) => commit(setting, e.currentTarget.value)}
              />
            {:else if setting.kind === "choice"}
              <!--
                The options come with the setting, from the same table the
                resolver parses with — so this control cannot offer a value
                the file would then reject.
              -->
              <select
                id={`set-${setting.key}`}
                value={setting.value}
                disabled={!session.editable}
                onchange={(e) => commit(setting, e.currentTarget.value)}
              >
                {#each setting.choices ?? [] as choice (choice)}
                  <option value={choice}>{wordsFor(choice)}</option>
                {/each}
              </select>
            {:else if setting.kind === "font"}
              <!--
                Typed as well as picked. The field stays editable because a
                publisher may know the exact name of a font they are about to
                install, and refusing to accept it until then would be the
                dialog getting in the way rather than helping.
              -->
              <span class="font-field">
                <input
                  id={`set-${setting.key}`}
                  type="text"
                  dir="auto"
                  value={setting.value}
                  spellcheck="false"
                  disabled={!session.editable}
                  onchange={(e) => commit(setting, e.currentTarget.value)}
                />
                <button type="button" onclick={() => (picking = setting.key)}>{t("choose")}</button>
              </span>
              {#if picking === setting.key}
                <FontPicker
                  current={setting.value}
                  onchoose={(family) => commit(setting, family)}
                  onclose={() => (picking = null)}
                />
              {/if}
            {:else}
              <input
                id={`set-${setting.key}`}
                type="text"
                dir="auto"
                value={setting.value}
                placeholder={placeholderFor(setting.key) ?? ""}
                spellcheck="false"
                disabled={!session.editable}
                onchange={(e) => commit(setting, e.currentTarget.value)}
              />
            {/if}

            <button
              type="button"
              class="reset"
              disabled={!session.editable || !setting.overridden}
              title={setting.overridden ? "Restore the built-in value" : "Already the default"}
              onclick={() => void session.resetSetting(setting.key)}
            >
              Reset
            </button>

            <span class="origin">{origin(setting)}</span>

            {#each errors as error (error.code + error.message)}
              <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
            {/each}
          </div>
        {/each}
      </fieldset>
    {/each}
  {/if}
</section>

<style>
  fieldset {
    margin-block-end: 1rem;
    margin-inline: 0;
    padding-inline: 0.75rem;
    padding-block: 0.25rem 0.6rem;
    border: 1px solid color-mix(in oklab, currentColor 18%, transparent);
    border-radius: 6px;
  }
  legend {
    padding-inline: 0.35rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.7;
  }
  .row {
    display: grid;
    grid-template-columns: 11rem minmax(6rem, 1fr) auto;
    gap: 0.4rem 0.6rem;
    align-items: center;
    padding-block: 0.2rem;
  }
  label {
    font-size: 0.9rem;
  }
  input[type="text"],
  input[type="number"] {
    inline-size: 100%;
    padding-block: 0.25rem;
    padding-inline: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.9rem;
  }
  input[type="checkbox"] {
    justify-self: start;
  }
  .font-field {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }
  .font-field button {
    flex: none;
    padding-block: 0.2rem;
    padding-inline: 0.5rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .reset {
    font-size: 0.75rem;
    padding-block: 0.15rem;
    padding-inline: 0.45rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .reset:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .origin {
    grid-column: 2 / -1;
    font-size: 0.72rem;
    opacity: 0.55;
  }
  .row.overridden .origin {
    opacity: 0.8;
  }
  .error {
    grid-column: 2 / -1;
    margin: 0;
    font-size: 0.8rem;
    color: #c0392b;
  }
  .empty {
    opacity: 0.65;
  }
  input:disabled {
    opacity: 0.75;
  }
</style>
