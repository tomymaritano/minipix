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
      'inline-flex items-center gap-[6px] rounded-[6px] border border-[var(--line)] px-[9px] py-[4px] text-[12px] font-medium text-[var(--fg)] transition-colors duration-150 hover:border-[var(--line-strong)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] cursor-pointer',
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
      class="text-[var(--fg-dim)] shrink-0"
    >
      <path d="M1 1l3.5 3.5L8 1" stroke="currentColor" stroke-width="1.2" fill="none" />
    </svg>
  </SelectPrimitive.Trigger>

  <SelectPrimitive.Content
    class="z-50 min-w-[80px] overflow-hidden rounded-[6px] border border-[var(--line)] bg-[var(--bg-elev)] py-1 shadow-xl"
    sideOffset={6}
  >
    <SelectPrimitive.Viewport>
      {#each items as item (item.value)}
        <SelectPrimitive.Item
          value={item.value}
          label={item.label}
          class={cn(
            'relative flex cursor-pointer select-none items-center px-[10px] py-[5px] text-[12px] text-[var(--fg)] transition-colors duration-100 outline-none',
            'data-[highlighted]:bg-[var(--mint)] data-[highlighted]:text-[var(--bg)]',
            'data-[selected]:text-[var(--mint)]',
          )}
        >
          {item.label}
        </SelectPrimitive.Item>
      {/each}
    </SelectPrimitive.Viewport>
  </SelectPrimitive.Content>
</SelectPrimitive.Root>
