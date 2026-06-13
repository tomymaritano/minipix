<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLSpanElement> {
    variant?: 'default' | 'warning' | 'outline';
    class?: string;
  }

  const { variant = 'default', class: className, children, ...rest }: Props = $props();

  const base =
    'inline-flex items-center justify-center font-semibold tabular-nums text-[11px] leading-none rounded-full border px-[8px] py-[3px] transition-colors duration-150';

  const variants: Record<string, string> = {
    default: 'text-[var(--mint)] bg-[var(--mint-dim)] border-[rgba(52,211,153,0.28)]',
    warning: 'text-[var(--warn)] bg-[var(--warn-dim)] border-[rgba(245,181,68,0.28)]',
    outline: 'text-[var(--fg-dim)] bg-transparent border-[var(--line)]',
  };
</script>

<span class={cn(base, variants[variant], className)} {...rest}>
  {@render children?.()}
</span>
