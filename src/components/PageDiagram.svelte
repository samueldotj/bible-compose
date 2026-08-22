<script lang="ts">
  /**
   * The page, drawn to scale, with every measurement editable *on* it.
   *
   * The page settings were nine numbers in a column. "Inner margin" and
   * "outer margin" are not self-explanatory to anyone who has not set a book
   * before — and worse, they are not checkable: a publisher reading `0.66in`
   * and `0.48in` cannot tell from a form which of the two is against the
   * spine, which is the whole point of having two.
   *
   * So the drawing *is* the form. A spread, because inner and outer only mean
   * anything across a fold; verso and recto with the fold between them, the
   * type area, the columns and their gutter, and bands where the running head
   * and folio sit. Each field sits on the thing it measures — the top margin
   * in the top margin, the gutter's width in the gutter — so there is nothing
   * to match up between a list of names and a picture. Change one and the
   * drawing changes with it.
   *
   * The dimensions come from the shell in points, already parsed. A second
   * unit parser here would be a second answer to what `13.97mm` is.
   */
  import type { Geometry, Setting } from "../lib/services/backend";
  import { session } from "../lib/session.svelte";
  import { TRIMS } from "../lib/trims";

  const { geometry: g }: { geometry: Geometry } = $props();

  /** Room outside the paper for the trim measurements. */
  const PAD = 30;
  const FOLD = 12;

  /** Points to viewBox units. The height is what the drawing is scaled by. */
  const scale = $derived(260 / g.pageHeight);
  const s = (points: number) => points * scale;

  const w = $derived(g.pageWidth * scale);
  const h = $derived(g.pageHeight * scale);
  const view = $derived({ width: w * 2 + FOLD + PAD * 2, height: h + PAD * 2 });

  const versoX = PAD;
  const rectoX = $derived(PAD + w + FOLD);
  const top = PAD;

  /**
   * The type area of one page. `inner` is the margin against the spine, which
   * is the right edge of the verso and the left edge of the recto — the
   * mirroring is the thing the drawing exists to show.
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

  function columns(area: { x: number; width: number }) {
    const n = Math.max(1, g.columns);
    const gap = s(g.columnGap);
    const each = (area.width - gap * (n - 1)) / n;
    return Array.from({ length: n }, (_, i) => ({ x: area.x + i * (each + gap), width: each }));
  }

  /** viewBox units as a percentage of the box, so a field can be placed on it. */
  const px = (x: number) => `${(x / view.width) * 100}%`;
  const py = (y: number) => `${(y / view.height) * 100}%`;

  const setting = (key: string): Setting | undefined =>
    session.settings.find((x) => x.key === key);

  function commit(key: string, value: string): void {
    const current = setting(key);
    if (!current || value.trim() === current.value) return;
    void session.setSetting(key, value);
  }

  /**
   * Whether the trim list is showing.
   *
   * A list of our own rather than a `<datalist>`, which was the obvious thing
   * and does not work here: the browser filters a datalist by what is already
   * in the field, so a trim box reading `6x9in` offers exactly one suggestion —
   * `6x9in` — and looks broken. The field is still free text; this is a way to
   * see what the usual answers are, which is the whole job.
   */
  let trims = $state(false);

  /** Every field, and where on the drawing it belongs. */
  const fields = $derived([
    {
      key: "page.size",
      label: "Trim",
      x: versoX + w / 2,
      y: top / 2,
      wide: true,
    },
    { key: "page.margin_top", label: "Top", x: recto.x + recto.width / 2, y: top + s(g.marginTop) / 2 },
    {
      key: "page.margin_bottom",
      label: "Bottom",
      x: recto.x + recto.width / 2,
      y: top + h - s(g.marginBottom) / 2,
    },
    { key: "page.margin_inner", label: "Inner", x: rectoX + s(g.marginInner) / 2, y: top + h * 0.62 },
    {
      key: "page.margin_outer",
      label: "Outer",
      x: rectoX + w - s(g.marginOuter) / 2,
      y: top + h * 0.62,
    },
    {
      key: "page.header_gap",
      label: "Head",
      x: verso.x + verso.width / 2,
      y: verso.y - s(g.headerGap) / 2,
    },
    {
      key: "page.footer_gap",
      label: "Foot",
      x: verso.x + verso.width / 2,
      y: verso.y + verso.height + s(g.footerGap) / 2,
    },
    { key: "page.columns", label: "Columns", x: verso.x + verso.width / 2, y: verso.y + verso.height * 0.3 },
    {
      key: "page.column_gap",
      label: "Gutter",
      // In the gutter when there is one; where it would be when there is not.
      x:
        g.columns > 1
          ? columns(verso)[0]!.x + columns(verso)[0]!.width + s(g.columnGap) / 2
          : verso.x + verso.width / 2,
      y: verso.y + verso.height * 0.62,
    },
  ]);
</script>

<figure class="sheet" style:aspect-ratio={`${view.width} / ${view.height}`}>
  <svg viewBox={`0 0 ${view.width} ${view.height}`} role="img" aria-label="The page, to scale">
    {#each [versoX, rectoX] as x (x)}
      <rect class="paper" {x} y={top} width={w} height={h} rx="1" />
    {/each}

    <!-- The fold. Drawn, not implied: it is what inner and outer refer to. -->
    <line
      class="spine"
      x1={PAD + w + FOLD / 2}
      y1={top - 5}
      x2={PAD + w + FOLD / 2}
      y2={top + h + 5}
    />

    {#each [verso, recto] as area (area.x)}
      <rect class="block" x={area.x} y={area.y} width={area.width} height={area.height} />
      {#each columns(area) as col, i (i)}
        <rect class="column" x={col.x} y={area.y} width={col.width} height={area.height} />
      {/each}
      <!-- Running head and folio: bands, because their gap is what is set. -->
      <rect
        class="furniture"
        x={area.x}
        y={area.y - s(g.headerGap)}
        width={area.width}
        height={s(g.headerGap)}
      />
      <rect
        class="furniture"
        x={area.x}
        y={area.y + area.height}
        width={area.width}
        height={s(g.footerGap)}
      />
    {/each}
  </svg>

  {#each fields as field (field.key)}
    {@const value = setting(field.key)}
    {#if value}
      {@const errors = session.fieldErrors[field.key] ?? []}
      <label
        class="field"
        class:wide={field.wide}
        class:set={value.overridden}
        class:open={field.key === "page.size" && trims}
        style:inset-inline-start={px(field.x)}
        style:inset-block-start={py(field.y)}
      >
        <span class="name">{field.label}</span>
        <!--
          The trim keeps its text field — `page.size` takes any two dimensions,
          and a publisher entering what their press quoted them is the ordinary
          case — with the usual answers a click away beside it.
        -->
        <span class="entry">
          <input
            type="text"
            value={value.value}
            spellcheck="false"
            class:bad={errors.length > 0}
            disabled={!session.editable}
            title={errors.map((e) => e.message).join(" ") || `${field.label} — ${value.key}`}
            onchange={(e) => commit(field.key, e.currentTarget.value)}
          />
          {#if field.key === "page.size"}
            <button
              type="button"
              class="chevron"
              aria-label="Common trim sizes"
              aria-expanded={trims}
              disabled={!session.editable}
              onclick={() => (trims = !trims)}
            >
              ▾
            </button>
          {/if}
        </span>

        {#if field.key === "page.size" && trims}
          <ul class="trims">
            {#each TRIMS as trim (trim.value)}
              <li>
                <button
                  type="button"
                  class:current={trim.value === value.value}
                  onclick={() => {
                    commit(field.key, trim.value);
                    trims = false;
                  }}
                >
                  <span class="size">{trim.value}</span>
                  <span class="what">{trim.name}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </label>
    {/if}
  {/each}
</figure>

{#each session.settings.filter((x) => x.key.startsWith("page.")) as s2 (s2.key)}
  {#each session.fieldErrors[s2.key] ?? [] as error (error.code + error.message)}
    <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
  {/each}
{/each}

<style>
  /* The drawing takes the pane. Its aspect ratio comes from the page being
     drawn, so a percentage position in here lands exactly where the same
     coordinate lands in the viewBox — which is what lets a field sit on the
     thing it measures. */
  .sheet {
    position: relative;
    inline-size: 100%;
    max-block-size: 100%;
    margin: 0;
  }
  svg {
    position: absolute;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
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
    fill: color-mix(in oklab, currentColor 10%, transparent);
  }
  .furniture {
    fill: color-mix(in oklab, currentColor 6%, transparent);
  }

  /* Each field is centred on its own measurement. */
  .field {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.1rem;
    translate: -50% -50%;
  }
  /*
    The field with a list open comes to the front — and it has to be raised
    *here*, on the field, not on the list inside it.

    `translate` makes each field its own stacking context, so a z-index on the
    list only orders it against its own siblings. Against the other fields the
    whole field still stacks by document order, and Trim is the first of nine:
    every measurement below it was painting over the open list.
  */
  .field.open {
    z-index: 5;
  }
  .name {
    padding-inline: 0.25rem;
    border-radius: 3px;
    background: color-mix(in oklab, Canvas 75%, transparent);
    font-size: 0.68rem;
    line-height: 1.2;
    opacity: 0.75;
    white-space: nowrap;
  }
  input {
    inline-size: 4.1rem;
    padding-block: 0.1rem;
    padding-inline: 0.25rem;
    border: 1px solid color-mix(in oklab, currentColor 30%, transparent);
    border-radius: 4px;
    background: color-mix(in oklab, Canvas 85%, transparent);
    color: inherit;
    font: inherit;
    font-size: 0.78rem;
    text-align: center;
  }
  .field.wide input {
    inline-size: 6rem;
  }
  .entry {
    display: flex;
    gap: 0.15rem;
    align-items: stretch;
  }
  .chevron {
    inline-size: 1.2rem;
    padding: 0;
    border: 1px solid color-mix(in oklab, currentColor 30%, transparent);
    border-radius: 4px;
    background: color-mix(in oklab, Canvas 85%, transparent);
    color: inherit;
    font: inherit;
    font-size: 0.7rem;
    line-height: 1;
    cursor: pointer;
  }
  /* Over the drawing rather than pushing it about, and above everything on it
     — a list that the paper showed through would be unreadable. */
  .trims {
    position: absolute;
    inset-block-start: calc(100% + 0.2rem);
    inset-inline-start: 50%;
    translate: -50% 0;
    inline-size: 15rem;
    max-block-size: 15rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    list-style: none;
    margin: 0;
    padding: 0.2rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 6px;
    background: Canvas;
    box-shadow: 0 6px 20px rgb(0 0 0 / 0.25);
  }
  .trims button {
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    inline-size: 100%;
    padding: 0.2rem 0.35rem;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.76rem;
    text-align: start;
    cursor: pointer;
  }
  .trims button:hover {
    background: color-mix(in oklab, currentColor 10%, transparent);
  }
  .trims button.current {
    font-weight: 700;
  }
  .size {
    flex: none;
    inline-size: 5.6rem;
    font-variant-numeric: tabular-nums;
  }
  .what {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.7;
  }
  .field.set input {
    border-color: color-mix(in oklab, currentColor 55%, transparent);
    font-weight: 600;
  }
  input:focus {
    background: Canvas;
  }
  input.bad {
    border-color: #c0392b;
  }
  input:disabled {
    opacity: 0.6;
  }
  .error {
    margin: 0.3rem 0 0;
    font-size: 0.8rem;
    color: #c0392b;
  }
</style>
