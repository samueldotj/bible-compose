<script lang="ts">
  /**
   * STY-008: for any element, what each property is and where it came from.
   *
   * "Any element" is the part the editor cannot do. The editor curates about
   * twenty-five selectors because a form over all hundred and forty is a
   * spreadsheet; this shows every one of them, read-only, including the ones
   * with nothing set — because "nothing decides this" is the answer a
   * publisher wondering why a paragraph looks like body text is after.
   *
   * Free, as ADR-005 predicted. Every value already carried its origin from
   * the moment it was resolved, so this is a read rather than a mechanism.
   */
  import { session } from "../lib/session.svelte";
  import { ALL_PROPERTIES, labelForSelector } from "../lib/styles";
  import type { StyleProperty } from "../lib/services/backend";

  const selectors = $derived(
    session.styles
      .map((s) => s.selector)
      .filter((s) => {
        const needle = session.inspectFilter.trim().toLowerCase();
        if (!needle) return true;
        return (
          s.toLowerCase().includes(needle) ||
          (labelForSelector(s) ?? "").toLowerCase().includes(needle)
        );
      }),
  );

  const chosen = $derived(
    session.inspected && selectors.includes(session.inspected)
      ? session.inspected
      : (selectors[0] ?? null),
  );

  const properties = $derived(
    new Map(
      (session.styles.find((s) => s.selector === chosen)?.properties ?? []).map((p) => [p.name, p]),
    ),
  );

  function origin(p: StyleProperty | undefined): string {
    if (!p) return "not set";
    if (p.origin === "inherited") return `inherited from ${p.from ?? "another style"}`;
    if (p.origin === "file") {
      const at = p.location;
      return at?.line ? `styles.toml:${at.line}` : "set in this project";
    }
    return "built-in default";
  }
</script>

<section class="pane" aria-label="Style inspector">
  <div class="split">
    <div class="list">
      <input
        type="search"
        placeholder="Filter selectors"
        spellcheck="false"
        aria-label="Filter selectors"
        bind:value={session.inspectFilter}
      />
      <ul>
        {#each selectors as selector (selector)}
          <li>
            <button
              type="button"
              class:selected={chosen === selector}
              aria-current={chosen === selector ? "true" : undefined}
              onclick={() => (session.inspected = selector)}
            >
              <span class="key">{selector}</span>
              {#if labelForSelector(selector)}
                <span class="named">{labelForSelector(selector)}</span>
              {/if}
            </button>
          </li>
        {:else}
          <li class="none">Nothing matches.</li>
        {/each}
      </ul>
    </div>

    <div class="detail">
      {#if !chosen}
        <p class="none">No element selected.</p>
      {:else}
        <h3>{chosen}</h3>
        <table>
          <thead>
            <tr><th>Property</th><th>Value</th><th>From</th></tr>
          </thead>
          <tbody>
            {#each ALL_PROPERTIES as property (property.name)}
              {@const p = properties.get(property.name)}
              <tr class:unset={!p}>
                <td>{property.label}</td>
                <td class="value">{p?.value ?? "—"}</td>
                <td class="from">
                  {#if p?.origin === "inherited" && p.from}
                    <!-- The chain is the answer more often than the file is,
                         so it is walkable rather than only readable. -->
                    <button type="button" class="link" onclick={() => (session.inspected = p.from!)}>
                      {origin(p)}
                    </button>
                  {:else}
                    {origin(p)}
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
</section>

<style>
  .split {
    display: grid;
    grid-template-columns: minmax(10rem, 16rem) 1fr;
    gap: 1rem;
    align-items: start;
  }
  input[type="search"] {
    inline-size: 100%;
    padding-block: 0.25rem;
    padding-inline: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
  }
  ul {
    list-style: none;
    margin-block: 0.4rem 0;
    padding: 0;
    /* Its own scroller: the list is long and the detail beside it is not, so
       they should not scroll together. */
    max-block-size: 24rem;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .list button {
    display: block;
    inline-size: 100%;
    padding-block: 0.2rem;
    padding-inline: 0.35rem;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .list button:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .list button.selected {
    background: color-mix(in oklab, currentColor 14%, transparent);
  }
  .key {
    display: block;
    font-family: ui-monospace, monospace;
    font-size: 0.78rem;
  }
  .named {
    display: block;
    font-size: 0.7rem;
    opacity: 0.55;
  }
  h3 {
    margin-block: 0 0.5rem;
    font-family: ui-monospace, monospace;
    font-size: 0.9rem;
  }
  table {
    inline-size: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }
  th {
    text-align: start;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.6;
    padding-block-end: 0.25rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 18%, transparent);
  }
  td {
    padding-block: 0.25rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 8%, transparent);
    vertical-align: baseline;
  }
  tr.unset td {
    opacity: 0.45;
  }
  .value {
    font-family: ui-monospace, monospace;
  }
  .from {
    opacity: 0.75;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline dotted;
    cursor: pointer;
  }
  .none {
    opacity: 0.6;
    font-size: 0.85rem;
  }
</style>
