<script lang="ts">
  /**
   * A switch: on or off, as a pill with a knob, the way the design draws
   * every yes-or-no question. A button with the switch role rather than a
   * checkbox restyled, so the keyboard and a screen reader get the truth
   * without a stylesheet's help.
   */
  const {
    checked,
    disabled = false,
    label,
    onchange,
  }: {
    checked: boolean;
    disabled?: boolean;
    /** The accessible name, when the visible label is not tied to it. */
    label?: string;
    onchange: (next: boolean) => void;
  } = $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    flex: none;
    inline-size: 32px;
    block-size: 18px;
    padding: 0;
    border: 0;
    border-radius: 9px;
    background: var(--line2);
    cursor: pointer;
    transition: background 120ms ease-out;
  }
  .toggle[aria-checked="true"] {
    background: var(--acc);
  }
  .toggle:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .toggle:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 2px;
  }
  .knob {
    position: absolute;
    inset-block-start: 2px;
    inset-inline-start: 2px;
    inline-size: 14px;
    block-size: 14px;
    border-radius: 7px;
    background: var(--knob);
    transition: inset-inline-start 120ms ease-out;
  }
  .toggle[aria-checked="true"] .knob {
    inset-inline-start: 16px;
  }
</style>
