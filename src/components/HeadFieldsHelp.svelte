<script lang="ts">
  /**
   * What a head or foot slot can say, in a dialog.
   *
   * Every field a template can name, from the table the backend checks
   * templates against — so this documents exactly what the file accepts,
   * and a field added there appears here without anyone remembering to say
   * so. A dialog rather than a table under the spread: the reader who wants
   * it is the one with the Custom box open, and everyone else had a screen
   * of reference material under their page.
   */
  import { session } from "../lib/session.svelte";
  import { t } from "../lib/i18n";
  import { modal } from "../lib/modal";

  const { onclose }: { onclose: () => void } = $props();

  $effect(() => {
    void session.loadHeadFields();
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window {onkeydown} />

<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={t("headFieldsTitle")}
    tabindex="-1"
    use:modal
  >
    <h2>{t("headFieldsTitle")}</h2>
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
    {:else}
      <p class="note">{t("loading")}</p>
    {/if}
    <footer>
      <button type="button" class="primary" onclick={onclose}>{t("close")}</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.4);
    z-index: 10;
  }
  .dialog {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    inline-size: min(46rem, 92vw);
    max-block-size: 90vh;
    padding: 1.1rem 1.3rem;
    border-radius: 8px;
    background: Canvas;
    color: CanvasText;
    box-shadow: 0 8px 30px rgb(0 0 0 / 0.35);
    font-size: 0.85rem;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  .note {
    margin: 0;
    opacity: 0.8;
  }
  table {
    border-collapse: collapse;
    inline-size: 100%;
    overflow-y: auto;
  }
  th,
  td {
    padding: 0.25rem 0.6rem 0.25rem 0;
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
  footer {
    display: flex;
    justify-content: end;
    margin-block-start: 0.3rem;
  }
</style>
