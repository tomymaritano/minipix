<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'default' | 'ghost' | 'secondary';
    size?: 'default' | 'sm';
    class?: string;
  }

  const {
    variant = 'default',
    size = 'default',
    class: className,
    children,
    ...rest
  }: Props = $props();

  const base =
    'inline-flex items-center justify-center font-medium transition-all duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer';

  const variants: Record<string, string> = {
    default: 'bg-[var(--ink-btn)] text-[var(--ink-btn-fg)] hover:-translate-y-px hover:shadow-lg',
    ghost: 'bg-transparent text-[var(--fg-dim)] hover:text-[var(--fg)] hover:bg-white/5',
    secondary:
      'bg-[var(--bg-elev)] text-[var(--fg)] border border-[var(--line)] hover:border-[var(--line-strong)]',
  };

  const sizes: Record<string, string> = {
    default: 'px-[14px] py-[6px] text-[12px] rounded-[6px]',
    sm: 'px-[9px] py-[2px] text-[12px] rounded-[5px]',
  };
</script>

<button class={cn(base, variants[variant], sizes[size], className)} {...rest}>
  {@render children?.()}
</button>
