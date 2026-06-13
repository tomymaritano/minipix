<script lang="ts">
  import { Slider as SliderPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';

  interface Props {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    onValueChange?: (value: number) => void;
    class?: string;
  }

  const {
    value = 75,
    min = 1,
    max = 100,
    step = 1,
    disabled = false,
    onValueChange,
    class: className,
  }: Props = $props();

  function handleChange(v: number) {
    onValueChange?.(v);
  }
</script>

<SliderPrimitive.Root
  type="single"
  {value}
  {min}
  {max}
  {step}
  {disabled}
  onValueChange={handleChange}
  class={cn(
    'relative flex w-[132px] touch-none select-none items-center',
    disabled && 'opacity-32 cursor-not-allowed',
    className,
  )}
>
  {#snippet children({ thumbItems })}
    <!-- Track -->
    <span
      class="relative h-[3px] w-full grow overflow-hidden rounded-full bg-[var(--bg-3)] ring-1 ring-inset ring-[var(--line-soft)]"
    >
      <span
        class="absolute inset-y-0 left-0 rounded-full bg-[var(--mint)] shadow-[0_0_8px_-1px_var(--mint-glow)] transition-[width] duration-75"
        style:width="{((value - min) / (max - min)) * 100}%"
      ></span>
    </span>
    <!-- Thumb -->
    {#each thumbItems as thumb (thumb.index)}
      <SliderPrimitive.Thumb
        index={thumb.index}
        class="block h-[14px] w-[14px] rounded-full border border-[var(--line-strong)] bg-[var(--ink-btn)] shadow-[var(--shadow-md)] transition-transform duration-100 hover:scale-115 active:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-2)] disabled:pointer-events-none"
      />
    {/each}
  {/snippet}
</SliderPrimitive.Root>
