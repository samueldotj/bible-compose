<script lang="ts">
  /**
   * Beside the book list: what is in the publication, in what order, and
   * what is in the folder.
   */
  import Inspector from "./ui/Inspector.svelte";
  import { included, isCanonical, restoreCanonical, rows } from "../lib/books.svelte";
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";

  const chosen = $derived(rows().filter((b) => b.included));
  const chapters = $derived(chosen.reduce((n, b) => n + b.chapters, 0));
  const file = (path: string) => path.split(/[/\\]/).pop() ?? path;
</script>

<Inspector kicker={t("navPublication")} title={t("selectionTitle")} description={t("selectionDesc")}>
  <div class="section-title">{t("inPublication")}</div>
  {#if chosen.length === 0}
    <p class="muted note">{t("nothingIncluded")}</p>
  {:else}
    <ul class="chosen">
      {#each chosen as book (book.code)}
        <li>
          <span class="name">{book.name}</span>
          <span class="mono muted">{book.code} · {file(book.path)}</span>
        </li>
      {/each}
    </ul>
    <p class="muted note">{phrases().chaptersInBooks(chapters, chosen.length)}</p>
  {/if}

  <div class="section-title">{t("orderTitle")}</div>
  <div class="row">
    <span>{t("booksFollow")}</span>
    <span class="value">{isCanonical() ? t("canonicalOrder") : t("customOrder")}</span>
  </div>
  <p class="muted note">{t("orderHint")}</p>
  {#if !isCanonical()}
    <button type="button" class="btn small" data-search-key="action:restore-canonical" disabled={!session.editable} onclick={restoreCanonical}>
      {t("restoreCanonical")}
    </button>
  {/if}

  <div class="section-title">{t("filesTitle")}</div>
  <p class="muted note">
    {phrases().booksInFolder(session.books.length)} · {phrases().inPublicationOf(included().size, session.books.length)}
  </p>
  <button type="button" class="btn small" data-search-key="action:open-folder" onclick={() => void session.showFolder()}>
    {t("openProjectFolder")}
  </button>
</Inspector>

<style>
  .chosen {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .chosen li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    padding: 9px 0;
    border-block-end: 1px solid var(--line);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 0;
    border-block-end: 1px solid var(--line);
  }
  .value {
    color: var(--mut);
  }
  .note {
    margin: 0;
    padding: 8px 0;
    font-size: 12px;
  }
  .btn {
    margin-block-start: 6px;
  }
</style>
