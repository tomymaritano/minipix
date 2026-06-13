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
    'rounded-[var(--radius)] border border-[var(--line)] bg-[var(--bg-2)] text-[var(--fg)] shadow-[var(--shadow-md)]',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</div>
