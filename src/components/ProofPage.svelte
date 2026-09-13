<script lang="ts">
  /**
   * One page of the proof spread.
   *
   * A page of 1 John set to the project's trim and margins, in the project's
   * columns, with everything the settings switch on and off — so a publisher
   * sees a running head with a reference range in it rather than a checkbox
   * called "reference range in head". Clicking anything on it opens the
   * setting that governs it; pointing at a row of the inspector lights the
   * thing here it governs.
   *
   * The page is not a proof: it is not SILE, it does not use the project's
   * fonts, and it makes no claim about where a line will break. What it is
   * exact about is what these sections decide — which of these things are
   * on the page at all, and the shape of the sheet they are on.
   *
   * The passage is Berean Standard Bible, public domain, and the same
   * translation the Scripture fixtures use.
   */
  import SlotControl from "./ui/SlotControl.svelte";
  import { SAMPLE, SAMPLE_ALT_BOOK, SAMPLE_BOOK, SAMPLE_INTRO, SAMPLE_OUTLINE } from "../lib/sample";
  import { labelFor, wordsFor } from "../lib/labels";
  import { SWITCH_GROUPS } from "../lib/switches";
  import { LINES, POSITIONS, slotKey } from "../lib/heads";
  import { session } from "../lib/session.svelte";
  import { ui } from "../lib/ui.svelte";
  import { fromPoints } from "../lib/units";
  import { phrases, t } from "../lib/i18n";

  const {
    side,
    outline = null,
    guides = false,
    slots = false,
    onpick,
    onpickstyle,
  }: {
    /** Left-hand pages are the even-numbered ones. */
    side: "left" | "right";
    /** A style selector whose elements are outlined on the page. */
    outline?: string | null;
    /** Draw the margins as guides, with their measurements. */
    guides?: boolean;
    /** Put the six slot controls on the page's head and foot. */
    slots?: boolean;
    onpick?: (key: string) => void;
    onpickstyle?: (selector: string) => void;
  } = $props();

  const sideName = $derived(side === "left" ? t("leftPage") : t("rightPage"));

  /** A setting's value, defaulting to on so the page is never blank. */
  function on(key: string): boolean {
    return session.settings.find((s) => s.key === key)?.value !== "false";
  }
  const valueOf = (key: string) => session.settings.find((s) => s.key === key)?.value ?? "";

  const chapters = $derived(on("numbering.show_chapter_numbers"));
  const labels = $derived(on("numbering.show_chapter_labels"));
  const verses = $derived(on("numbering.show_verse_numbers"));
  const footnotes = $derived(on("notes.show_footnotes"));
  const refs = $derived(on("notes.show_cross_references"));
  const intros = $derived(on("contents.show_book_introductions"));
  const outlines = $derived(on("contents.show_introductory_outlines"));
  const headings = $derived(on("contents.show_section_headings"));
  const dropcaps = $derived(on("contents.drop_caps"));
  const numberDrops = $derived(dropcaps && valueOf("contents.drop_cap_of") === "chapter_number");
  const initialDrops = $derived(dropcaps && !numberDrops);
  const placement = (key: string) => valueOf(key) || "in_text";
  const chapterMargin = $derived(placement("numbering.chapter_number_placement"));
  const verseMargin = $derived(placement("numbering.verse_number_placement"));
  const marginLeft = $derived(chapterMargin === "left_margin" || verseMargin === "left_margin");
  const marginRight = $derived(chapterMargin === "right_margin" || verseMargin === "right_margin");
  const verseLines = $derived(valueOf("contents.verse_starts") === "next_line");
  const justified = $derived(on("typography.justify"));

  // ------------------------------------------------------------- geometry
  /** The page as numbers, in points, with the built-in trim before any arrive. */
  const g = $derived(
    session.geometry ?? {
      pageWidth: 504,
      pageHeight: 720,
      marginTop: 61,
      marginBottom: 79,
      marginInner: 61,
      marginOuter: 50,
      columnGap: 18,
      headerGap: 26,
      footerGap: 19,
      columns: 2,
    },
  );
  /** Percentages of the page's width, which is what padding is measured against. */
  const pw = (points: number) => `${(points / g.pageWidth) * 100}%`;
  const ph = (points: number) => `${(points / g.pageHeight) * 100}%`;
  const spineLeft = $derived(side === "right");
  const leftMargin = $derived(spineLeft ? g.marginInner : g.marginOuter);
  const rightMargin = $derived(spineLeft ? g.marginOuter : g.marginInner);
  const blockWidth = $derived(g.pageWidth - g.marginInner - g.marginOuter);
  const columns = $derived(Math.max(1, Math.min(3, g.columns)));
  const gap = $derived(`${(g.columnGap / blockWidth) * 100}%`);

  /**
   * The body type, at the page's own scale: `typography.font_size` as a
   * share of the page's height, so the page shows the measure the settings
   * give it — with a floor, since a page fitted to a small window would
   * otherwise set its text too small to read.
   */
  function points(value: string): number | null {
    const m = /^\s*([0-9.]+)\s*(pt|in|mm)?\s*$/.exec(value);
    if (!m) return null;
    const n = Number(m[1]);
    return m[2] === "in" ? n * 72 : m[2] === "mm" ? (n / 25.4) * 72 : n;
  }
  const bodyPt = $derived(points(valueOf("typography.font_size")) ?? 9.5);
  const leadPt = $derived(points(valueOf("typography.leading")) ?? bodyPt * 1.28);
  const fontSize = $derived(`clamp(8px, ${(bodyPt / g.pageHeight) * 100}cqb, 22px)`);
  const lineHeight = $derived(String(Math.max(1.05, leadPt / bodyPt)));

  // ------------------------------------------------------------- the text
  type Section = (typeof SAMPLE)[number]["sections"][number];
  type Verse = Section["verses"][number];

  const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
  function opening(section: Section): string {
    const text = section.verses[0]?.text.trimStart() ?? "";
    const first = segmenter.segment(text)[Symbol.iterator]().next();
    return first.done ? "" : first.value.segment;
  }
  function versesFor(section: Section, i: number): readonly (Verse & { opened?: boolean })[] {
    if (!(dropcaps && i === 0)) return section.verses;
    const [first, ...rest] = section.verses;
    if (!first) return section.verses;
    const text = initialDrops ? first.text.trimStart().slice(opening(section).length) : first.text;
    return [{ ...first, text, opened: true }, ...rest];
  }
  function around(text: string, after: string): [string, string] {
    const at = text.indexOf(after);
    if (at < 0) return [text, ""];
    const cut = at + after.length;
    return [text.slice(0, cut), text.slice(cut)];
  }

  const apparatus = $derived.by(() => {
    const out: { kind: "note" | "ref"; mark: string; text: string }[] = [];
    for (const chapter of SAMPLE) {
      for (const section of chapter.sections) {
        for (const verse of section.verses) {
          if (verse.reference && refs) out.push({ kind: "ref", mark: verse.reference.mark, text: verse.reference.note });
          if (verse.footnote && footnotes) out.push({ kind: "note", mark: verse.footnote.mark, text: verse.footnote.note });
        }
      }
    }
    return out;
  });

  // ------------------------------------------------------- head and foot
  $effect(() => {
    void session.loadHeadFields();
  });
  const fold = (name: string) => name.replace(/_/g, "").toLowerCase();
  const table = $derived(side === "left" ? "headers.left_page" : "headers.right_page");
  const pageNumber = $derived(side === "left" ? "412" : "413");

  /** What a template reads on this page, with each field's example. */
  function render(template: string): string {
    const fields = session.headFields ?? [];
    let out = "";
    let anyField = false;
    let anyValue = false;
    let i = 0;
    while (i < template.length) {
      const c = template[i];
      const next = template[i + 1];
      if ((c === "{" || c === "}") && next === c) {
        out += c;
        i += 2;
        continue;
      }
      if (c === "{") {
        const close = template.indexOf("}", i);
        if (close < 0) {
          out += template.slice(i);
          break;
        }
        const name = fold(template.slice(i + 1, close));
        const field = fields.find((f) => fold(f.name) === name);
        anyField = true;
        let value = field?.example ?? "";
        if (field?.name === "Page") value = pageNumber;
        if (field?.name === "Book") value = SAMPLE_BOOK;
        if (field?.name === "AltBook") value = SAMPLE_ALT_BOOK;
        if (value) {
          anyValue = true;
          out += value;
        }
        i = close + 1;
        continue;
      }
      out += c;
      i += 1;
    }
    return anyField && !anyValue ? "" : out;
  }
  const slot = (key: string) => render(valueOf(key));
  const header = $derived(POSITIONS.map((p) => ({ key: `${table}.header_${p}`, text: slot(`${table}.header_${p}`) })));
  const footer = $derived(POSITIONS.map((p) => ({ key: `${table}.footer_${p}`, text: slot(`${table}.footer_${p}`) })));

  // --------------------------------------------------------------- lights
  const shows = (key: string) => ui.lit === key;
  /** The label on the lit thing: what it is, and what it is set to. */
  function chip(key: string): string {
    const setting = session.settings.find((s) => s.key === key);
    if (!setting) return labelFor(key);
    // A switch with a choice folded into it reads as that choice.
    const folded = SWITCH_GROUPS.flatMap((g) => g.switches).find((s) => s.key === key)?.combined;
    if (folded) {
      const where = valueOf(folded.where);
      return `${labelFor(key)} · ${folded.labels[where] ?? wordsFor(where)}`;
    }
    const value =
      setting.kind === "boolean" ? "" : setting.kind === "choice" ? wordsFor(setting.value) : setting.value;
    return value ? `${labelFor(key)} · ${value}` : labelFor(key);
  }

  function clicked(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    // A control on the page is its own thing to click.
    if (target.closest("select, input, button")) return;
    const styled = target.closest<HTMLElement>("[data-style]");
    if (onpickstyle && styled?.dataset.style) {
      onpickstyle(styled.dataset.style);
      return;
    }
    const el = target.closest<HTMLElement>("[data-setting]");
    if (el?.dataset.setting && onpick) onpick(el.dataset.setting);
  }

  const outlined = (selector: string) => outline === selector;
  const slotName = (line: string, position: string) =>
    line === "header" ? phrases().headerSlot(sideName, position) : phrases().footerSlot(sideName, position);
</script>

<!-- The sheet is a size container of its own, so the type inside is a
     share of the page and not of the room the page stands in. -->
<div class="sheet" style:aspect-ratio={`${g.pageWidth} / ${g.pageHeight}`}>
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="paper"
  role="presentation"
  class:pickable={onpick !== undefined || onpickstyle !== undefined}
  style:padding={`${pw(g.marginTop)} ${pw(rightMargin)} ${pw(g.marginBottom)} ${pw(leftMargin)}`}
  style:font-size={fontSize}
  style:line-height={lineHeight}
  onclick={clicked}
>
  <!-- The margins, as things to point at. -->
  <span class="margin top" data-setting="page.margin_top" data-label={chip("page.margin_top")} class:lit={shows("page.margin_top")} style:block-size={ph(g.marginTop)}></span>
  <span class="margin bottom" data-setting="page.margin_bottom" data-label={chip("page.margin_bottom")} class:lit={shows("page.margin_bottom")} style:block-size={ph(g.marginBottom)}></span>
  <span
    class="margin side"
    data-setting={spineLeft ? "page.margin_inner" : "page.margin_outer"}
    data-label={chip(spineLeft ? "page.margin_inner" : "page.margin_outer")}
    class:lit={shows(spineLeft ? "page.margin_inner" : "page.margin_outer")}
    style:inset-inline-start="0"
    style:inline-size={pw(leftMargin)}
  ></span>
  <span
    class="margin side"
    data-setting={spineLeft ? "page.margin_outer" : "page.margin_inner"}
    data-label={chip(spineLeft ? "page.margin_outer" : "page.margin_inner")}
    class:lit={shows(spineLeft ? "page.margin_outer" : "page.margin_inner")}
    style:inset-inline-end="0"
    style:inline-size={pw(rightMargin)}
  ></span>

  <!-- The running head, in the top margin, the head gap above the text. -->
  <div
    class="line head"
    class:outlined={outlined("head")}
    data-style="head"
    style:inset-block-start={ph(g.marginTop - g.headerGap)}
    style:inset-inline-start={pw(leftMargin)}
    style:inset-inline-end={pw(rightMargin)}
  >
    {#each header as part (part.key)}
      <span class="slot" class:lit={shows(part.key)} data-setting={part.key} data-label={chip(part.key)}>{part.text}</span>
    {/each}
  </div>

  <div
    class="body"
    class:ragged={!justified}
    class:margin-left={marginLeft}
    class:margin-right={marginRight}
    style:column-count={columns}
    style:column-gap={gap}
  >
    {#if intros}
      <div class="front" class:lit={shows("contents.show_book_introductions")} data-setting="contents.show_book_introductions" data-label={chip("contents.show_book_introductions")}>
        <h3 class="heading">{SAMPLE_INTRO.heading}</h3>
        {#each SAMPLE_INTRO.paragraphs as para (para)}
          <p class="prose intro" class:outlined={outlined("paragraph.ip")} data-style="paragraph.ip">{para}</p>
        {/each}
      </div>
    {/if}

    {#if outlines}
      <div class="front" class:lit={shows("contents.show_introductory_outlines")} data-setting="contents.show_introductory_outlines" data-label={chip("contents.show_introductory_outlines")}>
        <h3 class="heading">{SAMPLE_OUTLINE.heading}</h3>
        <ul class="outline">
          {#each SAMPLE_OUTLINE.entries as entry (entry.text)}
            <li class:deep={entry.level > 1}>
              <span>{entry.text}</span>
              <span class="ref">{entry.reference}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#each SAMPLE as chapter (chapter.number)}
      {#each chapter.sections as section, i (section.heading)}
        {#if headings}
          <h3
            class="heading"
            class:lit={shows("contents.show_section_headings")}
            class:outlined={outlined("heading.s1")}
            data-setting="contents.show_section_headings"
            data-label={chip("contents.show_section_headings")}
            data-style="heading.s1"
          >
            {section.heading}
          </h3>
        {/if}
        {#if section.parallels}
          <p class="parallels" class:outlined={outlined("heading.r1")} data-style="heading.r1">({section.parallels})</p>
        {/if}

        <p class="prose" class:outlined={outlined("paragraph.p")} data-style="paragraph.p" data-setting="typography.justify" data-label={chip("typography.justify")}>
          {#if i === 0 && (chapters || (chapter.label && labels))}
            <span class="opening" class:own-line={initialDrops}>
              {#if chapters}
                <span
                  class="chapter"
                  class:in-margin={chapterMargin !== "in_text"}
                  class:at-right={chapterMargin === "right_margin"}
                  class:dropped={numberDrops && chapterMargin === "in_text"}
                  class:lit={shows("numbering.show_chapter_numbers") ||
                    shows("numbering.chapter_number_placement") ||
                    (numberDrops && shows("contents.drop_cap_of"))}
                  class:outlined={outlined("chapter")}
                  data-setting="numbering.show_chapter_numbers"
                  data-label={chip("numbering.show_chapter_numbers")}
                  data-style="chapter"
                >
                  {chapter.number}
                </span>
              {/if}
              {#if chapter.label && labels}
                <span class="label" class:lit={shows("numbering.show_chapter_labels")} data-setting="numbering.show_chapter_labels" data-label={chip("numbering.show_chapter_labels")}>
                  {chapter.label}
                </span>
              {/if}
            </span>
          {/if}
          {#if initialDrops && i === 0 && opening(section)}<span
              class="initial"
              class:lit={shows("contents.drop_caps") || shows("contents.drop_cap_of") || shows("contents.drop_cap_lines")}
              data-setting="contents.drop_caps"
              data-label={chip("contents.drop_caps")}>{opening(section)}</span
            >{/if}{#each versesFor(section, i) as verse, j (verse.number)}
            {#if verseLines && j > 0}<br />{/if}{#if verses && !verse.opened}<span
                class="verse"
                class:in-margin={verseMargin !== "in_text"}
                class:at-right={verseMargin === "right_margin"}
                class:lit={shows("numbering.show_verse_numbers") ||
                  shows("numbering.verse_number_placement") ||
                  (j === 0 && shows("numbering.hide_first_verse_number"))}
                class:outlined={outlined("verse")}
                data-setting="numbering.show_verse_numbers"
                data-label={chip("numbering.show_verse_numbers")}
                data-style="verse">{verse.number}</span
              >{/if}{#if verse.reference}{@const parts = around(verse.text, verse.reference.after)}
              {parts[0]}{#if refs}<sup
                  class="ref"
                  class:lit={shows("notes.show_cross_references")}
                  class:outlined={outlined("reference")}
                  data-setting="notes.show_cross_references"
                  data-label={chip("notes.show_cross_references")}
                  data-style="reference">{verse.reference.mark}</sup
                >{/if}{parts[1]}
            {:else if verse.footnote}
              {@const parts = around(verse.text, verse.footnote.after)}
              {parts[0]}{#if footnotes}<sup
                  class="note"
                  class:lit={shows("notes.show_footnotes")}
                  class:outlined={outlined("note.f")}
                  data-setting="notes.show_footnotes"
                  data-label={chip("notes.show_footnotes")}
                  data-style="note.f">{verse.footnote.mark}</sup
                >{/if}{parts[1]}
            {:else}
              {verse.text}
            {/if}
          {/each}
        </p>
      {/each}
    {/each}
  </div>

  {#if apparatus.length > 0}
    <div
      class="apparatus"
      class:lit={shows("notes.cross_reference_placement") || shows("notes.restart_numbering")}
      data-setting="notes.show_footnotes"
      data-label={chip("notes.show_footnotes")}
    >
      {#each apparatus as entry (entry.kind + entry.mark + entry.text)}
        <p class:outlined={outlined(entry.kind === "note" ? "note.f" : "reference")} data-style={entry.kind === "note" ? "note.f" : "reference"}>
          <sup>{entry.mark}</sup>
          {entry.text}
        </p>
      {/each}
    </div>
  {/if}

  <!-- The foot, in the bottom margin, the foot gap below the text. -->
  <div
    class="line foot"
    class:outlined={outlined("folio")}
    data-style="folio"
    style:inset-block-end={ph(g.marginBottom - g.footerGap)}
    style:inset-inline-start={pw(leftMargin)}
    style:inset-inline-end={pw(rightMargin)}
  >
    {#each footer as part (part.key)}
      <span class="slot" class:lit={shows(part.key)} data-setting={part.key} data-label={chip(part.key)}>{part.text}</span>
    {/each}
  </div>

  {#if guides}
    <!-- The text block, the head and foot lines, and the columns, drawn
         over the page in the accent so a margin is a thing to see. -->
    <div class="guide block" style:inset={`${ph(g.marginTop)} ${pw(rightMargin)} ${ph(g.marginBottom)} ${pw(leftMargin)}`}></div>
    <div class="guide rule" style:inset-block-start={ph(g.marginTop - g.headerGap)} style:inset-inline-start={pw(leftMargin)} style:inset-inline-end={pw(rightMargin)}></div>
    <div class="guide rule" style:inset-block-end={ph(g.marginBottom - g.footerGap)} style:inset-inline-start={pw(leftMargin)} style:inset-inline-end={pw(rightMargin)}></div>
    {#if side === "right"}
      <button type="button" class="tag" style:inset-block-start={ph(g.marginTop / 2)} style:inset-inline-start="50%" onclick={() => onpick?.("page.margin_top")}>{t("top")} {fromPoints(g.marginTop, ui.unit)}</button>
      <button type="button" class="tag" style:inset-block-end={ph(g.marginBottom / 2)} style:inset-inline-start="50%" onclick={() => onpick?.("page.margin_bottom")}>{t("bottom")} {fromPoints(g.marginBottom, ui.unit)}</button>
      <button type="button" class="tag side-tag" style:inset-block-start="50%" style:inset-inline-start={pw(g.marginInner / 2)} onclick={() => onpick?.("page.margin_inner")}>{t("inner")} {fromPoints(g.marginInner, ui.unit)}</button>
      <button type="button" class="tag side-tag" style:inset-block-start="50%" style:inset-inline-end={pw(g.marginOuter / 2)} style:translate="50% -50%" onclick={() => onpick?.("page.margin_outer")}>{t("outer")} {fromPoints(g.marginOuter, ui.unit)}</button>
    {:else}
      <button type="button" class="tag side-tag" style:inset-block-start="50%" style:inset-inline-start={pw(g.marginOuter / 2)} onclick={() => onpick?.("page.margin_outer")}>{t("outer")} {fromPoints(g.marginOuter, ui.unit)}</button>
      <button type="button" class="tag" style:inset-block-start={ph(g.marginTop - g.headerGap / 2)} style:inset-inline-start="50%" onclick={() => onpick?.("page.header_gap")}>{t("headRow")} {fromPoints(g.headerGap, ui.unit)}</button>
      <button type="button" class="tag" style:inset-block-end={ph(g.marginBottom - g.footerGap / 2)} style:inset-inline-start="50%" onclick={() => onpick?.("page.footer_gap")}>{t("footRow")} {fromPoints(g.footerGap, ui.unit)}</button>
    {/if}
  {/if}

  {#if slots}
    <!-- The six controls, each over the slot it fills. -->
    {#each LINES as line (line)}
      <div class="slots {line}" style:inset-inline-start={pw(leftMargin)} style:inset-inline-end={pw(rightMargin)}>
        {#each POSITIONS as position (position)}
          <SlotControl key={slotKey(side, line, position)} name={slotName(line, position)} compact />
        {/each}
      </div>
    {/each}
  {/if}
</div>
</div>

<style>
  .sheet {
    block-size: 100%;
    container-type: size;
  }
  /* The page. Paper whatever the window's theme: a page is a page. Its
     type is sized to its own height, so a page that fits a small window is
     the same page smaller. */
  .paper {
    position: relative;
    display: flex;
    flex-direction: column;
    inline-size: 100%;
    block-size: 100%;
    overflow: hidden;
    background: var(--paper);
    color: var(--paper-ink);
    font-family: var(--serif);
    font-variation-settings: "opsz" 10;
  }
  .paper.pickable [data-setting],
  .paper.pickable [data-style] {
    cursor: pointer;
  }

  .margin {
    position: absolute;
    inset-inline: 0;
  }
  .margin.top {
    inset-block-start: 0;
  }
  .margin.bottom {
    inset-block-end: 0;
  }
  .margin.side {
    inset-block: 0;
    inset-inline: auto;
  }

  /* Three slots on one line: the outer two at the margins, the middle one
     between them. The same arrangement top and bottom, because a head and a
     foot are the same three questions. */
  .line {
    position: absolute;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: baseline;
    gap: 0.5em;
    font-size: 0.92em;
    font-variant: small-caps;
    letter-spacing: 0.06em;
  }
  .line.head {
    translate: 0 -100%;
    padding-block-end: 0.28em;
    border-block-end: 1px solid var(--paper-ink);
  }
  .line.foot {
    translate: 0 100%;
    text-align: center;
  }
  .line .slot:first-child {
    text-align: start;
  }
  .line .slot:last-child {
    text-align: end;
  }
  .slot:empty::after {
    content: "\00a0";
  }

  .body {
    flex: 1;
    min-block-size: 0;
    overflow: hidden;
    column-fill: auto;
  }
  .heading {
    margin: 0.85em 0 0.12em;
    font-size: 1.12em;
    font-weight: 700;
    break-after: avoid;
  }
  .heading:first-child {
    margin-block-start: 0;
  }
  .parallels {
    margin: 0 0 0.3em;
    font-size: 0.92em;
    font-style: italic;
    opacity: 0.8;
  }
  .prose {
    position: relative;
    margin: 0 0 0.55em;
    text-align: justify;
    hyphens: auto;
  }
  .body.ragged .prose {
    text-align: start;
  }
  .front {
    margin-block-end: 0.7em;
  }
  .outline {
    list-style: none;
    margin: 0.18em 0 0;
    padding: 0;
    font-size: 0.95em;
  }
  .outline li {
    display: flex;
    justify-content: space-between;
    gap: 0.6em;
  }
  .outline li.deep {
    padding-inline-start: 1.2em;
  }
  .outline .ref {
    opacity: 0.7;
    font-variant-numeric: tabular-nums;
  }
  .label {
    margin-inline-end: 0.35em;
  }
  .chapter {
    float: inline-start;
    margin-inline-end: 0.3em;
    font-size: 2.5em;
    font-weight: 700;
    line-height: 0.9;
  }
  .verse {
    margin-inline-end: 0.15em;
    font-size: 0.68em;
    font-weight: 700;
    vertical-align: super;
  }
  /* Numbers in the margin: the body makes room at the side, and each number
     is taken out of the line and set at that side, level with the line it
     would have started. */
  .body.margin-left {
    padding-inline-start: 2.2em;
  }
  .body.margin-right {
    padding-inline-end: 2.2em;
  }
  .in-margin {
    position: absolute;
    inset-inline-start: -2em;
    inline-size: 1.6em;
    margin: 0;
    text-align: end;
    vertical-align: baseline;
    float: none;
  }
  .in-margin.at-right {
    inset-inline-start: auto;
    inset-inline-end: -2em;
    text-align: start;
  }
  .chapter.in-margin {
    font-size: 1.6em;
    line-height: 1;
    inline-size: 1.15em;
  }
  sup.note,
  sup.ref {
    font-size: 0.68em;
    font-weight: 700;
    line-height: 0;
  }
  sup.ref {
    font-style: italic;
  }
  .opening.own-line {
    display: block;
    line-height: 1.1;
  }
  .opening.own-line .chapter {
    float: none;
    margin-inline-end: 0.18em;
  }
  .initial,
  .chapter.dropped {
    float: inline-start;
    font-size: 3.1em;
    line-height: 0.82;
    font-weight: 600;
    padding-inline-end: 0.06em;
    margin-block-start: 0.04em;
  }
  .chapter.dropped {
    font-weight: 700;
    margin-inline-end: 0;
  }

  .apparatus {
    flex: none;
    margin-block-start: 0.8em;
    padding-block-start: 0.4em;
    border-block-start: 1px solid var(--paper-ink);
    font-size: 0.86em;
    line-height: 1.5;
    max-block-size: 30%;
    overflow: hidden;
  }
  .apparatus p {
    margin: 0;
  }

  /* What the row under the pointer governs, and its name. */
  .lit {
    position: relative;
    border-radius: 3px;
    outline: 1.5px solid var(--acc);
    outline-offset: 2px;
    background: color-mix(in oklab, var(--acc) 10%, transparent);
  }
  .margin.lit {
    outline-offset: -1.5px;
  }
  .lit[data-label]::after {
    content: attr(data-label);
    position: absolute;
    inset-block-start: calc(100% + 0.6em);
    inset-inline-start: 0;
    z-index: 3;
    padding: 0.25em 0.7em;
    border-radius: 5px;
    background: var(--acc);
    color: var(--on-acc);
    font: 600 11px var(--sans);
    font-variant: normal;
    letter-spacing: normal;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    pointer-events: none;
  }
  .margin.top.lit::after,
  .line.head .lit::after {
    inset-block-start: calc(100% + 0.2em);
    inset-inline-start: 50%;
    translate: -50% 0;
  }
  .margin.bottom.lit::after,
  .line.foot .lit::after {
    inset-block-start: auto;
    inset-block-end: calc(100% + 0.2em);
    inset-inline-start: 50%;
    translate: -50% 0;
  }
  .margin.side.lit::after {
    inset-block-start: 40%;
    inset-inline-start: 0.5em;
  }

  /* The chosen style, outlined. */
  .outlined {
    outline: 1.5px solid var(--acc);
    outline-offset: 2px;
    border-radius: 3px;
    background: color-mix(in oklab, var(--acc) 8%, transparent);
  }

  /* Guides. */
  .guide {
    position: absolute;
    pointer-events: none;
  }
  .guide.block {
    border: 1px dashed var(--acc);
  }
  .guide.rule {
    block-size: 0;
    border-block-start: 1px dashed var(--acc);
    opacity: 0.6;
  }
  .tag {
    position: absolute;
    z-index: 2;
    translate: -50% -50%;
    padding: 3px 8px;
    border: 0;
    border-radius: 5px;
    background: var(--acc);
    color: var(--on-acc);
    font: 600 11px var(--sans);
    white-space: nowrap;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }
  .tag.side-tag {
    translate: -50% -50%;
  }

  /* The slot controls over the head and the foot. */
  .slots {
    position: absolute;
    z-index: 2;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 6px;
    font: 11px var(--sans);
    font-variant: normal;
    letter-spacing: normal;
  }
  .slots.header {
    inset-block-start: 10px;
  }
  .slots.footer {
    inset-block-end: 8px;
  }
</style>
