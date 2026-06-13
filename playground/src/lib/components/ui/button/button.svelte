<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'default' | 'primary' | 'ghost' | 'secondary' | 'icon';
    size?: 'default' | 'sm' | 'icon';
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
    'relative inline-flex items-center justify-center font-medium select-none transition-[transform,background-color,border-color,box-shadow,color] duration-150 ease-[var(--ease)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg)] disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none cursor-pointer active:translate-y-0';

  const variants: Record<string, string> = {
    // neutral ink — default actions
    default:
      'bg-[var(--ink-btn)] text-[var(--ink-btn-fg)] shadow-[var(--shadow-sm)] hover:-translate-y-px hover:shadow-[var(--shadow-md)]',
    // emerald signal — primary CTA (Download, Select Files)
    primary:
      'bg-[var(--mint)] text-[var(--mint-foreground,#06120d)] shadow-[0_2px_14px_-2px_var(--mint-glow)] hover:-translate-y-px hover:bg-[var(--mint-bright)] hover:shadow-[0_6px_22px_-4px_var(--mint-glow)]',
    ghost: 'bg-transparent text-[var(--fg-dim)] hover:text-[var(--fg)] hover:bg-white/[0.06]',
    secondary:
      'bg-[var(--bg-3)] text-[var(--fg)] border border-[var(--line)] hover:border-[var(--line-strong)] hover:bg-[var(--bg-3)]',
    icon: 'bg-transparent text-[var(--fg-dim)] hover:text-[var(--fg)] hover:bg-white/[0.06]',
  };

  const sizes: Record<string, string> = {
    default: 'gap-[8px] px-[14px] h-[34px] text-[12px] rounded-[7px]',
    sm: 'gap-[5px] px-[9px] h-[24px] text-[11px] rounded-[6px]',
    icon: 'h-[30px] w-[30px] rounded-[7px]',
  };
</script>

<button class={cn(base, variants[variant], sizes[size], className)} {...rest}>
  {@render children?.()}
</button>
