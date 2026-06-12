<script lang="ts">
  import { globalOptions } from '../state.svelte.js';

  type Format = 'auto' | 'png' | 'jpeg' | 'webp' | 'avif';

  // "auto" means compress (no format conversion); map to/from globalOptions.format
  let selectedFormat = $state<Format>(globalOptions.format ?? 'auto');

  $effect(() => {
    if (selectedFormat === 'auto') {
      globalOptions.format = undefined;
    } else {
      globalOptions.format = selectedFormat;
    }
  });

  // When user picks WebP lossless is the only safe client-side path; auto-set it.
  $effect(() => {
    if (selectedFormat === 'webp' && !globalOptions.lossless) {
      globalOptions.lossless = true;
    }
  });

  // When user leaves WebP, clear the auto-set lossless so it doesn't bleed into
  // other formats unexpectedly (only if it was auto-set — we track nothing extra;
  // clearing unconditionally is the simplest safe choice given the hint).
  // Use a plain (non-reactive) variable to track the previous format value.
  let prevFormat: Format = globalOptions.format ?? 'auto';
  $effect(() => {
    const current = selectedFormat;
    if (prevFormat === 'webp' && current !== 'webp') {
      globalOptions.lossless = false;
    }
    prevFormat = current;
  });

  const webpLosslessHint =
    'In-browser WebP encoding only supports lossless mode. Lossless has been enabled automatically.';
  const avifSlowHint =
    'AVIF encoding is CPU-intensive. Low effort values (1–3) are recommended for reasonable speed.';
  const safariAvifHint =
    'AVIF decoding is not supported in Safari < 17. Output may fail in those browsers.';
</script>

<section class="controls" aria-label="Global compression options">
  <h2 class="controls__title">Options</h2>

  <div class="controls__grid">
    <!-- Format -->
    <label class="control-label" for="format-select">Output format</label>
    <div class="control-value">
      <select id="format-select" bind:value={selectedFormat}>
        <option value="auto">auto (smart compress)</option>
        <option value="png">PNG</option>
        <option value="jpeg">JPEG</option>
        <option value="webp">WebP</option>
        <option value="avif">AVIF</option>
      </select>

      {#if selectedFormat === 'webp'}
        <p class="hint hint--warn">{webpLosslessHint}</p>
      {/if}

      {#if selectedFormat === 'avif'}
        <p class="hint hint--warn">{avifSlowHint}</p>
        <p class="hint hint--info">{safariAvifHint}</p>
      {/if}
    </div>

    <!-- Quality -->
    <label class="control-label" for="quality-range">
      Quality
      <span class="control-label__value">{globalOptions.quality ?? 75}</span>
    </label>
    <div class="control-value">
      <input
        id="quality-range"
        type="range"
        min="1"
        max="100"
        bind:value={globalOptions.quality}
        disabled={globalOptions.lossless === true}
        aria-valuemin={1}
        aria-valuemax={100}
        aria-valuenow={globalOptions.quality ?? 75}
      />
      {#if globalOptions.lossless}
        <p class="hint hint--info">Quality is ignored in lossless mode.</p>
      {/if}
    </div>

    <!-- Effort -->
    <label class="control-label" for="effort-range">
      Effort
      <span class="control-label__value">{globalOptions.effort ?? 4}</span>
    </label>
    <div class="control-value">
      <input
        id="effort-range"
        type="range"
        min="0"
        max="9"
        bind:value={globalOptions.effort}
        aria-valuemin={0}
        aria-valuemax={9}
        aria-valuenow={globalOptions.effort ?? 4}
      />
      {#if selectedFormat === 'avif'}
        <p class="hint hint--info">Suggested: keep effort at 2 or below for AVIF.</p>
      {/if}
    </div>

    <!-- Lossless -->
    <label class="control-label" for="lossless-check">Lossless</label>
    <div class="control-value control-value--inline">
      <input
        id="lossless-check"
        type="checkbox"
        bind:checked={globalOptions.lossless}
        disabled={selectedFormat === 'webp'}
        aria-describedby={selectedFormat === 'webp' ? 'webp-lossless-note' : undefined}
      />
      {#if selectedFormat === 'webp'}
        <span id="webp-lossless-note" class="hint hint--inline">required for WebP</span>
      {/if}
    </div>
  </div>
</section>

<style>
  .controls {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.25rem 1.5rem;
  }

  .controls__title {
    font-size: 0.875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-muted);
    margin-bottom: 1rem;
  }

  .controls__grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.75rem 1.25rem;
    align-items: start;
  }

  .control-label {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    padding-top: 0.3rem;
    white-space: nowrap;
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
  }

  .control-label__value {
    font-weight: 600;
    color: var(--color-text);
    min-width: 1.75rem;
    text-align: right;
  }

  .control-value {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .control-value--inline {
    flex-direction: row;
    align-items: center;
    gap: 0.6rem;
    padding-top: 0.3rem;
  }

  select {
    background: var(--color-bg);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: 5px;
    padding: 0.3rem 0.6rem;
    font-size: 0.875rem;
    cursor: pointer;
    width: 100%;
    max-width: 16rem;
  }

  input[type='range'] {
    width: 100%;
    max-width: 16rem;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  input[type='range']:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  input[type='checkbox'] {
    width: 1rem;
    height: 1rem;
    accent-color: var(--color-accent);
    cursor: pointer;
    flex-shrink: 0;
  }

  input[type='checkbox']:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hint {
    font-size: 0.78rem;
    line-height: 1.4;
    margin: 0;
  }

  .hint--warn {
    color: var(--color-warn);
  }

  .hint--info {
    color: var(--color-muted);
  }

  .hint--inline {
    font-size: 0.78rem;
    color: var(--color-muted);
  }
</style>
