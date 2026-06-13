<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';

  interface Props {
    /** Tooltip text. */
    content: string;
    side?: 'top' | 'bottom' | 'left' | 'right';
    sideOffset?: number;
    delayDuration?: number;
    class?: string;
    /** The trigger element(s). */
    children: Snippet;
  }

  const {
    content,
    side = 'top',
    sideOffset = 8,
    delayDuration = 200,
    class: className,
    children,
  }: Props = $props();
</script>

<TooltipPrimitive.Provider {delayDuration} disableHoverableContent>
  <TooltipPrimitive.Root>
    <TooltipPrimitive.Trigger class="inline-flex">
      {@render children()}
    </TooltipPrimitive.Trigger>
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        {side}
        {sideOffset}
        class={cn(
          'z-[80] rounded-[6px] border border-[var(--line)] bg-[var(--bg-3)] px-2 py-1 text-[11px] font-medium text-[var(--fg)] shadow-[var(--shadow-lg)] tt-content',
          className,
        )}
      >
        {content}
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  </TooltipPrimitive.Root>
</TooltipPrimitive.Provider>

<style>
  :global(.tt-content) {
    animation: tt-in 0.14s var(--ease);
  }
  @keyframes tt-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
