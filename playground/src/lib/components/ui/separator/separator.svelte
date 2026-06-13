<script lang="ts">
  import { Separator as SeparatorPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';

  interface Props {
    orientation?: 'horizontal' | 'vertical';
    class?: string;
  }

  const { orientation = 'horizontal', class: className }: Props = $props();
</script>

<SeparatorPrimitive.Root
  {orientation}
  class={cn(
    'shrink-0 bg-[var(--line)]',
    orientation === 'vertical' ? 'h-full w-px' : 'h-px w-full',
    className,
  )}
/>
