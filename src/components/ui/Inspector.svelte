<script lang="ts">
  /**
   * The right-hand pane: the settings for one section.
   *
   * A header that says which section — the rail's group in the italic
   * kicker, the section's name in the serif, a sentence on what it decides
   * — and under it the rows, scrolling on their own so the header stays
   * put.
   */
  import type { Snippet } from "svelte";

  const {
    kicker,
    title,
    description,
    beside,
    children,
  }: {
    kicker: string;
    title: string;
    description?: string;
    /** Something beside the title: a selector chip, a count. */
    beside?: Snippet;
    children: Snippet;
  } = $props();
</script>

<aside class="inspector" aria-label={title}>
  <header>
    <div class="kicker">{kicker}</div>
    <div class="titles">
      <h2>{title}</h2>
      {#if beside}{@render beside()}{/if}
    </div>
    {#if description}
      <p class="desc">{description}</p>
    {/if}
  </header>
  <div class="body">
    {@render children()}
  </div>
</aside>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    flex: none;
    inline-size: var(--inspector-w);
    min-block-size: 0;
    border-inline-start: 1px solid var(--line);
    background: var(--panel);
  }
  header {
    flex: none;
    padding: 18px 22px 14px;
    border-block-end: 1px solid var(--line);
  }
  .titles {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-block-start: 2px;
  }
  h2 {
    font: 500 24px/1.15 var(--serif);
  }
  .desc {
    margin: 6px 0 0;
    color: var(--mut);
  }
  .body {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0 22px 16px;
  }
</style>
