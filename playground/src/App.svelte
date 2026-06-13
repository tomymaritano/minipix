<script lang="ts">
  import { playground } from './lib/state.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import Viewer from './lib/components/Viewer.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';

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

<!-- Brand mark -->
<header
  class="absolute top-[22px] left-[26px] z-30 inline-flex items-center gap-2 text-[13px] tracking-tight select-none"
>
  <span class="text-[10px] text-[var(--mint)]">◆</span>
  <span class="text-[var(--fg)] font-medium">minipix</span>
</header>

{#if playground.active}
  <Viewer job={playground.active} />
{:else}
  <EmptyState {openPicker} {isMac} />
{/if}

<Toolbar active={playground.active} />

{#if dragOver}
  <div
    class="fixed inset-[9px] z-[70] grid place-items-center bg-black/70 backdrop-blur-sm border border-dashed border-[var(--mint)] rounded-sm animate-[pop_0.15s_ease]"
    style="animation: pop 0.15s var(--ease);"
  >
    <span class="text-[15px] text-[var(--mint)] tracking-wide">Drop to compress</span>
  </div>
{/if}

<style>
  @keyframes pop {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
