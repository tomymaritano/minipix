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
    'relative flex w-[150px] touch-none select-none items-center',
    disabled && 'opacity-32 cursor-not-allowed',
    className,
  )}
>
  {#snippet children({ thumbItems })}
    <!-- Track -->
    <span class="relative h-[2px] w-full grow overflow-hidden rounded-full bg-[var(--fg-ghost)]">
      <span
        class="absolute h-full bg-[var(--mint)]"
        style:width="{((value - min) / (max - min)) * 100}%"
      ></span>
    </span>
    <!-- Thumb -->
    {#each thumbItems as thumb (thumb.index)}
      <SliderPrimitive.Thumb
        index={thumb.index}
        class="block h-[12px] w-[12px] rounded-full bg-[var(--ink-btn)] shadow-md transition-transform duration-100 hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] disabled:pointer-events-none"
      />
    {/each}
  {/snippet}
</SliderPrimitive.Root>
