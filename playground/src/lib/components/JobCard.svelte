<script lang="ts">
  import { playground } from '../state.svelte.js';
  import type { Job } from '../state.svelte.js';

  interface Props {
    job: Job;
  }

  const { job }: Props = $props();

  // Object URL for the original file preview — revoked when component is destroyed.
  let previewUrl = $state<string | undefined>(undefined);

  $effect(() => {
    const url = URL.createObjectURL(job.file);
    previewUrl = url;
    return () => {
      URL.revokeObjectURL(url);
    };
  });

  function fmt(bytes: number): string {
    if (bytes < 1024) return `${String(bytes)} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  function fmtRatio(ratio: number): string {
    return `−${String(Math.round((1 - ratio) * 100))}%`;
  }

  function handleRetry(): void {
    playground.retry(job);
  }

  const errorLabels: Record<string, string> = {
    UnsupportedFormat: 'Unsupported format',
    InvalidOptions: 'Invalid options',
    InternalPanic: 'Internal error (wasm panic)',
    WasmInitError: 'WebAssembly failed to initialize',
    WorkerSpawnError: 'Could not start compression worker',
    WorkerError: 'Worker crashed',
    ReadError: 'Could not read file',
    UnknownError: 'Unknown error',
    Destroyed: 'Pool destroyed',
  };

  function errorLabel(code: string | undefined): string {
    if (!code) return 'Error';
    return errorLabels[code] ?? code;
  }
</script>

<article class="job-card" aria-label={`Job: ${job.file.name}`}>
  <!-- Preview -->
  <div class="job-card__preview">
    {#if previewUrl}
      <img src={previewUrl} alt={job.file.name} loading="lazy" />
    {:else}
      <div class="job-card__preview-placeholder" aria-hidden="true"></div>
    {/if}
  </div>

  <!-- Info -->
  <div class="job-card__info">
    <p class="job-card__name" title={job.file.name}>{job.file.name}</p>
    <p class="job-card__size">{fmt(job.file.size)}</p>

    {#if job.avifSlowWarning && (job.status === 'queued' || job.status === 'working')}
      <p class="job-card__hint job-card__hint--warn">
        Large file + AVIF — encoding may take a while.
      </p>
    {/if}
  </div>

  <!-- Status -->
  <div class="job-card__status">
    {#if job.status === 'queued'}
      <span class="badge badge--queued">Queued</span>
    {:else if job.status === 'working'}
      <span class="badge badge--working" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        {job.phase === 'encoding' ? 'Encoding…' : 'Decoding…'}
      </span>
    {:else if job.status === 'done' && job.result}
      <span class="badge badge--done">Done</span>
      <p class="job-card__result">
        {fmt(job.result.bytesOut)}
        <span class="job-card__ratio">{fmtRatio(job.result.ratio)}</span>
      </p>
      <p class="job-card__dims">{job.result.width}×{job.result.height} · {job.result.format}</p>
    {:else if job.status === 'error'}
      <span class="badge badge--error">{errorLabel(job.errorCode)}</span>
      {#if job.errorMessage}
        <p class="job-card__error-msg">{job.errorMessage}</p>
      {/if}
    {/if}
  </div>

  <!-- Actions -->
  <div class="job-card__actions">
    {#if job.status === 'error' || job.status === 'done'}
      <button class="btn-retry" onclick={handleRetry} aria-label={`Retry ${job.file.name}`}>
        Retry
      </button>
    {/if}
  </div>
</article>

<style>
  .job-card {
    display: grid;
    grid-template-columns: 3.5rem 1fr auto auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
  }

  .job-card__preview {
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 5px;
    overflow: hidden;
    background: var(--color-border);
    flex-shrink: 0;
  }

  .job-card__preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .job-card__preview-placeholder {
    width: 100%;
    height: 100%;
    background: var(--color-border);
  }

  .job-card__info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .job-card__name {
    font-size: 0.875rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-text);
  }

  .job-card__size {
    font-size: 0.78rem;
    color: var(--color-muted);
  }

  .job-card__hint {
    font-size: 0.75rem;
    margin: 0;
  }

  .job-card__hint--warn {
    color: var(--color-warn);
  }

  .job-card__status {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.25rem;
    min-width: 6rem;
  }

  .job-card__result {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    display: flex;
    gap: 0.35rem;
    align-items: baseline;
  }

  .job-card__ratio {
    font-size: 0.75rem;
    color: var(--color-success);
    font-weight: 600;
  }

  .job-card__dims {
    font-size: 0.75rem;
    color: var(--color-muted);
  }

  .job-card__error-msg {
    font-size: 0.75rem;
    color: var(--color-error);
    max-width: 12rem;
    text-align: right;
    word-break: break-word;
  }

  .job-card__actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  /* Badges */
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.2rem 0.55rem;
    border-radius: 99px;
    white-space: nowrap;
  }

  .badge--queued {
    background: var(--color-border);
    color: var(--color-muted);
  }

  .badge--working {
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
  }

  .badge--done {
    background: color-mix(in srgb, var(--color-success) 18%, transparent);
    color: var(--color-success);
  }

  .badge--error {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  /* Spinner */
  .spinner {
    display: inline-block;
    width: 0.7rem;
    height: 0.7rem;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Retry button */
  .btn-retry {
    font-size: 0.8125rem;
    padding: 0.3rem 0.75rem;
    background: var(--color-surface);
    color: var(--color-text-secondary);
    border: 1px solid var(--color-border);
    border-radius: 5px;
    cursor: pointer;
    transition: background 0.12s;
  }

  .btn-retry:hover {
    background: var(--color-border);
    color: var(--color-text);
  }

  .btn-retry:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
</style>
