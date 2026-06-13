<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';

  interface SelectItem {
    value: string;
    label: string;
  }

  interface Props {
    value?: string;
    items: SelectItem[];
    onValueChange?: (value: string) => void;
    /** aria-label forwarded to the trigger button (required for e2e: 'Output format') */
    'aria-label'?: string;
    class?: string;
  }

  const {
    value,
    items,
    onValueChange,
    'aria-label': ariaLabel,
    class: className,
  }: Props = $props();

  const currentLabel = $derived(items.find((i) => i.value === value)?.label ?? '');
</script>

<SelectPrimitive.Root type="single" {value} {onValueChange}>
  <SelectPrimitive.Trigger
    aria-label={ariaLabel}
    class={cn(
      'select-trigger inline-flex h-[30px] items-center gap-[7px] rounded-[7px] border border-[var(--line)] bg-[var(--bg-3)] px-[11px] text-[12px] font-medium text-[var(--fg)] transition-colors duration-150 hover:border-[var(--line-strong)] data-[state=open]:border-[var(--mint)] data-[state=open]:ring-2 data-[state=open]:ring-[var(--mint-dim)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] cursor-pointer',
      className,
    )}
  >
    <SelectPrimitive.Value placeholder="Format">
      {currentLabel}
    </SelectPrimitive.Value>
    <svg
      width="9"
      height="6"
      viewBox="0 0 9 6"
      aria-hidden="true"
      class="select-chevron text-[var(--fg-faint)] shrink-0 transition-transform duration-150"
    >
      <path
        d="M1 1l3.5 3.5L8 1"
        stroke="currentColor"
        stroke-width="1.4"
        fill="none"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </SelectPrimitive.Trigger>

  <SelectPrimitive.Portal>
    <SelectPrimitive.Content
      class="z-[80] min-w-[var(--bits-select-anchor-width,96px)] overflow-hidden rounded-[8px] border border-[var(--line)] bg-[var(--bg-3)] p-1 shadow-[var(--shadow-lg)] select-content"
      sideOffset={8}
    >
      <SelectPrimitive.Viewport>
        {#each items as item (item.value)}
          <SelectPrimitive.Item
            value={item.value}
            label={item.label}
            class={cn(
              'relative flex cursor-pointer select-none items-center justify-between gap-3 rounded-[5px] px-[10px] py-[6px] text-[12px] font-medium text-[var(--fg-dim)] transition-colors duration-100 outline-none',
              'data-[highlighted]:bg-white/[0.06] data-[highlighted]:text-[var(--fg)]',
              'data-[selected]:text-[var(--mint)]',
            )}
          >
            {#snippet children({ selected })}
              {item.label}
              {#if selected}
                <svg width="11" height="9" viewBox="0 0 11 9" aria-hidden="true" class="shrink-0">
                  <path
                    d="M1 4.5 4 7.5 10 1"
                    stroke="currentColor"
                    stroke-width="1.4"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              {/if}
            {/snippet}
          </SelectPrimitive.Item>
        {/each}
      </SelectPrimitive.Viewport>
    </SelectPrimitive.Content>
  </SelectPrimitive.Portal>
</SelectPrimitive.Root>

<style>
  :global(.select-trigger[data-state='open'] .select-chevron) {
    transform: rotate(180deg);
    color: var(--mint);
  }
  :global(.select-content) {
    animation: select-in 0.14s var(--ease);
    transform-origin: top center;
  }
  @keyframes select-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
