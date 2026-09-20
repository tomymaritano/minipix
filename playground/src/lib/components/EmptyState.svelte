<script lang="ts">
  import { Layers } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';

  interface Props {
    openPicker: () => void;
    isMac: boolean;
  }
  const { openPicker, isMac }: Props = $props();
  const mod = $derived(isMac ? '⌘' : 'Ctrl');

  const formats = ['PNG', 'JPEG', 'WebP', 'AVIF'];
</script>

<div class="flex h-full min-h-0 flex-col">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="group relative flex min-h-0 flex-1 cursor-pointer flex-col items-center justify-center rounded-xl border border-dashed border-border bg-card px-10 py-10 text-center hover:border-primary/55"
    onclick={openPicker}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        openPicker();
      }
    }}
  >
    <div
      class="mb-7 grid h-[84px] w-[84px] place-items-center rounded-xl border border-border bg-secondary"
    >
      <Layers size={40} class="text-primary" />
    </div>

    <h1
      class="text-[18px] font-medium leading-[1.1] tracking-[-0.02em] text-foreground lg:text-[28px]"
    >
      Compress images, privately
    </h1>
    <p class="mt-3 max-w-[400px] text-[13px] leading-relaxed text-muted-foreground">
      Drop a PNG, JPEG, WebP or AVIF. Everything runs in your browser —
      <span class="text-foreground">nothing is ever uploaded.</span>
    </p>

    <div class="mt-8">
      <Button
        variant="primary"
        class="h-10 px-6 text-[13px]"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          openPicker();
        }}
      >
        Select files
        <span
          class="kbd ml-1 border-primary-foreground/25 bg-primary-foreground/15 text-primary-foreground shadow-none"
          >{mod}O</span
        >
      </Button>
    </div>

    <div class="mt-7 flex items-center gap-1.5">
      {#each formats as f, i (f)}
        {#if i > 0}
          <span class="text-muted-foreground/40">·</span>
        {/if}
        <Badge variant="outline" class="px-2 py-1 text-[10px] tracking-[0.06em]">{f}</Badge>
      {/each}
    </div>

    <p class="absolute bottom-4 inline-flex items-center gap-1.5 text-[11px] text-muted-foreground">
      <span class="kbd">{mod}V</span>
      <span>to paste from clipboard</span>
    </p>
  </div>
</div>
