<script lang="ts">
  import { playground } from '../state.svelte.js';

  let dragOver = $state(false);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  function handleFiles(files: FileList | null | undefined): void {
    if (!files || files.length === 0) return;
    const accepted: File[] = [];
    for (const file of files) {
      if (/^image\/(png|jpeg|webp|avif)$/.test(file.type)) {
        accepted.push(file);
      }
    }
    if (accepted.length > 0) {
      playground.addFiles(accepted);
    }
  }

  function onDragOver(e: DragEvent): void {
    e.preventDefault();
    dragOver = true;
  }

  function onDragLeave(): void {
    dragOver = false;
  }

  function onDrop(e: DragEvent): void {
    e.preventDefault();
    dragOver = false;
    handleFiles(e.dataTransfer?.files);
  }

  function onInputChange(e: Event): void {
    const target = e.currentTarget;
    if (target instanceof HTMLInputElement) {
      handleFiles(target.files);
      // Reset so the same file can be re-added after a retry.
      target.value = '';
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      inputEl?.click();
    }
  }
</script>

<div
  class="dropzone"
  class:dragover={dragOver}
  role="button"
  tabindex="0"
  aria-label="Drop images here or click to select files"
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  onkeydown={onKeydown}
  onclick={() => inputEl?.click()}
>
  <input
    bind:this={inputEl}
    type="file"
    multiple
    accept="image/png,image/jpeg,image/webp,image/avif"
    aria-hidden="true"
    tabindex="-1"
    onchange={onInputChange}
  />

  <div class="dropzone__content">
    <svg
      class="dropzone__icon"
      aria-hidden="true"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </svg>
    <p class="dropzone__label">Drop images here</p>
    <p class="dropzone__sub">or click to select PNG, JPEG, WebP, AVIF</p>
  </div>
</div>

<style>
  .dropzone {
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--color-border);
    border-radius: 10px;
    padding: 2.5rem 1.5rem;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background-color 0.15s;
    background-color: var(--color-surface);
    outline: none;
    user-select: none;
  }

  .dropzone:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-muted);
  }

  .dropzone.dragover {
    border-color: var(--color-accent);
    background-color: var(--color-accent-muted);
  }

  .dropzone input {
    display: none;
  }

  .dropzone__content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    pointer-events: none;
  }

  .dropzone__icon {
    width: 2.5rem;
    height: 2.5rem;
    color: var(--color-muted);
    margin-bottom: 0.25rem;
  }

  .dropzone__label {
    font-size: 1rem;
    font-weight: 500;
    color: var(--color-text);
  }

  .dropzone__sub {
    font-size: 0.8125rem;
    color: var(--color-muted);
  }
</style>
