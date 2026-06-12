<script lang="ts">
  import { playground } from './lib/state.svelte.js';
  import DropZone from './lib/components/DropZone.svelte';
  import GlobalControls from './lib/components/GlobalControls.svelte';
  import JobCard from './lib/components/JobCard.svelte';
  import { downloadAllZip } from './lib/zip.js';

  function fmt(bytes: number): string {
    if (bytes < 1024) return `${String(bytes)} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  function handleDownloadAll(): void {
    downloadAllZip(playground.jobs);
  }
</script>

<div class="app">
  <header class="app__header">
    <h1 class="app__title">minipix</h1>
    <p class="app__tagline">Compress images locally — nothing leaves your machine.</p>
  </header>

  <main class="app__main">
    <DropZone />

    <GlobalControls />

    {#if playground.jobs.length > 0}
      <!-- Totals bar -->
      {#if playground.totals.doneCount > 0}
        <div class="totals" aria-live="polite">
          <span class="totals__label">
            {playground.totals.doneCount} file{playground.totals.doneCount !== 1 ? 's' : ''} done
          </span>
          <span class="totals__arrow">·</span>
          <span class="totals__bytes">
            {fmt(playground.totals.bytesIn)} → {fmt(playground.totals.bytesOut)}
          </span>
          <span class="totals__arrow">·</span>
          <span class="totals__saving">
            {playground.totals.percent}% smaller
          </span>

          {#if playground.totals.doneCount >= 2}
            <button class="btn-download-all" onclick={handleDownloadAll}>
              Download all (.zip)
            </button>
          {/if}
        </div>
      {/if}

      <!-- Job list -->
      <ul class="job-list" aria-label="Compression jobs">
        {#each playground.jobs as job (job.id)}
          <li>
            <JobCard {job} />
          </li>
        {/each}
      </ul>
    {/if}
  </main>
</div>

<style>
  .app {
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
  }

  .app__header {
    padding: 2rem 1.5rem 1rem;
    text-align: center;
  }

  .app__title {
    font-size: 1.75rem;
    font-weight: 700;
    letter-spacing: -0.03em;
    color: var(--color-text);
  }

  .app__tagline {
    font-size: 0.9375rem;
    color: var(--color-muted);
    margin-top: 0.3rem;
  }

  .app__main {
    flex: 1;
    width: 100%;
    max-width: 52rem;
    margin: 0 auto;
    padding: 0 1rem 3rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .totals {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.875rem;
    background: color-mix(in srgb, var(--color-success) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-success) 30%, transparent);
    border-radius: 8px;
    padding: 0.6rem 1rem;
    flex-wrap: wrap;
  }

  .totals__label {
    font-weight: 600;
    color: var(--color-success);
  }

  .totals__arrow {
    color: var(--color-muted);
  }

  .totals__bytes {
    color: var(--color-text-secondary);
  }

  .totals__saving {
    font-weight: 600;
    color: var(--color-success);
  }

  .btn-download-all {
    margin-left: auto;
    font-size: 0.8125rem;
    padding: 0.3rem 0.85rem;
    background: var(--color-surface);
    color: var(--color-accent);
    border: 1px solid color-mix(in srgb, var(--color-accent) 40%, transparent);
    border-radius: 5px;
    cursor: pointer;
    transition: background 0.12s;
    white-space: nowrap;
  }

  .btn-download-all:hover {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
  }

  .btn-download-all:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .job-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
</style>
