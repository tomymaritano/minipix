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
    'relative flex w-[132px] max-[899px]:w-[96px] touch-none select-none items-center',
    disabled && 'opacity-32 cursor-not-allowed',
    className,
  )}
>
  {#snippet children({ thumbItems })}
    <!-- Track -->
    <span class="relative h-[3px] w-full grow overflow-hidden rounded-full bg-secondary">
      <span
        class="absolute inset-y-0 left-0 rounded-full bg-primary transition-[width] duration-75"
        style:width="{((value - min) / (max - min)) * 100}%"
      ></span>
    </span>
    <!-- Thumb -->
    {#each thumbItems as thumb (thumb.index)}
      <SliderPrimitive.Thumb
        index={thumb.index}
        class="block h-3.5 w-3.5 rounded-full border border-border bg-foreground shadow-[var(--shadow-md)] transition-transform duration-100 hover:scale-115 active:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none"
      />
    {/each}
  {/snippet}
</SliderPrimitive.Root>
