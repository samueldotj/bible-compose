<script lang="ts">
  /**
   * A few choices in a row, one of them chosen: Spread or Page, Fit or 100%,
   * one column or two. A group of buttons rather than a dropdown, because
   * every option is short and seeing them all is the point.
   */
  const {
    options,
    value,
    disabled = false,
    label,
    onchange,
    small = false,
  }: {
    options: readonly { value: string; label: string }[];
    value: string;
    disabled?: boolean;
    label?: string;
    onchange: (next: string) => void;
    small?: boolean;
  } = $props();
</script>

<span class="segmented" class:small role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      class:active={option.value === value}
      aria-pressed={option.value === value}
      {disabled}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</span>

<style>
  .segmented {
    display: inline-flex;
    overflow: hidden;
    border: 1px solid var(--line2);
    border-radius: var(--radius);
    font-size: 12px;
    white-space: nowrap;
  }
  button {
    padding: 5px 12px;
    border: 0;
    border-inline-start: 1px solid var(--line2);
    background: transparent;
    color: var(--ink);
    font: inherit;
    cursor: pointer;
  }
  .small button {
    padding: 4px 10px;
  }
  button:first-child {
    border-inline-start: 0;
  }
  button:hover:not(:disabled) {
    background: color-mix(in oklab, var(--acctint) 60%, transparent);
  }
  button.active {
    background: var(--acctint);
    color: var(--acctext);
    font-weight: 600;
  }
  button:disabled {
    cursor: default;
    opacity: 0.6;
  }
</style>
