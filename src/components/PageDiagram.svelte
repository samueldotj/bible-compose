<script lang="ts">
  /**
   * The page, drawn to scale, with every measurement on it.
   *
   * The page settings were nine numbers in a column. "Inner margin" and
   * "outer margin" are not self-explanatory to anyone who has not set a book
   * before — and worse, they are not checkable: a publisher reading `0.66in`
   * and `0.48in` cannot tell from the form that the wider one is against the
   * spine, which is the whole point of having two.
   *
   * So: a spread, because inner and outer only mean anything across a fold.
   * Two facing pages, verso then recto, with the gutter between them and the
   * type area, columns, running head and folio drawn where they will actually
   * fall. Change a number and the drawing changes with it.
   *
   * The dimensions come from the shell in points, already parsed. A second
   * unit parser here would be a second answer to what `13.97mm` is.
   */
  import type { Geometry } from "../lib/services/backend";

  const { geometry: g, highlight }: { geometry: Geometry; highlight?: string | null } = $props();

  /** Room for the dimension labels drawn outside the paper. */
  const PAD = 34;
  const GUTTER = 10;

  const scale = $derived(Math.min(1, 250 / g.pageHeight));
  const w = $derived(g.pageWidth * scale);
  const h = $derived(g.pageHeight * scale);

  const view = $derived({
    width: w * 2 + GUTTER + PAD * 2,
    height: h + PAD * 2,
  });

  /** Verso (left) and recto (right) origins. */
  const versoX = $derived(PAD);
  const rectoX = $derived(PAD + w + GUTTER);
  const top = PAD;

  const s = (points: number) => points * scale;

  /**
   * The type area of one page.
   *
   * `inner` is the margin against the spine, which is the right-hand edge of
   * the verso and the left-hand edge of the recto — the mirroring is the
   * thing the drawing exists to show.
   */
  function block(x: number, side: "verso" | "recto") {
    const left = side === "recto" ? s(g.marginInner) : s(g.marginOuter);
    const right = side === "recto" ? s(g.marginOuter) : s(g.marginInner);
    return {
      x: x + left,
      y: top + s(g.marginTop),
      width: w - left - right,
      height: h - s(g.marginTop) - s(g.marginBottom),
    };
  }

  const recto = $derived(block(rectoX, "recto"));
  const verso = $derived(block(versoX, "verso"));

  /** Column rectangles inside a type area. */
  function columns(area: { x: number; y: number; width: number; height: number }) {
    const n = Math.max(1, g.columns);
    const gap = s(g.columnGap);
    const each = (area.width - gap * (n - 1)) / n;
    return Array.from({ length: n }, (_, i) => ({
      x: area.x + i * (each + gap),
      width: each,
    }));
  }

  const lit = (key: string) => highlight === key;

  const inches = (points: number) => `${(points / 72).toFixed(2)}in`;
</script>

<figure class:has-highlight={highlight != null}>
  <svg viewBox={`0 0 ${view.width} ${view.height}`} role="img" aria-label="The page, to scale">
    <!-- The sheets. -->
    {#each [versoX, rectoX] as x (x)}
      <rect class="paper" {x} y={top} width={w} height={h} rx="1" />
    {/each}

    <!-- The fold. Drawn, not implied: it is what inner and outer refer to. -->
    <line
      class="spine"
      x1={PAD + w + GUTTER / 2}
      y1={top - 6}
      x2={PAD + w + GUTTER / 2}
      y2={top + h + 6}
    />

    {#each [verso, recto] as area (area.x)}
      <!-- The type area, and the columns inside it. -->
      <rect
        class="block"
        class:lit={lit("page.size")}
        x={area.x}
        y={area.y}
        width={area.width}
        height={area.height}
      />
      {#each columns(area) as col, i (i)}
        <rect
          class="column"
          class:lit={lit("page.columns")}
          x={col.x}
          y={area.y}
          width={col.width}
          height={area.height}
        />
      {/each}

      {#if g.columns > 1}
        {#each columns(area).slice(0, -1) as col, i (i)}
          <rect
            class="gap"
            class:lit={lit("page.column_gap")}
            x={col.x + col.width}
            y={area.y}
            width={s(g.columnGap)}
            height={area.height}
          />
        {/each}
      {/if}

      <!-- Running head and folio: bands, because their gap is what is set. -->
      <rect
        class="furniture"
        class:lit={lit("page.header_gap")}
        x={area.x}
        y={area.y - s(g.headerGap)}
        width={area.width}
        height={s(g.headerGap)}
      />
      <rect
        class="furniture"
        class:lit={lit("page.footer_gap")}
        x={area.x}
        y={area.y + area.height}
        width={area.width}
        height={s(g.footerGap)}
      />
    {/each}

    <!-- Margins, marked on the recto only: marking both would double every
         label and say nothing the mirroring has not already said. -->
    <g class="marks">
      <!-- Top -->
      <line
        class:lit={lit("page.margin_top")}
        x1={recto.x + recto.width / 2}
        y1={top}
        x2={recto.x + recto.width / 2}
        y2={recto.y}
      />
      <text
        class:lit={lit("page.margin_top")}
        x={recto.x + recto.width / 2 + 3}
        y={top + s(g.marginTop) / 2 + 3}
      >
        {inches(g.marginTop)}
      </text>

      <!-- Bottom -->
      <line
        class:lit={lit("page.margin_bottom")}
        x1={recto.x + recto.width / 2}
        y1={recto.y + recto.height}
        x2={recto.x + recto.width / 2}
        y2={top + h}
      />
      <text
        class:lit={lit("page.margin_bottom")}
        x={recto.x + recto.width / 2 + 3}
        y={top + h - s(g.marginBottom) / 2 + 3}
      >
        {inches(g.marginBottom)}
      </text>

      <!-- Inner, against the spine. -->
      <line
        class:lit={lit("page.margin_inner")}
        x1={rectoX}
        y1={top + h + 10}
        x2={recto.x}
        y2={top + h + 10}
      />
      <text
        class:lit={lit("page.margin_inner")}
        class="below"
        x={rectoX + s(g.marginInner) / 2}
        y={top + h + 22}
      >
        inner
      </text>

      <!-- Outer, at the fore-edge. -->
      <line
        class:lit={lit("page.margin_outer")}
        x1={recto.x + recto.width}
        y1={top + h + 10}
        x2={rectoX + w}
        y2={top + h + 10}
      />
      <text
        class:lit={lit("page.margin_outer")}
        class="below"
        x={recto.x + recto.width + s(g.marginOuter) / 2}
        y={top + h + 22}
      >
        outer
      </text>

      <!-- The trim, up the fore-edge of the verso. -->
      <text class:lit={lit("page.size")} class="trim" x={PAD - 8} y={top + h / 2}>
        {inches(g.pageHeight)}
      </text>
      <text
        class:lit={lit("page.size")}
        class="below"
        x={versoX + w / 2}
        y={top + h + 22}
      >
        {inches(g.pageWidth)}
      </text>
    </g>
  </svg>

  <figcaption>
    A spread, to scale — verso and recto, with the fold between them. The inner
    margin is the one against the spine, so it changes sides from page to page.
  </figcaption>
</figure>

<style>
  figure {
    margin: 0 0 1rem;
  }
  svg {
    inline-size: 100%;
    block-size: auto;
    max-block-size: 20rem;
  }
  .paper {
    fill: color-mix(in oklab, Canvas 92%, CanvasText);
    stroke: color-mix(in oklab, currentColor 35%, transparent);
    stroke-width: 0.7;
  }
  .spine {
    stroke: color-mix(in oklab, currentColor 30%, transparent);
    stroke-width: 0.7;
    stroke-dasharray: 3 3;
  }
  .block {
    fill: none;
    stroke: color-mix(in oklab, currentColor 30%, transparent);
    stroke-width: 0.6;
    stroke-dasharray: 2 2;
  }
  .column {
    fill: color-mix(in oklab, currentColor 12%, transparent);
    stroke: none;
  }
  .gap {
    fill: color-mix(in oklab, currentColor 4%, transparent);
  }
  .furniture {
    fill: color-mix(in oklab, currentColor 6%, transparent);
  }
  .marks line {
    stroke: color-mix(in oklab, currentColor 45%, transparent);
    stroke-width: 0.8;
  }
  .marks text {
    fill: currentColor;
    font-size: 7px;
    opacity: 0.7;
  }
  .marks text.below {
    text-anchor: middle;
  }
  .marks text.trim {
    text-anchor: end;
  }
  /* What the form is asking about, lit up on the page. */
  .lit {
    opacity: 1;
  }
  rect.lit {
    fill: color-mix(in oklab, currentColor 32%, transparent);
    stroke: currentColor;
    stroke-width: 1;
  }
  line.lit {
    stroke: currentColor;
    stroke-width: 1.6;
  }
  text.lit {
    font-weight: 700;
    opacity: 1;
  }
  /* Everything unlit recedes while something is highlighted, so the answer to
     "which one is that" is the only thing on the page that is dark. */
  figure.has-highlight .paper,
  figure.has-highlight .column:not(.lit),
  figure.has-highlight .furniture:not(.lit),
  figure.has-highlight .marks :not(.lit) {
    opacity: 0.4;
  }
  figcaption {
    margin-block-start: 0.4rem;
    font-size: 0.76rem;
    opacity: 0.6;
  }
</style>
