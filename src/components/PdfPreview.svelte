<script lang="ts">
  /**
   * GUI-008: the pages of the PDF just built, inside the window.
   *
   * The shell draws each page (ADR-003, revision 3) and this shows the
   * pictures. Everything is laid out from the page sizes before anything is
   * drawn, so the scroll bar is the length of the book from the start; a page
   * is only asked for when it comes near the screen, which is what lets a
   * Bible of two thousand pages open as quickly as a tract.
   *
   * What it is not: a PDF reader. No text to select, no search, no printing —
   * Open PDF is for those, and the reader it opens is the one the printer's
   * file will be checked in anyway.
   */
  import { onDestroy, untrack } from "svelte";
  import Segmented from "./ui/Segmented.svelte";
  import { asDiagnostics, backend, type PreviewDocument } from "../lib/services/backend";
  import { phrases, t } from "../lib/i18n";
  import { session } from "../lib/session.svelte";

  const { path }: { path: string } = $props();

  let doc = $state<PreviewDocument | null>(null);
  let failed = $state<string | null>(null);
  // Started from where the reader last was: this viewer is taken down while a
  // build runs and put back when it ends.
  const place = untrack(() => session.previewPlace);
  let layout = $state<"page" | "spread">(place.layout);
  /** Relative to fitting the width of the pane: 1 fits, 2 is twice that. */
  let zoom = $state(place.zoom);
  const ZOOMS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3] as const;
  const MIN_ZOOM = 0.5;
  const MAX_ZOOM = 3;

  let scroller = $state<HTMLDivElement | null>(null);
  let paneWidth = $state(0);
  /** Pages near enough the screen to be drawn. */
  let near = $state<ReadonlySet<number>>(new Set());
  /** The page at the top of the pane, which the toolbar reports. */
  let current = $state(place.page);
  let typed = $state(String(place.page));
  $effect(() => {
    session.previewPlace = { layout, zoom, page: current };
  });

  // A newer request supersedes an older one, so a rebuild that lands while
  // the previous PDF is still being read cannot be overwritten by it.
  let asked = 0;
  $effect(() => {
    const mine = ++asked;
    const file = path;
    failed = null;
    backend()
      .openPreview(file)
      .then((opened) => {
        if (mine !== asked) return;
        doc = opened;
        near = new Set();
        if (current > opened.pages.length) current = 1;
      })
      .catch((e: unknown) => {
        if (mine !== asked) return;
        doc = null;
        failed = asDiagnostics(e)
          .map((d) => d.message)
          .join(" ");
      });
  });

  onDestroy(() => {
    asked++;
    void backend().closePreview();
  });

  /** Rows of page numbers. A spread opens on a right-hand page, as a book does. */
  const rows = $derived.by((): (number | null)[][] => {
    const count = doc?.pages.length ?? 0;
    const out: (number | null)[][] = [];
    if (layout === "page") {
      for (let p = 1; p <= count; p++) out.push([p]);
      return out;
    }
    if (count > 0) out.push([null, 1]);
    for (let p = 2; p <= count; p += 2) out.push(p + 1 <= count ? [p, p + 1] : [p, null]);
    return out;
  });

  const GAP = 16;
  const PAD = 24;

  /** How wide one page is drawn, in CSS pixels. Every page shares the width. */
  const pageWidth = $derived.by(() => {
    const across = layout === "spread" ? 2 : 1;
    const room = Math.max(paneWidth - PAD * 2 - GAP * (across - 1), 120);
    // At fit, a single page is not stretched past a comfortable reading width.
    const fit = layout === "page" ? Math.min(room, 900) : room / across;
    return Math.floor(fit * zoom);
  });

  function heightOf(page: number): number {
    const size = doc?.pages[page - 1];
    if (!size) return pageWidth * 1.5;
    return Math.round((pageWidth * size[1]) / size[0]);
  }

  /**
   * The address to draw one page at. The width asked for is rounded up to a
   * step, so a pane resized by a few pixels reuses the pages already drawn
   * instead of drawing them all again.
   */
  function src(page: number): string | null {
    if (!doc) return null;
    const device = pageWidth * (window.devicePixelRatio || 1);
    const stepped = Math.ceil(device / 128) * 128;
    return backend().previewPage(doc.generation, page, stepped);
  }

  // The pane's width inside its scroll bar. Measured rather than bound:
  // `bind:clientWidth` reports the width with the scroll bar in it, and a spread
  // fitted to that is a scroll bar's width too wide.
  $effect(() => {
    const pane = scroller;
    if (!pane) return;
    const measure = new ResizeObserver(() => (paneWidth = pane.clientWidth));
    measure.observe(pane);
    paneWidth = pane.clientWidth;
    return () => measure.disconnect();
  });

  /** One observer for all the pages, made when the pane exists. */
  let observer: IntersectionObserver | null = null;
  const watched = new Map<Element, number>();
  $effect(() => {
    if (!scroller) return;
    const ob = new IntersectionObserver(
      (entries) => {
        const next = new Set(near);
        for (const entry of entries) {
          const page = watched.get(entry.target);
          if (page === undefined) continue;
          if (entry.isIntersecting) next.add(page);
          else next.delete(page);
        }
        near = next;
        reportCurrent();
      },
      // A screen above and below, so a page is drawn before it is reached.
      { root: scroller, rootMargin: "100% 0px" },
    );
    for (const el of watched.keys()) ob.observe(el);
    observer = ob;
    return () => {
      ob.disconnect();
      observer = null;
    };
  });

  function observe(node: HTMLElement, page: number) {
    watched.set(node, page);
    observer?.observe(node);
    return {
      update(next: number) {
        watched.set(node, next);
      },
      destroy() {
        watched.delete(node);
        observer?.unobserve(node);
      },
    };
  }

  /** The first page whose bottom is below the top of the pane. */
  function reportCurrent(): void {
    if (!scroller || restoring !== null) return;
    const top = scroller.getBoundingClientRect().top;
    let best: number | null = null;
    for (const [el, page] of watched) {
      const box = el.getBoundingClientRect();
      if (box.bottom > top + 8 && (best === null || page < best)) best = page;
    }
    if (best !== null && best !== current) {
      current = best;
      typed = String(best);
    }
  }

  function go(page: number): void {
    if (!doc) return;
    const target = Math.min(Math.max(Math.round(page), 1), doc.pages.length);
    for (const [el, p] of watched) {
      if (p === target) {
        el.scrollIntoView({ block: "start" });
        current = target;
        typed = String(target);
        return;
      }
    }
  }

  function step(direction: 1 | -1): void {
    const at = ZOOMS.findIndex((z) => z >= zoom - 1e-6);
    const next = Math.min(Math.max((at === -1 ? ZOOMS.length - 1 : at) + direction, 0), ZOOMS.length - 1);
    zoom = ZOOMS[next] ?? 1;
  }

  function keys(event: KeyboardEvent): void {
    if (event.target instanceof HTMLInputElement) return;
    if ((event.ctrlKey || event.metaKey) && (event.key === "=" || event.key === "+")) {
      event.preventDefault();
      step(1);
    } else if ((event.ctrlKey || event.metaKey) && event.key === "-") {
      event.preventDefault();
      step(-1);
    } else if ((event.ctrlKey || event.metaKey) && event.key === "0") {
      event.preventDefault();
      zoom = 1;
    } else if (event.key === "Home") {
      event.preventDefault();
      go(1);
    } else if (event.key === "End" && doc) {
      event.preventDefault();
      go(doc.pages.length);
    }
  }

  // Zooming, switching layout, resizing the pane or rebuilding keeps the page
  // that was being read in view, rather than leaving the scroll offset where it
  // was and the reader somewhere else in the book.
  //
  // Until the first document is laid out, the page to return to is held
  // aside: the pages arrive at the top of the pane, and reading the position
  // then would report page 1 and forget where the reader was.
  let restoring: number | null = place.page;
  $effect(() => {
    void pageWidth;
    const count = rows.length;
    const page = restoring ?? untrack(() => current);
    requestAnimationFrame(() => {
      go(page);
      if (count > 0) restoring = null;
    });
  });
</script>

<div class="preview">
  <div class="toolbar">
    <Segmented
      small
      options={[
        { value: "page", label: t("pageView") },
        { value: "spread", label: t("spreadView") },
      ]}
      value={layout}
      onchange={(v) => (layout = v === "spread" ? "spread" : "page")}
    />
    <span class="zoom">
      <button type="button" class="icon" title={t("zoomOut")} aria-label={t("zoomOut")} disabled={zoom <= MIN_ZOOM} onclick={() => step(-1)}>−</button>
      <button type="button" class="fit" class:active={zoom === 1} onclick={() => (zoom = 1)}>{t("fit")}</button>
      <button type="button" class="icon" title={t("zoomIn")} aria-label={t("zoomIn")} disabled={zoom >= MAX_ZOOM} onclick={() => step(1)}>+</button>
      <span class="mono muted pct">{Math.round(zoom * 100)}%</span>
    </span>
    <span class="spacer"></span>
    {#if doc}
      <label class="goto">
        <input
          type="text"
          inputmode="numeric"
          class="mono"
          aria-label={t("goToPage")}
          bind:value={typed}
          onkeydown={(e) => {
            if (e.key === "Enter") go(Number.parseInt(typed, 10) || current);
          }}
          onblur={() => (typed = String(current))}
        />
        <span class="muted">{phrases().ofPages(doc.pages.length)}</span>
      </label>
    {/if}
  </div>

  <!-- A scrolling region takes focus so the keyboard can scroll it (NFR-011),
       and the zoom and Home/End keys belong to it. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="pages"
    bind:this={scroller}
    tabindex="0"
    role="region"
    aria-label={t("previewView")}
    onkeydown={keys}
    onscroll={reportCurrent}
  >
    {#if failed}
      <p class="note error">{failed}</p>
    {:else if !doc}
      <p class="note muted">{t("previewOpening")}</p>
    {:else}
      {#each rows as row, r (layout + r)}
        <div class="row" style:gap="{GAP}px">
          {#each row as page, i (i)}
            {#if page === null}
              <div class="blank" style:inline-size="{pageWidth}px"></div>
            {:else}
              <div
                class="page"
                use:observe={page}
                style:inline-size="{pageWidth}px"
                style:block-size="{heightOf(page)}px"
              >
                {#if near.has(page)}
                  <img src={src(page)} alt={phrases().previewPage(page, doc.pages.length)} draggable="false" />
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-block-size: 0;
    min-inline-size: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding-block-end: 12px;
  }
  .zoom {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .zoom button {
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: inherit;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .zoom .icon {
    inline-size: 26px;
    block-size: 24px;
    padding: 0;
  }
  .zoom .fit {
    padding: 3px 10px;
  }
  .zoom .fit.active {
    border-color: var(--acc);
    color: var(--acc);
  }
  .zoom button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .pct {
    min-inline-size: 3.5em;
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
  .goto {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
  }
  .goto input {
    inline-size: 4em;
    padding: 3px 6px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
    color: inherit;
    font-size: 12.5px;
    text-align: end;
  }
  .pages {
    flex: 1;
    min-block-size: 0;
    overflow: auto;
    overscroll-behavior: contain;
    /* The width pages are fitted to is measured once the scroll bar's room is
       already taken; otherwise the bar arrives after the fit and pushes the
       pages under a horizontal one. */
    scrollbar-gutter: stable;
    padding: 24px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: color-mix(in oklab, var(--line) 45%, var(--bg, transparent));
  }
  .pages:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  .row {
    display: flex;
    justify-content: center;
    margin-block-end: 16px;
    inline-size: max-content;
    min-inline-size: 100%;
  }
  .page {
    flex: none;
    background: #fff;
    box-shadow:
      0 0 0 1px rgb(0 0 0 / 0.08),
      0 2px 8px rgb(0 0 0 / 0.12);
  }
  .blank {
    flex: none;
  }
  .page img {
    display: block;
    inline-size: 100%;
    block-size: 100%;
    user-select: none;
  }
  .note {
    margin: 0;
    padding: 16px;
  }
  .error {
    color: var(--err, #b42318);
  }
</style>
