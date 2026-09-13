<script lang="ts">
  /**
   * GUI-004 and STY-005: how one kind of element looks, editable and
   * persisted, with the element outlined on the page beside it.
   *
   * Each row shows the value in force and where it came from — the built-in
   * set, this project's file, or the style it inherits from. That last one
   * is the whole reason ADR-005 made `Inherited` a distinct origin: "why
   * does this look like this" is usually answered by the inheritance.
   * Reset is offered only where there is something to reset.
   */
  import FontPicker from "./FontPicker.svelte";
  import Inspector from "./ui/Inspector.svelte";
  import Segmented from "./ui/Segmented.svelte";
  import SettingRow from "./ui/SettingRow.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import { GROUPS, STYLE_TABS, subTabTitle } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { ALIGNMENTS, STYLE_GROUPS, type PropertyRow } from "../lib/styles";
  import type { StyleProperty } from "../lib/services/backend";
  import { shownStyle, ui } from "../lib/ui.svelte";
  import { locale, phrases, t, word } from "../lib/i18n";

  const sub = $derived(STYLE_TABS.find((x) => x.id === session.stylePane) ?? STYLE_TABS[0]!);
  const group = $derived(STYLE_GROUPS.find((g) => sub.styleGroups.includes(g.id)) ?? null);
  const selector = $derived(shownStyle());
  const row = $derived(group?.rows.find((r) => r.selector === selector) ?? null);
  const settingGroups = $derived(GROUPS.filter((g) => sub.settingGroups.includes(g.id)));

  const bySelector = $derived(new Map(session.styles.map((s) => [s.selector, s])));
  function held(property: string): StyleProperty | undefined {
    return selector ? bySelector.get(selector)?.properties.find((p) => p.name === property) : undefined;
  }

  function origin(p: StyleProperty | undefined): string {
    if (!p) return t("notSet");
    if (p.origin === "inherited") return phrases().inheritedFrom(p.from ?? "");
    if (p.origin === "file") {
      const at = p.location;
      return at?.line ? `styles.toml:${at.line} · ${t("setInProject")}` : t("setInProject");
    }
    return t("builtInDefault");
  }

  function commit(property: PropertyRow, value: string): void {
    if (!selector) return;
    const current = held(property.name);
    if (current && value === current.value) return;
    void session.setStyle(selector, property.name, value);
  }

  const key = (name: string) => `style:${selector}.${name}`;
  let picking = $state<string | null>(null);
  const UNSET_COLOR = "#000000";
</script>

<Inspector
  kicker={`${t("navType")} › ${subTabTitle(sub)}`}
  title={row ? word(`style:${row.selector}`, row.label) : subTabTitle(sub)}
  description={row ? t("stylesDesc") : sub.inspector ? t("inspectDesc") : locale().help[`subtab:${sub.id}`]}
>
  {#snippet beside()}
    {#if row}<span class="chip mono">{row.selector}</span>{/if}
  {/snippet}

  {#if sub.inspector}
    <div class="section-title">{t("filterSelectors")}</div>
    <input
      type="search"
      class="input full"
      placeholder={t("filterSelectors")}
      aria-label={t("filterSelectors")}
      spellcheck="false"
      bind:value={session.inspectFilter}
    />
  {/if}

  {#each settingGroups as g (g.id)}
    <div class="section-title">{word(`group:${g.id}`, g.title)}</div>
    {#each g.keys as k (k)}
      <SettingRow key={k} />
    {/each}
  {/each}

  {#if group}
    <!-- The group's elements, one of them chosen. -->
    <div class="pills" role="tablist" aria-label={group.title}>
      {#each group.rows as r (r.selector)}
        <button
          type="button"
          role="tab"
          class="pill"
          class:active={r.selector === selector}
          aria-selected={r.selector === selector}
          data-search-key={`style:${r.selector}`}
          onclick={() => (ui.style = r.selector)}
        >
          {word(`style:${r.selector}`, r.label)}
        </button>
      {/each}
    </div>

    {#if row && selector}
      {#each row.properties as property (property.name)}
        {@const p = held(property.name)}
        {@const id = `sty-${selector}-${property.name}`}
        {@const errors = session.styleErrors[`${selector}.${property.name}`] ?? []}
        <div class="prow" class:set={p?.origin === "file"} data-search-key={key(property.name)}>
          <div class="main">
            <label for={id}>{word(`property:${property.name}`, property.label)}</label>
            {#if property.kind === "boolean"}
              <Toggle
                checked={p?.value === "true"}
                disabled={!session.editable}
                label={property.label}
                onchange={(on) => commit(property, on ? "true" : "false")}
              />
            {:else if property.kind === "choice"}
              <span class="select-wrap">
                <select {id} class="input" value={p?.value ?? property.choices?.[0]?.value ?? ""} disabled={!session.editable} onchange={(e) => commit(property, e.currentTarget.value)}>
                  {#each property.choices ?? [] as choice (choice.value)}
                    <option value={choice.value}>{choice.label}</option>
                  {/each}
                </select>
              </span>
            {:else if property.kind === "align"}
              <Segmented
                small
                options={ALIGNMENTS.map((a) => ({ value: a, label: word(`align:${a}`, a) }))}
                value={p?.value ?? "start"}
                label={property.label}
                disabled={!session.editable}
                onchange={(a) => commit(property, a)}
              />
            {:else if property.kind === "font"}
              <span class="pair">
                <input
                  {id}
                  class="input text"
                  type="text"
                  value={p?.value ?? ""}
                  placeholder={t("theBodyFont")}
                  spellcheck="false"
                  disabled={!session.editable}
                  onchange={(e) => commit(property, e.currentTarget.value)}
                />
                <button type="button" class="btn small" disabled={!session.editable} onclick={() => (picking = property.name)}>{t("choose")}</button>
              </span>
              {#if picking === property.name}
                <FontPicker current={p?.value ?? ""} onchoose={(family) => commit(property, family)} onclose={() => (picking = null)} />
              {/if}
            {:else if property.kind === "color"}
              <span class="pair">
                <input
                  type="color"
                  class="swatch"
                  value={p?.value ?? UNSET_COLOR}
                  disabled={!session.editable}
                  aria-label={phrases().colourSwatch(property.label)}
                  oninput={(e) => commit(property, e.currentTarget.value)}
                />
                <input
                  {id}
                  class="input count"
                  type="text"
                  value={p?.value ?? ""}
                  placeholder={t("unset")}
                  spellcheck="false"
                  disabled={!session.editable}
                  onchange={(e) => commit(property, e.currentTarget.value)}
                />
              </span>
            {:else}
              <input
                {id}
                class="input count"
                type="text"
                value={p?.value ?? ""}
                placeholder={property.kind === "integer" ? "400" : t("unset")}
                spellcheck="false"
                disabled={!session.editable}
                onchange={(e) => commit(property, e.currentTarget.value)}
              />
            {/if}
          </div>
          <div class="origin">
            <span class:mono={p?.origin === "file"}>{origin(p)}</span>
            {#if p?.origin === "file" && session.editable}
              <button type="button" class="link" onclick={() => void session.resetStyle(selector, property.name)}>{t("reset")}</button>
            {/if}
          </div>
          {#each errors as error (error.code + error.message)}
            <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
          {/each}
        </div>
      {/each}
    {/if}
  {/if}
</Inspector>

<style>
  .chip {
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--line);
    color: var(--mut);
  }
  .full {
    inline-size: 100%;
    margin-block-start: 8px;
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 14px 0 10px;
  }
  .pill {
    padding: 4px 10px;
    border: 1px solid var(--line2);
    border-radius: 999px;
    background: var(--field);
    color: var(--ink);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .pill.active {
    border-color: var(--acc);
    background: var(--acctint);
    color: var(--acctext);
    font-weight: 600;
  }
  .prow {
    padding: 9px 10px;
    margin-inline: -10px;
    border-block-end: 1px solid var(--line);
    border-radius: var(--radius);
  }
  .prow.set {
    background: color-mix(in oklab, var(--acctint) 45%, transparent);
  }
  .main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .prow.set label {
    font-weight: 600;
    color: var(--acctext);
  }
  .pair {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }
  .text {
    inline-size: 140px;
  }
  .count {
    inline-size: 96px;
  }
  .swatch {
    inline-size: 30px;
    block-size: 30px;
    padding: 0;
    border: 1px solid var(--line2);
    border-radius: var(--radius);
    background: var(--field);
    cursor: pointer;
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
