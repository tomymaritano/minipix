<script lang="ts">
  import { globalOptions, playground, type Job } from '../state.svelte';
  import { FORMATS, FORMAT_LABEL, fmtBytes, fmtSavings, mimeFor, renameTo } from '../format';
  import type { FormatId } from '../format';
  import { Button } from '$lib/components/ui/button';
  import { Select } from '$lib/components/ui/select';
  import { Slider } from '$lib/components/ui/slider';
  import { Switch } from '$lib/components/ui/switch';
  import { Badge } from '$lib/components/ui/badge';
  import { Separator } from '$lib/components/ui/separator';

  interface Props {
    active: Job | null;
  }
  const { active }: Props = $props();

  // Per-format constraints (unchanged logic)
  const fmt = $derived(globalOptions.format ?? 'png');
  const losslessForced = $derived(fmt === 'webp');
  const losslessAllowed = $derived(fmt === 'png' || fmt === 'webp');
  const isLossless = $derived(losslessForced || globalOptions.lossless === true);
  const qualityActive = $derived(!isLossless);
  const resizeOn = $derived(globalOptions.resize !== undefined);

  let timer: ReturnType<typeof setTimeout> | undefined;
  function recompress(delay: number): void {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const a = playground.active;
      if (a) playground.retry(a, { ...globalOptions });
    }, delay);
  }

  function pickFormat(value: string): void {
    const f = value as FormatId;
    globalOptions.format = f;
    if (f === 'webp') globalOptions.lossless = true;
    if (f === 'jpeg' || f === 'avif') globalOptions.lossless = false;
    recompress(0);
  }

  function toggleLossless(checked: boolean): void {
    if (losslessForced) return;
    globalOptions.lossless = checked;
    recompress(0);
  }

  function toggleResize(checked: boolean): void {
    globalOptions.resize = checked ? 1024 : undefined;
    recompress(0);
  }

  function setResizeDim(v: number): void {
    globalOptions.resize = Math.min(8192, Math.max(16, Math.round(v) || 16));
    recompress(220);
  }

  function download(): void {
    if (!active?.result) return;
    const blob = new Blob([active.result.data], { type: mimeFor(active.result.format) });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = renameTo(active.file.name, active.result.format);
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  const formatItems = FORMATS.map((f) => ({ value: f, label: FORMAT_LABEL[f] }));
</script>

<div
  class="toolbar fixed inset-x-0 bottom-0 z-40 border-t border-[var(--line)]"
  style="background: linear-gradient(to top, var(--bg-1) 60%, rgba(16,18,22,0.78)); backdrop-filter: blur(14px) saturate(1.2); box-shadow: var(--shadow-panel);"
>
  <div
    class="mx-auto flex w-full max-w-[1520px] flex-wrap items-center justify-between gap-x-6 gap-y-3 px-5 py-3 lg:px-7"
    style="min-height: var(--bar-h);"
  >
    <!-- Left group: controls (stays one line; wraps the right group below when cramped) -->
    <div class="flex flex-nowrap items-center gap-x-[13px]">
      <!-- Format select -->
      <div class="flex items-center gap-[10px]">
        <span class="label-xs">Format</span>
        <Select
          value={globalOptions.format ?? 'png'}
          items={formatItems}
          onValueChange={pickFormat}
          aria-label="Output format"
        />
      </div>

      <Separator orientation="vertical" class="h-[22px]" />

      <!-- Quality slider -->
      <div
        class="flex items-center gap-[11px] transition-opacity duration-200"
        style:opacity={qualityActive ? '1' : '0.4'}
      >
        <span class="label-xs">Smaller</span>
        <Slider
          value={globalOptions.quality ?? 75}
          min={1}
          max={100}
          disabled={!qualityActive}
          onValueChange={(v: number) => {
            globalOptions.quality = v;
            recompress(180);
          }}
        />
        <span class="label-xs">Sharper</span>
        {#if qualityActive}
          <span class="w-[26px] text-[11px] font-medium tabular-nums text-[var(--fg-dim)]"
            >{globalOptions.quality ?? 75}</span
          >
        {/if}
      </div>

      <Separator orientation="vertical" class="h-[22px]" />

      <!-- Lossless switch -->
      <Switch
        checked={isLossless}
        disabled={!losslessAllowed}
        label="Lossless"
        onCheckedChange={toggleLossless}
      />

      <!-- Resize switch + px input (only when active image) -->
      {#if active}
        <Separator orientation="vertical" class="h-[22px]" />
        <div class="flex items-center gap-[10px]">
          <Switch checked={resizeOn} label="Resize" onCheckedChange={toggleResize} />
          {#if resizeOn}
            <div
              class="inline-flex items-center gap-[5px] rounded-[7px] border border-[var(--line)] bg-[var(--bg-3)] pr-[8px] focus-within:border-[var(--mint)] focus-within:ring-2 focus-within:ring-[var(--mint-dim)]"
            >
              <input
                class="w-[52px] bg-transparent px-[8px] py-[5px] text-[12px] tabular-nums text-[var(--fg)] outline-none [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
                type="number"
                min="16"
                max="8192"
                value={globalOptions.resize ?? 1024}
                oninput={(e) => setResizeDim(Number(e.currentTarget.value))}
                aria-label="Max dimension in pixels"
              />
              <span class="label-xs">px</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Right group: stats + download -->
    {#if active}
      <div class="flex shrink-0 items-center gap-[14px]">
        {#if active.status === 'working'}
          <span class="inline-flex items-center gap-[8px] text-[12px] text-[var(--fg-dim)]">
            <span
              class="h-[12px] w-[12px] animate-spin rounded-full border-[1.5px] border-[var(--fg-ghost)] border-t-[var(--mint)]"
            ></span>
            {active.phase === 'decoding' ? 'decoding…' : 'compressing…'}
          </span>
        {:else if active.status === 'error'}
          <span class="text-[12px] font-medium text-[var(--bad)]"
            >{active.errorCode ?? 'error'}</span
          >
        {:else if active.status === 'done' && active.result}
          {@const r = active.result}
          <div class="flex items-center gap-[9px] text-[12px] tabular-nums">
            <div class="flex flex-col items-end leading-tight">
              <span class="label-xs">Original</span>
              <span class="font-medium text-[var(--fg)]">{fmtBytes(r.bytesIn)}</span>
            </div>
            <span class="text-[var(--fg-faint)]">→</span>
            <div class="flex flex-col items-end leading-tight">
              <span class="label-xs">Output</span>
              <span class="font-medium text-[var(--fg)]">{fmtBytes(r.bytesOut)}</span>
            </div>
            <Badge
              variant={r.ratio > 1 ? 'warning' : 'default'}
              class="savings ml-1"
              data-testid="savings"
            >
              {fmtSavings(r.ratio)}
            </Badge>
          </div>
          <Button variant="primary" onclick={download} class="h-[34px]">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            Download
          </Button>
        {/if}
      </div>
    {/if}
  </div>
</div>
