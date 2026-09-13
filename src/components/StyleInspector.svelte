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
  import { phrases, t, word } from "../lib/i18n";

  const selectors = $derived(
    session.styles
      .map((s) => s.selector)
      .filter((s) => {
        const needle = session.inspectFilter.trim().toLowerCase();
        if (!needle) return true;
        return s.toLowerCase().includes(needle) || (labelForSelector(s) ?? "").toLowerCase().includes(needle);
      }),
  );

  const chosen = $derived(
    session.inspected && selectors.includes(session.inspected) ? session.inspected : (selectors[0] ?? null),
  );

  const properties = $derived(
    new Map((session.styles.find((s) => s.selector === chosen)?.properties ?? []).map((p) => [p.name, p])),
  );

  function origin(p: StyleProperty | undefined): string {
    if (!p) return t("notSet");
    if (p.origin === "inherited") return phrases().inheritedFrom(p.from ?? "");
    if (p.origin === "file") {
      const at = p.location;
      return at?.line ? `styles.toml:${at.line}` : t("setInProject");
    }
    return t("builtInDefault");
  }
</script>

<section class="inspect" aria-label={t("styleInspector")}>
  <div class="list">
    <ul>
      {#each selectors as selector (selector)}
        <li>
          <button
            type="button"
            class:selected={chosen === selector}
            aria-current={chosen === selector ? "true" : undefined}
            onclick={() => (session.inspected = selector)}
          >
            <span class="mono key">{selector}</span>
            {#if labelForSelector(selector)}
              <span class="named">{labelForSelector(selector)}</span>
            {/if}
          </button>
        </li>
      {:else}
        <li class="none muted">{t("nothingMatches")}</li>
      {/each}
    </ul>
  </div>

  <div class="detail">
    {#if !chosen}
      <p class="muted">{t("noElementSelected")}</p>
    {:else}
      <h2><span class="mono">{chosen}</span></h2>
      {#if labelForSelector(chosen)}<p class="muted">{labelForSelector(chosen)}</p>{/if}
      <table>
        <thead>
          <tr><th>{t("property")}</th><th>{t("value")}</th><th>{t("from")}</th></tr>
        </thead>
        <tbody>
          {#each ALL_PROPERTIES as property (property.name)}
            {@const p = properties.get(property.name)}
            <tr class:unset={!p}>
              <td>{word(`property:${property.name}`, property.label)}</td>
              <td class="mono">{p?.value ?? "—"}</td>
              <td class="from">
                {#if p?.origin === "inherited" && p.from}
                  <!-- The chain is the answer more often than the file is,
                       so it is walkable rather than only readable. -->
                  <button type="button" class="link" onclick={() => (session.inspected = p.from!)}>{origin(p)}</button>
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
</section>

<style>
  .inspect {
    display: grid;
    grid-template-columns: minmax(180px, 260px) 1fr;
    gap: 24px;
    flex: 1;
    min-block-size: 0;
    padding: 22px 28px;
  }
  .list {
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel);
  }
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
  }
  .list button {
    display: block;
    inline-size: 100%;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .list button:hover {
    background: color-mix(in oklab, var(--acctint) 55%, transparent);
  }
  .list button.selected {
    background: var(--acctint);
    color: var(--acctext);
  }
  .key {
    display: block;
  }
  .named {
    display: block;
    font-size: 11.5px;
    color: var(--mut);
  }
  .none {
    padding: 8px 10px;
  }
  .detail {
    min-block-size: 0;
    overflow-y: auto;
  }
  h2 {
    font: 500 22px/1.2 var(--serif);
  }
  h2 .mono {
    font-size: 16px;
  }
  table {
    inline-size: 100%;
    margin-block-start: 12px;
    border-collapse: collapse;
  }
  th {
    padding: 8px 12px 8px 0;
    border-block-end: 1px solid var(--line2);
    text-align: start;
    font: italic 500 12.5px var(--serif);
    color: var(--mut);
  }
  td {
    padding: 8px 12px 8px 0;
    border-block-end: 1px solid var(--line);
    vertical-align: baseline;
  }
  tr.unset td {
    color: var(--mut);
  }
  .from {
    color: var(--mut);
  }
</style>
