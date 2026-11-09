<script lang="ts">
  /**
   * The Cover section's centre: the whole sheet — back, spine, front — as it
   * will be printed, redrawn whenever a setting changes.
   *
   * The drawing is the PDF's own: Rust draws the cover once and writes it out
   * as SVG for here and as the page of `book_cover.pdf`, the lettering as the
   * font's outlines. So what is shown is what is made, in a font the window
   * itself never has to load.
   */
  import Segmented from "./ui/Segmented.svelte";
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { backend, type CoverPreview } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";

  let guides = $state(true);
  let shown = $state<CoverPreview | null>(null);
  let drawing = $state(false);
  let failed = $state<string | null>(null);

  /**
   * A newer request wins. Settings are written one field at a time, and an
   * answer for the field before last arriving after the last one would put
   * a cover on screen that the settings no longer describe.
   */
  let asked = 0;

  $effect(() => {
    // Every settings write replaces the project, so reading it here is what
    // redraws the cover when a field changes.
    const project = session.project;
    const withGuides = guides;
    if (!project) return;
    const mine = ++asked;
    drawing = true;
    backend()
      .coverPreview(project.root, withGuides)
      .then((p) => {
        if (mine !== asked) return;
        shown = p;
        failed = null;
      })
      .catch((e: unknown) => {
        if (mine !== asked) return;
        failed = e instanceof Error ? e.message : String(e);
      })
      .finally(() => {
        if (mine === asked) drawing = false;
      });
  });

  /**
   * The drawing as an image, from a blob of its own. An `<img>` and not the
   * markup itself: an SVG shown as an image cannot run anything or reach
   * the page, whatever is in it.
   */
  const src = $derived.by(() => {
    if (!shown?.svg) return null;
    return URL.createObjectURL(new Blob([shown.svg], { type: "image/svg+xml" }));
  });
  $effect(() => {
    const url = src;
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  });

  /** Width over height, which decides how large the sheet can be drawn. */
  const ratio = $derived(shown && shown.height_in > 0 ? shown.width_in / shown.height_in : 1.4);
  const fitted = $derived(`min(100cqi, calc(100cqb * ${ratio}))`);
  /** The printed width in CSS pixels, for the 100% view. */
  const actualWidth = $derived(shown ? shown.width_in * 96 : 1200);
</script>

<div class="canvas">
  <div class="toolbar">
    <span class="which">
      {#if shown?.svg}
        <strong>
          {phrases().coverMade(shown.binding, shown.width_in, shown.height_in, shown.spine_in, shown.pages, shown.pages_from === "estimate")}
        </strong>
      {:else if drawing}
        <span class="muted">{t("coverDrawing")}</span>
      {/if}
    </span>
    <span class="spacer"></span>
    <span data-search-key="action:cover-guides">
      <Segmented
        small
        options={[
          { value: "on", label: t("coverGuides") },
          { value: "off", label: t("coverNoGuides") },
        ]}
        value={guides ? "on" : "off"}
        onchange={(v) => (guides = v === "on")}
      />
    </span>
    <span data-search-key="action:fit">
      <Segmented
        small
        options={[
          { value: "fit", label: t("fit") },
          { value: "actual", label: t("actualSize") },
        ]}
        value={ui.fit ? "fit" : "actual"}
        onchange={(v) => (ui.fit = v === "fit")}
      />
    </span>
  </div>

  {#if shown}
    {#each shown.diagnostics as d (d.code + d.message)}
      <p class="note" class:error={d.severity === "error"}>{d.message}{d.help ? ` — ${d.help}` : ""}</p>
    {/each}
  {/if}
  {#if failed}
    <p class="note error">{failed}</p>
  {/if}

  <div class="room" class:scrolls={!ui.fit}>
    {#if shown && src}
      <img
        class="sheet"
        class:stale={drawing}
        {src}
        alt={t("coverPreviewLabel")}
        style:inline-size={ui.fit ? fitted : `${actualWidth}px`}
        style:aspect-ratio={`${shown.width_in} / ${shown.height_in}`}
      />
    {:else if !drawing}
      <p class="muted empty">{t("coverNothing")}</p>
    {/if}
  </div>
</div>

<style>
  .canvas {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-inline-size: 0;
    min-block-size: 0;
  }
  .toolbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    padding: 12px 24px 8px;
    color: var(--mut);
  }
  .which strong {
    color: var(--ink);
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .note {
    margin: 0 24px 6px;
    font-size: 12px;
    color: var(--warn, #9a6a00);
  }
  .note.error {
    color: var(--err, #b42318);
  }
  .room {
    flex: 1;
    min-block-size: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 24px 20px;
    container-type: size;
  }
  .room.scrolls {
    display: block;
    overflow: auto;
    overscroll-behavior: contain;
  }
  .sheet {
    display: block;
    block-size: auto;
    box-shadow: var(--shadow-page);
    transition: opacity 120ms linear;
  }
  .room.scrolls .sheet {
    margin: 0 auto;
    max-inline-size: none;
  }
  .sheet.stale {
    opacity: 0.85;
  }
  .empty {
    font-size: 13px;
  }
</style>
