<script lang="ts">
  /**
   * What a head or foot slot can say, in a dialog.
   *
   * Every field a template can name, from the table the backend checks
   * templates against — so this documents exactly what the file accepts,
   * and a field added there appears here without anyone remembering to say
   * so.
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
      event.stopPropagation();
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
  <div class="dialog" role="dialog" aria-modal="true" aria-label={t("headFieldsTitle")} tabindex="-1" use:modal>
    <h2>{t("headFieldsTitle")}</h2>
    <p class="note muted">{t("headFieldsNote")}</p>
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
              <td><span class="mono">{`{${field.name}}`}</span></td>
              <td>{field.description}</td>
              <td>{field.example}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="note muted">{t("loading")}</p>
    {/if}
    <footer>
      <button type="button" class="btn primary" onclick={onclose}>{t("close")}</button>
    </footer>
  </div>
</div>

<style>
  .dialog {
    inline-size: min(46rem, 92vw);
    max-block-size: 90vh;
    overflow-y: auto;
  }
  .note {
    margin: 0;
    line-height: 1.5;
  }
  table {
    inline-size: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 6px 12px 6px 0;
    text-align: start;
    vertical-align: top;
    border-block-end: 1px solid var(--line);
  }
  th {
    font: italic 500 12.5px var(--serif);
    color: var(--mut);
  }
  .mono {
    white-space: nowrap;
  }
</style>
