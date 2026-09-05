<script lang="ts">
  /**
   * The Headers & Footers tab: a spread.
   *
   * A book is read two pages at a time, and the two are not the same page —
   * the outer edge of the left one is its left, and of the right one its
   * right, which is why a page number sits at the left of one and the right
   * of the other. So the tab shows both, side by side as a reader meets them,
   * each with its own six controls above and below. Left-hand pages are the
   * even-numbered ones, right-hand pages the odd, and the example's own page
   * numbers say so.
   */
  import ExamplePage from "./ExamplePage.svelte";
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";

  $effect(() => {
    void session.loadHeadFields();
  });
</script>

<div class="spread">
  <ExamplePage which="headers" side="left" />
  <ExamplePage which="headers" side="right" />
</div>

<!-- Every field a slot can name, from the table the backend checks templates
     against — so this documents exactly what the file accepts, and a field
     added there appears here without anyone remembering to say so. -->
<section class="fields" aria-label={t("headFieldsTitle")}>
  <h3>{t("headFieldsTitle")}</h3>
  <p class="note">{t("headFieldsNote")}</p>
  {#if session.headFields}
    <table>
      <thead>
        <tr>
          <th>{t("fieldColumn")}</th>
          <th>{t("meaningColumn")}</th>
          <th>{t("exampleColumn")}</th>
        </tr>
      </thead>
      <tbody>
        {#each session.headFields as field (field.name)}
          <tr>
            <td><code>{`{${field.name}}`}</code></td>
            <td>{field.description}</td>
            <td>{field.example}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  /* Two columns of one row, the row being all the height the tab hands
     down: each page then sizes its type to the half it has. */
  .spread {
    flex: 1;
    min-block-size: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    gap: 1rem;
  }
  .fields {
    flex: none;
    margin-block-start: 0.8rem;
    font-size: 0.85rem;
  }
  .fields h3 {
    margin: 0 0 0.2rem;
    font-size: 0.95rem;
  }
  .note {
    margin: 0 0 0.5rem;
    opacity: 0.75;
    max-inline-size: 70rem;
  }
  table {
    border-collapse: collapse;
    inline-size: 100%;
  }
  th,
  td {
    padding: 0.2rem 0.6rem 0.2rem 0;
    text-align: start;
    vertical-align: top;
    border-block-end: 1px solid color-mix(in oklab, currentColor 12%, transparent);
  }
  th {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.6;
    font-weight: 600;
  }
  code {
    font-family: ui-monospace, Consolas, monospace;
    white-space: nowrap;
  }
</style>
