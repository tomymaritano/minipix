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
</script>

<div
  class="absolute inset-0 flex flex-col"
  style="bottom: var(--bar-h); animation: fade 0.4s var(--ease);"
>
  <!-- Top bar: nav + close -->
  <div
    class="absolute top-0 left-0 right-0 h-[56px] flex items-center justify-center z-[6] pointer-events-none"
  >
    {#if pos && pos.total > 1}
      <div
        class="inline-flex items-center gap-3 text-[12px] text-[var(--fg-dim)] tabular-nums pointer-events-auto bg-black/35 backdrop-blur-md px-3 py-[5px] rounded-full border border-[var(--line)]"
      >
        <button
          onclick={() => playground.step(-1)}
          aria-label="Previous image"
          class="text-[var(--fg)] text-[16px] leading-none w-[18px] hover:text-[var(--mint)] transition-colors"
          >‹</button
        >
        <span>{pos.index} / {pos.total}</span>
        <button
          onclick={() => playground.step(1)}
          aria-label="Next image"
          class="text-[var(--fg)] text-[16px] leading-none w-[18px] hover:text-[var(--mint)] transition-colors"
          >›</button
        >
      </div>
    {/if}
    <button
      onclick={() => playground.removeActive()}
      aria-label="Close"
      class="absolute top-[22px] right-[26px] text-[22px] leading-none text-[var(--fg-dim)] pointer-events-auto hover:text-[var(--fg)] transition-colors"
      >×</button
    >
  </div>

  <!-- Stage -->
  <div class="flex-1 min-h-0 mx-10 mt-16 mb-3 grid place-items-stretch">
    {#if job.status === 'error'}
      <div class="place-self-center text-center">
        <p class="text-[var(--bad)] text-[14px]">{errorLabel(job.errorCode)}</p>
        {#if job.errorMessage}
          <p class="mt-2 text-[var(--fg-dim)] text-[12px] max-w-[440px] break-words">
            {job.errorMessage}
          </p>
        {/if}
      </div>
    {:else if job.result && afterUrl && beforeUrl}
      {#key job.id}
        <Compare {beforeUrl} {afterUrl} {zoom} />
      {/key}
    {:else if beforeUrl}
      <div class="relative w-full h-full grid place-items-center">
        <img
          src={beforeUrl}
          alt={job.file.name}
          class="max-w-full max-h-full w-auto h-auto rounded-[10px] opacity-50"
        />
        <div class="absolute inset-0 grid place-items-center">
          <span
            class="w-[26px] h-[26px] border-2 border-[var(--fg-ghost)] border-t-[var(--mint)] rounded-full animate-spin"
          ></span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Bottom rail -->
  <div class="flex items-center justify-between px-[42px] pb-[6px] h-[26px]">
    <div class="inline-flex items-baseline gap-[14px] min-w-0 text-[12px]">
      <span class="text-[var(--fg)] overflow-hidden text-ellipsis whitespace-nowrap max-w-[40vw]"
        >{job.file.name}</span
      >
      {#if job.result}
        <span class="text-[var(--fg-dim)] tabular-nums"
          >{job.result.width} × {job.result.height}</span
        >
      {/if}
      {#if job.avifSlowWarning}
        <span class="text-[var(--warn)] text-[11px]"
          >AVIF encode is slow in-browser — hang tight</span
        >
      {/if}
    </div>
    {#if job.result}
      <div class="inline-flex items-center gap-1 text-[12px]">
        <span class="text-[var(--fg-faint)] tracking-[0.04em] mr-1">Zoom</span>
        {#each [1, 2, 4] as z (z)}
          <Button
            variant={zoom === z ? 'default' : 'ghost'}
            size="sm"
            onclick={() => (zoom = z)}
            class={zoom === z
              ? 'bg-[var(--ink-btn)] text-[var(--ink-btn-fg)]'
              : 'text-[var(--fg-dim)]'}>{z}×</Button
          >
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
