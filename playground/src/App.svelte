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
  class="absolute top-[20px] left-[24px] z-30 inline-flex select-none items-center gap-[8px] text-[13px] tracking-tight"
>
  <span
    class="grid h-[20px] w-[20px] place-items-center rounded-[6px] border border-[rgba(52,211,153,0.3)] bg-[rgba(52,211,153,0.1)] text-[9px] text-[var(--mint)] shadow-[0_0_12px_-4px_var(--mint-glow)]"
    >◆</span
  >
  <span class="font-medium text-[var(--fg)]">minipix</span>
  <span class="hidden text-[10px] tracking-[0.04em] text-[var(--fg-faint)] sm:inline"
    >/ playground</span
  >
</header>

{#if playground.active}
  <Viewer job={playground.active} />
{:else}
  <EmptyState {openPicker} {isMac} />
{/if}

<Toolbar active={playground.active} />

{#if dragOver}
  <div
    class="drop-overlay fixed inset-[10px] z-[70] grid place-items-center rounded-[14px] border-2 border-dashed border-[var(--mint)] backdrop-blur-md"
    style="background: radial-gradient(60% 50% at 50% 50%, rgba(52,211,153,0.12), rgba(0,0,0,0.66) 75%);"
  >
    <div class="flex flex-col items-center gap-[14px]">
      <div
        class="grid h-[60px] w-[60px] place-items-center rounded-[18px] border border-[rgba(52,211,153,0.4)] bg-[rgba(52,211,153,0.12)] shadow-[0_0_40px_-8px_var(--mint-glow)]"
      >
        <svg
          width="28"
          height="28"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--mint)"
          stroke-width="1.4"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      </div>
      <span class="text-[16px] font-medium tracking-[0.01em] text-[var(--mint)]"
        >Drop to compress</span
      >
    </div>
  </div>
{/if}

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
