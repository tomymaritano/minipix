<script lang="ts">
  import { playground, type Job } from '../state.svelte';
  import { mimeFor, errorLabel } from '../format';
  import Compare from './Compare.svelte';
  import { Button } from '$lib/components/ui/button';

  interface Props {
    job: Job;
  }
  const { job }: Props = $props();

  let zoom = $state(1);

  let beforeUrl = $state('');
  $effect(() => {
    const url = URL.createObjectURL(job.file);
    beforeUrl = url;
    return () => {
      URL.revokeObjectURL(url);
    };
  });

  let afterUrl = $state('');
  $effect(() => {
    const r = job.result;
    if (!r) {
      afterUrl = '';
      return;
    }
    const url = URL.createObjectURL(new Blob([r.data], { type: mimeFor(r.format) }));
    afterUrl = url;
    return () => {
      URL.revokeObjectURL(url);
    };
  });

  const pos = $derived(playground.position);
  const zoomLevels = [1, 2, 4];
</script>

<div class="viewer absolute inset-0 z-[2] flex flex-col" style="bottom: var(--bar-h);">
  <!-- Top bar: nav (center) + close (right) -->
  <div
    class="pointer-events-none absolute inset-x-0 top-0 z-[6] flex h-[64px] items-center justify-center px-6"
  >
    {#if pos && pos.total > 1}
      <div
        class="pointer-events-auto inline-flex items-center gap-[10px] rounded-full border border-[var(--line)] bg-[var(--bg-2)]/80 px-[6px] py-[5px] text-[12px] tabular-nums text-[var(--fg-dim)] shadow-[var(--shadow-md)] backdrop-blur-md"
      >
        <Button
          variant="ghost"
          size="sm"
          onclick={() => playground.step(-1)}
          aria-label="Previous image"
          class="h-[24px] w-[24px] rounded-full px-0 text-[16px] leading-none"
        >
          ‹
        </Button>
        <span class="min-w-[42px] text-center font-medium text-[var(--fg)]"
          >{pos.index} <span class="text-[var(--fg-faint)]">/</span> {pos.total}</span
        >
        <Button
          variant="ghost"
          size="sm"
          onclick={() => playground.step(1)}
          aria-label="Next image"
          class="h-[24px] w-[24px] rounded-full px-0 text-[16px] leading-none"
        >
          ›
        </Button>
      </div>
    {/if}
    <Button
      variant="icon"
      size="icon"
      onclick={() => playground.removeActive()}
      aria-label="Close"
      class="pointer-events-auto absolute right-6 top-[17px] rounded-full border border-[var(--line)] bg-[var(--bg-2)]/70 backdrop-blur-md hover:border-[var(--line-strong)]"
    >
      <svg
        width="15"
        height="15"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        aria-hidden="true"
      >
        <path d="M18 6 6 18M6 6l12 12" />
      </svg>
    </Button>
  </div>

  <!-- Stage: a contained, framed panel that centers the image -->
  <div class="flex min-h-0 flex-1 items-center justify-center px-8 pb-3 pt-[72px]">
    <div
      class="stage relative grid h-full max-h-[640px] w-full max-w-[920px] place-items-stretch overflow-hidden rounded-[var(--radius)] border border-[var(--line)] bg-[var(--bg-1)]"
      style="box-shadow: var(--shadow-lg), inset 0 0 0 1px rgba(255,255,255,0.02), inset 0 30px 60px -40px rgba(0,0,0,0.8);"
    >
      {#if job.status === 'error'}
        <div class="place-self-center px-6 text-center">
          <div
            class="mx-auto mb-4 grid h-[44px] w-[44px] place-items-center rounded-full border border-[rgba(242,104,90,0.3)] bg-[rgba(242,104,90,0.1)]"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="var(--bad)"
              stroke-width="1.8"
              stroke-linecap="round"
              aria-hidden="true"
            >
              <path
                d="M12 9v4M12 17h.01M10.3 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.7 3.86a2 2 0 0 0-3.42 0Z"
              />
            </svg>
          </div>
          <p class="text-[14px] font-medium text-[var(--bad)]">{errorLabel(job.errorCode)}</p>
          {#if job.errorMessage}
            <p class="mx-auto mt-2 max-w-[440px] break-words text-[12px] text-[var(--fg-dim)]">
              {job.errorMessage}
            </p>
          {/if}
        </div>
      {:else if job.result && afterUrl && beforeUrl}
        {#key job.id}
          <div class="stage-fade contents">
            <Compare {beforeUrl} {afterUrl} {zoom} />
          </div>
        {/key}
      {:else if beforeUrl}
        <div class="relative grid h-full w-full place-items-center p-6">
          <img
            src={beforeUrl}
            alt={job.file.name}
            class="max-h-full max-w-full rounded-[8px] opacity-40"
          />
          <div class="absolute inset-0 grid place-items-center">
            <span
              class="h-[28px] w-[28px] animate-spin rounded-full border-2 border-[var(--fg-ghost)] border-t-[var(--mint)]"
            ></span>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Bottom rail: filename + dims + zoom -->
  <div class="flex h-[34px] items-center justify-between px-9 pb-[8px]">
    <div class="inline-flex min-w-0 items-center gap-[14px] text-[12px]">
      <span
        class="max-w-[40vw] overflow-hidden text-ellipsis whitespace-nowrap font-medium text-[var(--fg)]"
        >{job.file.name}</span
      >
      {#if job.result}
        <span class="tabular-nums text-[var(--fg-dim)]"
          >{job.result.width} <span class="text-[var(--fg-faint)]">×</span>
          {job.result.height}</span
        >
      {/if}
      {#if job.avifSlowWarning}
        <span class="inline-flex items-center gap-[6px] text-[11px] text-[var(--warn)]">
          <span class="h-[5px] w-[5px] rounded-full bg-[var(--warn)]"></span>
          AVIF encode is slow in-browser — hang tight
        </span>
      {/if}
    </div>
    {#if job.result}
      <div class="inline-flex items-center gap-[8px] text-[12px]">
        <span class="label-xs">Zoom</span>
        <div
          class="inline-flex items-center gap-[2px] rounded-[8px] border border-[var(--line)] bg-[var(--bg-2)] p-[2px]"
        >
          {#each zoomLevels as z (z)}
            <button
              type="button"
              onclick={() => (zoom = z)}
              aria-pressed={zoom === z}
              class="h-[22px] w-[30px] rounded-[6px] text-[11px] font-medium tabular-nums transition-colors duration-150 {zoom ===
              z
                ? 'bg-[var(--bg-3)] text-[var(--fg)] shadow-[var(--shadow-sm)]'
                : 'text-[var(--fg-faint)] hover:text-[var(--fg-dim)]'}">{z}×</button
            >
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes fade {
    from {
      opacity: 0;
      transform: scale(0.992);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  .viewer {
    animation: fade 0.4s var(--ease);
  }

  @keyframes stageFade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .stage-fade {
    animation: stageFade 0.32s var(--ease);
  }
</style>
