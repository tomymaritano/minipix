<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLSpanElement> {
    variant?: 'default' | 'warning' | 'outline';
    class?: string;
  }

  const { variant = 'default', class: className, children, ...rest }: Props = $props();

  const base =
    'inline-flex items-center justify-center font-semibold tabular-nums text-[11px] leading-none rounded-full border px-2 py-0.5 transition-colors duration-150';

  const variants: Record<string, string> = {
    default: 'text-primary bg-primary/16 border-primary/30',
    warning: 'text-warning bg-warning/16 border-warning/30',
    outline: 'text-muted-foreground bg-transparent border-border',
  };
</script>

<span class={cn(base, variants[variant], className)} {...rest}>
  {@render children?.()}
</span>
