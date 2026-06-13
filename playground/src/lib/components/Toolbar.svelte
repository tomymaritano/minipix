<script lang="ts">
  import { globalOptions, playground, type Job } from '../state.svelte';
  import { FORMATS, FORMAT_LABEL, fmtBytes, fmtSavings, mimeFor, renameTo } from '../format';
  import type { FormatId } from '../format';
  import { Button } from '$lib/components/ui/button';
  import { Select } from '$lib/components/ui/select';
  import { Slider } from '$lib/components/ui/slider';
  import { Switch } from '$lib/components/ui/switch';
  import { Badge } from '$lib/components/ui/badge';

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
  class="fixed left-0 right-0 bottom-0 z-40 flex items-center justify-between gap-4 px-5 border-t border-[var(--line)]"
  style="height: var(--bar-h); background: linear-gradient(to top, var(--bg) 70%, rgba(12,13,16,0.85)); backdrop-filter: blur(8px);"
>
  <!-- Left group -->
  <div class="flex items-center gap-[14px] min-w-0">
    <!-- Format select -->
    <div class="flex items-center gap-[9px]">
      <span class="text-[12px] text-[var(--fg-faint)] tracking-[0.04em]">Format</span>
      <Select
        value={globalOptions.format ?? 'png'}
        items={formatItems}
        onValueChange={pickFormat}
        aria-label="Output format"
      />
    </div>

    <span class="w-px h-[16px] bg-[var(--line)]"></span>

    <!-- Quality slider -->
    <div
      class="flex items-center gap-[9px] transition-opacity duration-200"
      style:opacity={qualityActive ? '1' : '0.32'}
    >
      <span class="text-[12px] text-[var(--fg-faint)] tracking-[0.04em]">Smaller</span>
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
      <span class="text-[12px] text-[var(--fg-faint)] tracking-[0.04em]">Sharper</span>
    </div>

    <span class="w-px h-[16px] bg-[var(--line)]"></span>

    <!-- Lossless switch -->
    <Switch
      checked={isLossless}
      disabled={!losslessAllowed}
      label="Lossless"
      onCheckedChange={toggleLossless}
    />

    <!-- Resize switch + px input (only when active image) -->
    {#if active}
      <Switch checked={resizeOn} label="Resize" onCheckedChange={toggleResize} />
      {#if resizeOn}
        <input
          class="w-[56px] bg-transparent border border-[var(--line)] rounded-[5px] px-[6px] py-[3px] text-[12px] text-[var(--fg)]"
          type="number"
          min="16"
          max="8192"
          value={globalOptions.resize ?? 1024}
          oninput={(e) => setResizeDim(Number(e.currentTarget.value))}
          aria-label="Max dimension in pixels"
        />
        <span class="text-[12px] text-[var(--fg-faint)] tracking-[0.04em]">px</span>
      {/if}
    {/if}
  </div>

  <!-- Right group -->
  <div class="flex items-center gap-[10px] shrink-0">
    {#if active}
      {#if active.status === 'working'}
        <span class="inline-flex items-center gap-2 text-[12px] text-[var(--fg-dim)]">
          <span
            class="w-[11px] h-[11px] border-[1.5px] border-[var(--fg-ghost)] border-t-[var(--mint)] rounded-full animate-spin"
          ></span>
          {active.phase === 'decoding' ? 'decoding…' : 'compressing…'}
        </span>
      {:else if active.status === 'error'}
        <span class="text-[12px] text-[var(--bad)]">{active.errorCode ?? 'error'}</span>
      {:else if active.status === 'done' && active.result}
        {@const r = active.result}
        <span class="inline-flex items-center gap-2 text-[12px] tabular-nums">
          <span class="text-[var(--fg-faint)] tracking-[0.04em]">Original</span>
          <span class="text-[var(--fg)] font-medium">{fmtBytes(r.bytesIn)}</span>
          <span class="text-[var(--fg-faint)]">→</span>
          <span class="text-[var(--fg)] font-medium">{fmtBytes(r.bytesOut)}</span>
          <Badge
            variant={r.ratio > 1 ? 'warning' : 'default'}
            class="savings"
            data-testid="savings"
          >
            {fmtSavings(r.ratio)}
          </Badge>
        </span>
        <Button onclick={download}>Download</Button>
      {/if}
    {/if}
  </div>
</div>
