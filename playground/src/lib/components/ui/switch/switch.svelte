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
      'relative inline-flex h-[14px] w-[26px] shrink-0 items-center rounded-full border border-[var(--line-strong)] transition-colors duration-150',
      checked ? 'bg-[var(--mint)] border-[var(--mint)]' : 'bg-[var(--fg-ghost)]',
      disabled && 'pointer-events-none',
    )}
  >
    <SwitchPrimitive.Thumb
      class={cn(
        'pointer-events-none block h-[10px] w-[10px] rounded-full shadow-sm transition-transform duration-150',
        checked ? 'translate-x-[14px] bg-[var(--bg)]' : 'translate-x-[1px] bg-[var(--fg-dim)]',
      )}
    />
  </SwitchPrimitive.Root>
  {#if label}
    <span class={cn('text-[var(--fg)]', disabled && 'text-[var(--fg-faint)]')}>{label}</span>
  {/if}
</label>
