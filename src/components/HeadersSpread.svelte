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
</script>

<div class="spread">
  <ExamplePage which="headers" side="left" />
  <ExamplePage which="headers" side="right" />
</div>


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
</style>
