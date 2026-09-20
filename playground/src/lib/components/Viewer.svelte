<script lang="ts">
  import type { Job } from '../state.svelte';
  import { mimeFor, errorLabel } from '../format';
  import Compare from './Compare.svelte';
  import { Segmented } from '$lib/components/ui/segmented';
  import { Tooltip } from '$lib/components/ui/tooltip';
  import { TriangleAlert, Clock, LoaderCircle } from '@lucide/svelte';

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

  const zoomItems = [
    { value: '1', label: '1×' },
    { value: '2', label: '2×' },
    { value: '4', label: '4×' },
  ];
</script>

<!-- Clears fixed toolbar (--bar-h); drop when Toolbar is in-flow -->
<div
  class="viewer flex h-[calc(100%-var(--bar-h))] min-h-0 flex-col overflow-hidden rounded-xl border border-border bg-card"
>
  <div class="relative min-h-0 flex-1">
    {#if job.status === 'error'}
      <div class="grid h-full place-items-center px-6 text-center">
        <div>
          <div class="mx-auto mb-4 grid place-items-center text-destructive">
            <TriangleAlert size={20} />
          </div>
          <p class="text-[14px] font-medium text-destructive">{errorLabel(job.errorCode)}</p>
          {#if job.errorMessage}
            <p class="mx-auto mt-2 max-w-[440px] break-words text-[12px] text-muted-foreground">
              {job.errorMessage}
            </p>
          {/if}
        </div>
      </div>
    {:else if job.result && afterUrl && beforeUrl}
      {#key job.id}
        <Compare {beforeUrl} {afterUrl} {zoom} />
      {/key}
    {:else if beforeUrl}
      <div class="checker relative grid h-full w-full place-items-center p-6">
        <img src={beforeUrl} alt={job.file.name} class="max-h-full max-w-full opacity-40" />
        <div class="absolute inset-0 grid place-items-center">
          <LoaderCircle size={28} class="animate-spin text-primary" />
        </div>
      </div>
    {/if}
  </div>

  <div class="flex h-10 shrink-0 items-center justify-between gap-3 border-t border-border px-4">
    <div class="inline-flex min-w-0 items-center gap-3 text-[12px]">
      <span
        class="max-w-[40vw] overflow-hidden text-ellipsis whitespace-nowrap font-medium text-foreground"
        >{job.file.name}</span
      >
      {#if job.result}
        <span class="tabular-nums text-muted-foreground"
          >{job.result.width} <span class="opacity-50">×</span> {job.result.height}</span
        >
      {/if}
      {#if job.avifSlowWarning}
        <Tooltip content="AVIF encode is slow in-browser">
          <span class="inline-flex text-warning" aria-label="AVIF encode is slow in-browser">
            <Clock size={14} />
          </span>
        </Tooltip>
      {/if}
    </div>
    {#if job.result}
      <div class="inline-flex items-center gap-2 text-[12px]">
        <span class="label-xs">Zoom</span>
        <Segmented
          value={String(zoom)}
          items={zoomItems}
          onValueChange={(v) => {
            zoom = Number(v);
          }}
        />
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
</style>
