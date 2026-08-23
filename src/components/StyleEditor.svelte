<script lang="ts">
  /**
   * GUI-004 and STY-005: how the book looks, editable and persisted.
   *
   * Each row shows the value in force and where it came from — the built-in
   * set, this project's file, or the style it inherits from. That last one is
   * the whole reason ADR-005 made `Inherited` a distinct origin: "why does
   * this look like this" is usually answered by the inheritance, and a form
   * that said "default" would be answering a different question.
   *
   * Reset is offered only where there is something to reset. A value that came
   * from the built-in set or by inheritance is already what the cascade would
   * decide.
   */
  import FontPicker from "./FontPicker.svelte";
  import { session } from "../lib/session.svelte";
  import { ALIGNMENTS, STYLE_GROUPS, type PropertyRow } from "../lib/styles";
  import type { StyleProperty } from "../lib/services/backend";

  /** Which of `STYLE_GROUPS` to show. The tab decides. */
  const { groups: show }: { groups: readonly string[] } = $props();

  /** Which style's font is being picked, as `selector.property`. */
  let picking = $state<string | null>(null);

  /**
   * A colour swatch needs something to show when nothing is set, and black is
   * both the honest answer — unset ink is the body colour — and the one a
   * publisher opening the control expects to see.
   */
  const UNSET_COLOR = "#000000";

  const shown = $derived(STYLE_GROUPS.filter((g) => show.includes(g.id)));
  const bySelector = $derived(new Map(session.styles.map((s) => [s.selector, s])));

  function held(selector: string, property: string): StyleProperty | undefined {
    return bySelector.get(selector)?.properties.find((p) => p.name === property);
  }

  function origin(p: StyleProperty | undefined): string {
    if (!p) return "not set";
    if (p.origin === "inherited") return `inherited from ${p.from ?? "another style"}`;
    if (p.origin === "file") {
      const at = p.location;
      return at?.line ? `styles.toml:${at.line}` : "set in this project";
    }
    return "built-in default";
  }

  function commit(selector: string, row: PropertyRow, value: string): void {
    const current = held(selector, row.name);
    if (current && value === current.value) return;
    void session.setStyle(selector, row.name, value);
  }

  function key(selector: string, name: string): string {
    return `${selector}.${name}`;
  }
</script>

<section class="pane" aria-label="Styles">
  {#if session.styles.length === 0}
    <p class="empty">Loading…</p>
  {:else}
    {#each shown as group (group.id)}
      <fieldset>
        <legend>{group.title}</legend>

        {#each group.rows as style (style.selector)}
          <div class="style">
            <h3>{style.label} <span class="selector">{style.selector}</span></h3>

            {#each style.properties as property (property.name)}
              {@const p = held(style.selector, property.name)}
              {@const id = `sty-${key(style.selector, property.name)}`}
              {@const errors = session.styleErrors[key(style.selector, property.name)] ?? []}
              <div class="row">
                <label for={id}>{property.label}</label>

                {#if property.kind === "boolean"}
                  <input
                    {id}
                    type="checkbox"
                    checked={p?.value === "true"}
                    disabled={!session.editable}
                    onchange={(e) =>
                      commit(style.selector, property, e.currentTarget.checked ? "true" : "false")}
                  />
                {:else if property.kind === "align"}
                  <select
                    {id}
                    value={p?.value ?? "start"}
                    disabled={!session.editable}
                    onchange={(e) => commit(style.selector, property, e.currentTarget.value)}
                  >
                    {#each ALIGNMENTS as option (option)}
                      <option value={option}>{option}</option>
                    {/each}
                  </select>
                {:else if property.kind === "font"}
                  <!--
                    The body font when nothing is set, which is what the
                    cascade means by unset — shown as a placeholder rather
                    than as a value, so Reset stays meaningful.
                  -->
                  <span class="pair">
                    <input
                      {id}
                      type="text"
                      value={p?.value ?? ""}
                      placeholder="the body font"
                      spellcheck="false"
                      disabled={!session.editable}
                      onchange={(e) => commit(style.selector, property, e.currentTarget.value)}
                    />
                    <button
                      type="button"
                      onclick={() => (picking = key(style.selector, property.name))}
                    >
                      Choose…
                    </button>
                  </span>
                  {#if picking === key(style.selector, property.name)}
                    <FontPicker
                      current={p?.value ?? ""}
                      onchoose={(family) => commit(style.selector, property, family)}
                      onclose={() => (picking = null)}
                    />
                  {/if}
                {:else if property.kind === "color"}
                  <!--
                    Swatch and text together. The swatch is how a colour gets
                    chosen; the text is how `#c81414` gets typed in from a
                    brand sheet, and how it can be cleared back to unset.
                  -->
                  <span class="pair">
                    <input
                      type="color"
                      class="swatch"
                      value={p?.value ?? UNSET_COLOR}
                      disabled={!session.editable}
                      aria-label={`${property.label} swatch`}
                      oninput={(e) => commit(style.selector, property, e.currentTarget.value)}
                    />
                    <input
                      {id}
                      type="text"
                      value={p?.value ?? ""}
                      placeholder="unset"
                      spellcheck="false"
                      disabled={!session.editable}
                      onchange={(e) => commit(style.selector, property, e.currentTarget.value)}
                    />
                  </span>
                {:else}
                  <input
                    {id}
                    type="text"
                    value={p?.value ?? ""}
                    placeholder={property.kind === "integer" ? "400" : "unset"}
                    spellcheck="false"
                    disabled={!session.editable}
                    onchange={(e) => commit(style.selector, property, e.currentTarget.value)}
                  />
                {/if}

                <button
                  type="button"
                  class="reset"
                  disabled={!session.editable || p?.origin !== "file"}
                  title={p?.origin === "file"
                    ? "Restore what the cascade would decide"
                    : "This project has not set it"}
                  onclick={() => void session.resetStyle(style.selector, property.name)}
                >
                  Reset
                </button>

                <span class="origin" class:set={p?.origin === "file"}>{origin(p)}</span>

                {#each errors as error (error.code + error.message)}
                  <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
                {/each}
              </div>
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
  .style {
    padding-block: 0.35rem;
  }
  .style + .style {
    border-block-start: 1px solid color-mix(in oklab, currentColor 10%, transparent);
  }
  h3 {
    margin-block: 0.2rem;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .selector {
    font-weight: 400;
    font-size: 0.75rem;
    opacity: 0.45;
    font-family: ui-monospace, monospace;
  }
  .row {
    display: grid;
    grid-template-columns: 8rem minmax(5rem, 1fr) auto;
    gap: 0.3rem 0.6rem;
    align-items: center;
    padding-block: 0.15rem;
  }
  label {
    font-size: 0.85rem;
    opacity: 0.85;
  }
  .pair {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }
  .pair button {
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
  .swatch {
    flex: none;
    inline-size: 2.2rem;
    block-size: 1.5rem;
    padding: 0;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
  }
  input[type="text"],
  select {
    inline-size: 100%;
    padding-block: 0.2rem;
    padding-inline: 0.35rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    font: inherit;
    font-size: 0.85rem;
  }
  input[type="checkbox"] {
    justify-self: start;
  }
  .reset {
    font-size: 0.72rem;
    padding-block: 0.1rem;
    padding-inline: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .reset:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .origin {
    grid-column: 2 / -1;
    font-size: 0.7rem;
    opacity: 0.5;
  }
  .origin.set {
    opacity: 0.8;
  }
  .error {
    grid-column: 2 / -1;
    margin: 0;
    font-size: 0.78rem;
    color: #c0392b;
  }
  .empty {
    opacity: 0.65;
  }
  input:disabled,
  select:disabled {
    opacity: 0.75;
  }
</style>
