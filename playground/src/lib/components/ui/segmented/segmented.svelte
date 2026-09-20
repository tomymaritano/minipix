<script lang="ts">
  import { cn } from '$lib/utils';

  interface SegmentedItem {
    value: string;
    label: string;
  }

  interface Props {
    value: string;
    items: SegmentedItem[];
    onValueChange: (value: string) => void;
    'aria-label'?: string;
    class?: string;
  }

  const {
    value,
    items,
    onValueChange,
    'aria-label': ariaLabel = 'Zoom',
    class: className,
  }: Props = $props();
</script>

<div
  role="group"
  aria-label={ariaLabel}
  class={cn(
    'inline-flex items-center gap-0.5 rounded-md border border-border bg-card p-0.5',
    className,
  )}
>
  {#each items as item (item.value)}
    <button
      type="button"
      aria-pressed={value === item.value}
      aria-label={item.label}
      onclick={() => onValueChange(item.value)}
      class={cn(
        'h-[22px] min-w-[30px] rounded-sm px-1.5 text-[11px] font-medium tabular-nums transition-colors duration-150',
        value === item.value
          ? 'bg-secondary text-foreground shadow-[var(--shadow-sm)]'
          : 'text-muted-foreground hover:text-foreground',
      )}
    >
      {item.label}
    </button>
  {/each}
</div>
