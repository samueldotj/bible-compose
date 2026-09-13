<script lang="ts">
  /**
   * One head or foot slot: a dropdown of the fields it usually holds, and a
   * box for the rest.
   *
   * The dropdown offers the fields one at a time because that is what most
   * heads are; Custom… opens the box for a template such as
   * `{Book} {Range}`, with the `?` beside it listing what a template can
   * name. The same control sits on the page itself, compact, and in the
   * inspector at full size.
   */
  import HeadFieldsHelp from "../HeadFieldsHelp.svelte";
  import { setSlot } from "../../lib/heads";
  import { session } from "../../lib/session.svelte";
  import { ui } from "../../lib/ui.svelte";
  import { t } from "../../lib/i18n";

  const {
    key,
    name,
    compact = false,
  }: {
    key: string;
    /** The accessible name: which side, which line, which slot. */
    name: string;
    /** On the page: smaller, and the dropdown alone until Custom… is chosen. */
    compact?: boolean;
  } = $props();

  $effect(() => {
    void session.loadHeadFields();
  });

  const value = $derived(session.settings.find((s) => s.key === key)?.value ?? "");
  const errors = $derived(session.fieldErrors[key] ?? []);

  /** A field name as it is compared: lower-case, underscores dropped. */
  const fold = (n: string) => n.replace(/_/g, "").toLowerCase();

  /**
   * Which dropdown entry a template is: `empty`, one field on its own, or
   * `custom` for anything else.
   */
  const entry = $derived.by(() => {
    if (value === "") return "empty";
    const one = /^\{([A-Za-z_]+)\}$/.exec(value);
    const field = one && (session.headFields ?? []).find((f) => fold(f.name) === fold(one[1]!));
    return field ? field.name : "custom";
  });

  let customising = $state(false);
  let help = $state(false);
  const showBox = $derived(customising || entry === "custom");

  function pick(next: string): void {
    if (next === "custom") {
      customising = true;
      return;
    }
    customising = false;
    void setSlot(key, next === "empty" ? "" : `{${next}}`);
  }

  const lit = $derived(ui.lit === key);
</script>

<span
  class="slot"
  role="group"
  class:compact
  class:lit
  class:filled={value !== ""}
  data-search-key={key}
  onpointerenter={() => (ui.lit = key)}
  onpointerleave={() => (ui.lit = null)}
  onfocusin={() => (ui.lit = key)}
  onfocusout={() => (ui.lit = null)}
>
  <span class="select-wrap">
    <select
      class="input"
      aria-label={name}
      value={customising ? "custom" : entry}
      disabled={!session.editable}
      onchange={(e) => pick(e.currentTarget.value)}
    >
      <option value="empty">{t("emptySlot")}</option>
      {#each session.headFields ?? [] as field (field.name)}
        <option value={field.name}>{field.label}</option>
      {/each}
      <option value="custom">{t("customSlot")}</option>
    </select>
  </span>
  {#if showBox}
    <span class="box">
      <input
        type="text"
        class="input mono"
        aria-label={name}
        {value}
        placeholder={t("templateHint")}
        spellcheck="false"
        disabled={!session.editable}
        onchange={(e) => void setSlot(key, e.currentTarget.value)}
      />
      <button
        type="button"
        class="help"
        aria-label={t("headFieldsTitle")}
        title={t("headFieldsTitle")}
        onclick={() => (help = true)}>{t("fieldsHelp")}</button
      >
    </span>
    {#each errors as problem (problem.message)}
      <span class="error">{problem.message}</span>
    {/each}
  {/if}
</span>

{#if help}
  <HeadFieldsHelp onclose={() => (help = false)} />
{/if}

<style>
  .slot {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-inline-size: 0;
  }
  .slot .select-wrap {
    inline-size: 100%;
  }
  .box {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .box .input {
    flex: 1;
    min-inline-size: 0;
  }
  .help {
    flex: none;
    inline-size: 26px;
    block-size: 26px;
    padding: 0;
    border: 1px solid var(--line2);
    border-radius: 50%;
    background: var(--field);
    color: var(--ink);
    font: 700 12px var(--sans);
    cursor: pointer;
  }
  .help:hover {
    border-color: var(--acc);
  }
  .error {
    font-size: 11.5px;
    color: var(--err-ink);
  }

  /* On the page: a small pill over the slot it fills, outlined in the
     accent when it holds something and dashed when it does not. */
  .compact .input {
    block-size: 24px;
    padding-inline: 7px 22px;
    border: 1px dashed var(--line2);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.85);
    color: #6e6860;
    font-size: 11px;
  }
  .compact .box .input {
    padding-inline: 7px;
  }
  .compact.filled .input,
  .compact.lit .input {
    border: 1px solid var(--acc);
    color: #2f4763;
    font-weight: 600;
  }
  .compact .help {
    inline-size: 22px;
    block-size: 22px;
    font-size: 11px;
  }
  .compact .select-wrap::after {
    inset-inline-end: 8px;
  }
</style>
