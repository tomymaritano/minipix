<script lang="ts">
  import { Switch as SwitchPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    onCheckedChange?: (checked: boolean) => void;
    class?: string;
    label?: string;
  }

  const {
    checked = false,
    disabled = false,
    onCheckedChange,
    class: className,
    label,
  }: Props = $props();
</script>

<label
  class={cn(
    'inline-flex items-center gap-[7px] text-[12px] select-none',
    disabled ? 'opacity-32 cursor-not-allowed' : 'cursor-pointer',
    className,
  )}
>
  <SwitchPrimitive.Root
    {checked}
    {disabled}
    {onCheckedChange}
    class={cn(
      'relative inline-flex h-[16px] w-[28px] shrink-0 items-center rounded-full border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
      checked ? 'bg-primary border-primary' : 'bg-secondary border-border',
      disabled && 'pointer-events-none',
    )}
  >
    <SwitchPrimitive.Thumb
      class={cn(
        'pointer-events-none block h-[11px] w-[11px] rounded-full shadow-[0_1px_2px_rgba(0,0,0,0.4)] transition-transform duration-150 ease-[var(--ease)]',
        checked ? 'translate-x-[14px] bg-background' : 'translate-x-[2px] bg-muted-foreground',
      )}
    />
  </SwitchPrimitive.Root>
  {#if label}
    <span
      class={cn(
        'text-[12px] tracking-[0.01em]',
        checked ? 'text-foreground' : 'text-muted-foreground',
        disabled && 'text-muted-foreground',
      )}>{label}</span
    >
  {/if}
</label>
