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
  import { Download, LoaderCircle } from '@lucide/svelte';

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

<div class="toolbar shrink-0 border-t border-border bg-card">
  <div
    class="mx-auto flex w-full min-h-[56px] max-w-[1520px] flex-col gap-3 px-5 py-3 max-[899px]:flex-col min-[900px]:flex-row min-[900px]:flex-wrap min-[900px]:items-center min-[900px]:justify-between lg:px-7"
  >
    <!-- Left group: controls -->
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
          <span class="w-[26px] text-[11px] tabular-nums text-muted-foreground"
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
              class="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary pr-2 focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/30"
            >
              <input
                class="w-[52px] bg-transparent px-2 py-1.5 text-[12px] tabular-nums text-foreground outline-none [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
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
          <span class="inline-flex items-center gap-2 text-[12px] text-muted-foreground">
            <LoaderCircle size={12} class="animate-spin text-primary" />
            {active.phase === 'decoding' ? 'decoding…' : 'compressing…'}
          </span>
        {:else if active.status === 'error'}
          <span class="text-[12px] font-medium text-destructive">{active.errorCode ?? 'error'}</span
          >
        {:else if active.status === 'done' && active.result}
          {@const r = active.result}
          <div class="flex items-center gap-[9px] text-[12px] tabular-nums">
            <div class="flex flex-col items-end leading-tight">
              <span class="label-xs">Original</span>
              <span class="font-medium text-foreground">{fmtBytes(r.bytesIn)}</span>
            </div>
            <span class="text-muted-foreground">→</span>
            <div class="flex flex-col items-end leading-tight">
              <span class="label-xs">Output</span>
              <span class="font-medium text-foreground">{fmtBytes(r.bytesOut)}</span>
            </div>
            <Badge
              variant={r.ratio > 1 ? 'warning' : 'default'}
              class="savings ml-1"
              data-testid="savings"
            >
              {fmtSavings(r.ratio)}
            </Badge>
          </div>
          <Button variant="primary" onclick={download}>
            <Download size={14} />
            Download
          </Button>
        {/if}
      </div>
    {/if}
  </div>
</div>
