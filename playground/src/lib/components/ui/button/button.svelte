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
    'relative inline-flex items-center justify-center font-medium select-none transition-[transform,background-color,border-color,box-shadow,color] duration-150 ease-[var(--ease)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none cursor-pointer';

  const variants: Record<string, string> = {
    default:
      'bg-secondary text-foreground border border-border hover:border-foreground/20',
    primary: 'bg-primary text-primary-foreground hover:brightness-110',
    ghost: 'bg-transparent text-muted-foreground hover:text-foreground hover:bg-white/[0.06]',
    secondary: 'bg-secondary text-foreground border border-border hover:border-foreground/20',
    icon: 'bg-transparent text-muted-foreground hover:text-foreground hover:bg-white/[0.06]',
  };

  const sizes: Record<string, string> = {
    default: 'gap-2 px-3.5 h-8 text-[13px] rounded-md',
    sm: 'gap-1 px-2 h-6 text-[11px] rounded-md',
    icon: 'h-8 w-8 rounded-md',
  };
</script>

<button class={cn(base, variants[variant], sizes[size], className)} {...rest}>
  {@render children?.()}
</button>
