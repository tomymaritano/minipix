<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLSpanElement> {
    variant?: 'default' | 'warning';
    class?: string;
  }

  const { variant = 'default', class: className, children, ...rest }: Props = $props();

  const base = 'inline-flex items-center font-semibold tabular-nums text-[12px] ml-0.5';

  const variants: Record<string, string> = {
    default: 'text-[var(--mint)]',
    warning: 'text-[var(--warn)]',
  };
</script>

<span class={cn(base, variants[variant], className)} {...rest}>
  {@render children?.()}
</span>
