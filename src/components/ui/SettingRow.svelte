<script lang="ts">
  /**
   * One setting, as a row of the inspector.
   *
   * The label on the left with a hint under it where one helps, the control
   * on the right — a switch for a yes-or-no, a dropdown for a choice, a
   * short field for a length or a count — and under both, when the project
   * has set it, where it was set and a way to unset it. Which control this is
   * is the schema's answer and not a field in a list, so a setting that
   * becomes a choice gets a dropdown without anyone remembering to say so.
   *
   * Pointing at the row lights the thing it governs on the page, and
   * pointing at that thing lights this row: `ui.lit` is the one place both
   * halves of the window read.
   */
  import FontPicker from "../FontPicker.svelte";
  import Toggle from "./Toggle.svelte";
  import { LANGUAGES } from "../../lib/languages";
  import { labelFor, placeholderFor, wordsFor } from "../../lib/labels";
  import { session } from "../../lib/session.svelte";
  import { ui } from "../../lib/ui.svelte";
  import type { Setting } from "../../lib/services/backend";
  import { phrases, t } from "../../lib/i18n";

  const {
    key,
    label,
    hint,
    combined,
    under,
    implied,
    range,
  }: {
    key: string;
    label?: string;
    hint?: string;
    /** One dropdown over this switch and the choice it enables. */
    combined?: { where: string; off: string; labels: Readonly<Record<string, string>> };
    /** Idle unless this setting is on. */
    under?: string;
    /** Decided, and shown on, while this setting is on. */
    implied?: string;
    /** For a number: the range the resolver accepts. */
    range?: readonly [number, number];
  } = $props();

  const setting = $derived(session.settings.find((s) => s.key === key));
  const errors = $derived(session.fieldErrors[key] ?? []);

  function on(k: string): boolean {
    return session.settings.find((s) => s.key === k)?.value !== "false";
  }
  const isImplied = $derived(implied !== undefined && on(implied));
  const idle = $derived((under !== undefined && !on(under)) || isImplied);
  const disabled = $derived(!session.editable || idle);
  const lit = $derived(ui.lit === key || (combined !== undefined && ui.lit === combined.where));

  function commit(value: string): void {
    if (!setting || value === setting.value) return;
    void session.setSetting(key, value);
  }

  async function toggle(next: boolean): Promise<void> {
    await session.setSetting(key, next ? "true" : "false");
    // Drop caps decide the first verse's number: the initial is its marker,
    // so the number goes. Written into the project rather than only implied
    // by the backend, so the setting says what the page does.
    if (key === "contents.drop_caps" && next) {
      await session.setSetting("numbering.hide_first_verse_number", "true");
    }
  }

  /** A combined row's current entry: the choice's value, or its off entry. */
  const combinedValue = $derived.by(() => {
    if (!combined) return "";
    if (!on(key)) return "__off";
    return session.settings.find((s) => s.key === combined.where)?.value ?? "";
  });
  const combinedChoices = $derived(
    combined ? (session.settings.find((s) => s.key === combined.where)?.choices ?? []) : [],
  );

  async function chooseCombined(entry: string): Promise<void> {
    if (!combined) return;
    if (entry === "__off") {
      await session.setSetting(key, "false");
      return;
    }
    if (!on(key)) await session.setSetting(key, "true");
    await session.setSetting(combined.where, entry);
  }

  /** Where the value came from, for the line under an overridden row. */
  const origin = $derived.by(() => {
    if (!setting?.overridden) return null;
    const at = setting.location;
    if (!at) return t("setInProject");
    const file = at.path.split(/[/\\]/).pop() ?? at.path;
    return at.line ? `${file}:${at.line}` : file;
  });

  /** Also overridden when the folded-in choice is. */
  const anyOverridden = $derived(
    setting?.overridden ||
      (combined !== undefined &&
        (session.settings.find((s) => s.key === combined.where)?.overridden ?? false)),
  );

  async function reset(): Promise<void> {
    if (setting?.overridden) await session.resetSetting(key);
    if (combined) {
      const where = session.settings.find((s) => s.key === combined.where);
      if (where?.overridden) await session.resetSetting(combined.where);
    }
  }

  let picking = $state(false);

  /**
   * The languages to offer, with whatever the project already says: a tag
   * this list has never heard of is an ordinary thing for a Bible to be.
   */
  function languages(current: string): { tag: string; label: string }[] {
    const listed = LANGUAGES.map((l) => ({ tag: l.tag, label: phrases().languageWithTag(l.name, l.tag) }));
    const tag = current.trim();
    if (tag !== "" && !LANGUAGES.some((l) => l.tag === tag)) {
      listed.unshift({ tag, label: phrases().setInThisProject(tag) });
    }
    return listed;
  }

  /** A text field wide enough for words, or short enough for a length. */
  function wide(s: Setting): boolean {
    return s.kind === "text" || s.kind === "path" || s.kind === "list" || s.kind === "page_size";
  }

  const id = $derived(`row-${key.replace(/\W/g, "-")}`);
</script>

<div
  class="row"
  class:lit
  class:idle
  role="group"
  data-search-key={key}
  onpointerenter={() => (ui.lit = key)}
  onpointerleave={() => (ui.lit = null)}
  onfocusin={() => (ui.lit = key)}
  onfocusout={() => (ui.lit = null)}
>
  <div class="main">
    <label class="label" for={id}>
      <span class="name">{label ?? labelFor(key)}</span>
      {#if hint}<span class="hint">{hint}</span>{/if}
    </label>

    {#if setting === undefined}
      <span class="hint">{t("notInThisBuild")}</span>
    {:else if combined}
      <span class="select-wrap">
        <select
          {id}
          class="input"
          value={combinedValue}
          {disabled}
          onchange={(e) => void chooseCombined(e.currentTarget.value)}
        >
          <option value="__off">{combined.off}</option>
          {#each combinedChoices as choice (choice)}
            <option value={choice}>{combined.labels[choice] ?? wordsFor(choice)}</option>
          {/each}
        </select>
      </span>
    {:else if setting.kind === "boolean"}
      <Toggle
        checked={setting.value === "true" || isImplied}
        {disabled}
        label={label ?? labelFor(key)}
        onchange={(next) => void toggle(next)}
      />
    {:else if setting.kind === "choice"}
      <span class="select-wrap">
        <select {id} class="input" value={setting.value} {disabled} onchange={(e) => commit(e.currentTarget.value)}>
          {#each setting.choices ?? [] as choice (choice)}
            <option value={choice}>{wordsFor(choice)}</option>
          {/each}
        </select>
      </span>
    {:else if setting.kind === "language"}
      <span class="select-wrap">
        <select {id} class="input" value={setting.value} {disabled} onchange={(e) => commit(e.currentTarget.value)}>
          {#each languages(setting.value) as l (l.tag)}
            <option value={l.tag}>{l.label}</option>
          {/each}
        </select>
      </span>
    {:else if setting.kind === "integer"}
      <input
        {id}
        class="input count"
        type="number"
        min={range?.[0] ?? 1}
        max={range?.[1]}
        value={setting.value}
        {disabled}
        onchange={(e) => commit(e.currentTarget.value)}
      />
    {:else if setting.kind === "font"}
      <span class="pair">
        <input
          {id}
          class="input text"
          type="text"
          dir="auto"
          value={setting.value}
          spellcheck="false"
          {disabled}
          onchange={(e) => commit(e.currentTarget.value)}
        />
        <button type="button" class="btn small" {disabled} onclick={() => (picking = true)}>{t("choose")}</button>
      </span>
      {#if picking}
        <FontPicker current={setting.value} onchoose={(family) => commit(family)} onclose={() => (picking = false)} />
      {/if}
    {:else}
      <input
        {id}
        class="input"
        class:text={wide(setting)}
        class:count={!wide(setting)}
        class:bad={errors.length > 0}
        type="text"
        dir="auto"
        value={setting.value}
        placeholder={placeholderFor(key) ?? ""}
        spellcheck="false"
        {disabled}
        onchange={(e) => commit(e.currentTarget.value)}
      />
    {/if}
  </div>

  {#if anyOverridden && session.editable}
    <div class="origin">
      <span class="mono">{origin ?? t("setInProject")}</span>
      <button type="button" class="link" onclick={() => void reset()}>{t("reset")}</button>
    </div>
  {/if}

  {#each errors as error (error.code + error.message)}
    <p class="error">{error.message}{error.help ? ` — ${error.help}` : ""}</p>
  {/each}
</div>

<style>
  .row {
    padding: 9px 10px;
    margin-inline: -10px;
    border-block-end: 1px solid var(--line);
    border-radius: var(--radius);
  }
  .row.lit {
    background: var(--acctint);
  }
  .row.lit .name {
    color: var(--acctext);
    font-weight: 600;
  }
  .row.idle .label {
    color: var(--mut);
  }
  .main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .label {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    cursor: default;
  }
  .name {
    white-space: nowrap;
  }
  .hint {
    font-size: 12px;
    color: var(--mut);
    white-space: normal;
  }
  .count {
    inline-size: 72px;
    text-align: end;
  }
  .text {
    inline-size: 170px;
  }
  .pair {
    display: inline-flex;
    gap: 6px;
    min-inline-size: 0;
  }
  .pair .text {
    inline-size: 120px;
  }
  .select-wrap {
    max-inline-size: 60%;
  }
  .origin {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-block-start: 5px;
    font-size: 12px;
    color: var(--mut);
  }
  .origin .mono {
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--err-ink);
  }
</style>
