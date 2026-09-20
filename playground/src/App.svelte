<script lang="ts">
  import { ChevronLeft, ChevronRight, X } from '@lucide/svelte';
  import { playground } from './lib/state.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import Viewer from './lib/components/Viewer.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';
  import { Button } from '$lib/components/ui/button';

  const isMac =
    typeof navigator !== 'undefined' && /mac/i.test(navigator.platform || navigator.userAgent);

  let fileInput: HTMLInputElement;
  let dragDepth = $state(0);
  const dragOver = $derived(dragDepth > 0);

  const ACCEPT = 'image/png,image/jpeg,image/webp,image/avif';

  function isImage(f: File): boolean {
    if (/^image\/(png|jpe?g|webp|avif)$/.test(f.type)) return true;
    return /\.(png|jpe?g|webp|avif)$/i.test(f.name);
  }

  function take(files: FileList | File[] | null | undefined): void {
    if (!files) return;
    const imgs = Array.from(files).filter(isImage);
    if (imgs.length) playground.addFiles(imgs);
  }

  function openPicker(): void {
    fileInput.click();
  }

  function onInputChange(e: Event): void {
    const input = e.currentTarget as HTMLInputElement;
    take(input.files);
    input.value = '';
  }

  function onDragEnter(e: DragEvent): void {
    if (e.dataTransfer?.types.includes('Files')) dragDepth++;
  }
  function onDragLeave(): void {
    if (dragDepth > 0) dragDepth--;
  }
  function onDragOver(e: DragEvent): void {
    if (e.dataTransfer?.types.includes('Files')) e.preventDefault();
  }
  function onDrop(e: DragEvent): void {
    e.preventDefault();
    dragDepth = 0;
    take(e.dataTransfer?.files);
  }

  function onPaste(e: ClipboardEvent): void {
    const items = e.clipboardData?.items;
    if (!items) return;
    const files: File[] = [];
    for (const it of items) {
      if (it.kind === 'file') {
        const f = it.getAsFile();
        if (f) files.push(f);
      }
    }
    take(files);
  }

  function onKeydown(e: KeyboardEvent): void {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      openPicker();
      return;
    }
    if (!playground.active) return;
    if (e.key === 'ArrowLeft') playground.step(-1);
    else if (e.key === 'ArrowRight') playground.step(1);
    else if (e.key === 'Escape') playground.removeActive();
  }
</script>

<svelte:window
  ondragenter={onDragEnter}
  ondragleave={onDragLeave}
  ondragover={onDragOver}
  ondrop={onDrop}
  onpaste={onPaste}
  onkeydown={onKeydown}
/>

<input
  bind:this={fileInput}
  type="file"
  multiple
  accept={ACCEPT}
  onchange={onInputChange}
  hidden
  aria-hidden="true"
  tabindex="-1"
/>

<div class="relative z-[2] flex h-full flex-col p-2">
  <header class="relative flex h-12 shrink-0 items-center px-3">
    <div class="flex select-none items-center gap-2 text-[13px] tracking-tight">
      <span class="font-medium text-foreground">minipix</span>
      <span class="hidden text-[10px] tracking-[0.04em] text-muted-foreground sm:inline"
        >/ playground</span
      >
    </div>

    {#if playground.position && playground.position.total > 1}
      <div
        class="absolute left-1/2 inline-flex -translate-x-1/2 items-center gap-2.5 text-[12px] tabular-nums text-muted-foreground"
      >
        <Button
          variant="icon"
          size="sm"
          class="h-6 w-6 rounded-full px-0"
          onclick={() => playground.step(-1)}
          aria-label="Previous image"
        >
          <ChevronLeft size={16} />
        </Button>
        <span class="min-w-[42px] text-center font-medium text-foreground">
          {playground.position.index}
          <span class="text-muted-foreground">/</span>
          {playground.position.total}
        </span>
        <Button
          variant="icon"
          size="sm"
          class="h-6 w-6 rounded-full px-0"
          onclick={() => playground.step(1)}
          aria-label="Next image"
        >
          <ChevronRight size={16} />
        </Button>
      </div>
    {/if}

    {#if playground.active}
      <Button
        variant="icon"
        size="icon"
        class="ml-auto"
        onclick={() => playground.removeActive()}
        aria-label="Close"
      >
        <X size={15} />
      </Button>
    {/if}
  </header>

  <div class="relative min-h-0 flex-1">
    {#if playground.active}
      <Viewer job={playground.active} />
    {:else}
      <EmptyState {openPicker} {isMac} />
    {/if}

    {#if dragOver}
      <div
        class="drop-overlay pointer-events-none absolute inset-0 z-20 grid place-items-center rounded-xl border-2 border-dashed border-primary bg-background/70"
      >
        <span class="text-[16px] font-medium tracking-[0.01em] text-primary">Drop to compress</span>
      </div>
    {/if}
  </div>

  <Toolbar active={playground.active} />
</div>

<style>
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.99);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  .drop-overlay {
    animation: pop 0.15s var(--ease);
  }
</style>
