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
      'relative inline-flex h-[16px] w-[28px] shrink-0 items-center rounded-full border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-2)]',
      checked
        ? 'bg-[var(--mint)] border-[var(--mint)] shadow-[0_0_10px_-2px_var(--mint-glow)]'
        : 'bg-[var(--bg-3)] border-[var(--line-strong)]',
      disabled && 'pointer-events-none',
    )}
  >
    <SwitchPrimitive.Thumb
      class={cn(
        'pointer-events-none block h-[11px] w-[11px] rounded-full shadow-[0_1px_2px_rgba(0,0,0,0.4)] transition-transform duration-150 ease-[var(--ease)]',
        checked ? 'translate-x-[14px] bg-[var(--bg)]' : 'translate-x-[2px] bg-[var(--fg-dim)]',
      )}
    />
  </SwitchPrimitive.Root>
  {#if label}
    <span
      class={cn(
        'text-[12px] tracking-[0.01em]',
        checked ? 'text-[var(--fg)]' : 'text-[var(--fg-dim)]',
        disabled && 'text-[var(--fg-faint)]',
      )}>{label}</span
    >
  {/if}
</label>
