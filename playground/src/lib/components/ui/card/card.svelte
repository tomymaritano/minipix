<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    class?: string;
  }

  const { class: className, children, ...rest }: Props = $props();
</script>

<div
  class={cn(
    'rounded-xl border border-border bg-card text-foreground shadow-[var(--shadow-md)]',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</div>
