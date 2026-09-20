<script lang="ts">
  interface Props {
    beforeUrl: string;
    afterUrl: string;
    zoom: number;
  }
  const { beforeUrl, afterUrl, zoom }: Props = $props();

  let pos = $state(50); // divisor position 0..100
  let stage: HTMLDivElement;
  let dragging = $state(false);

  function moveTo(clientX: number): void {
    const r = stage.getBoundingClientRect();
    const p = ((clientX - r.left) / r.width) * 100;
    pos = Math.min(100, Math.max(0, p));
  }

  function onPointerDown(e: PointerEvent): void {
    dragging = true;
    stage.setPointerCapture(e.pointerId);
    moveTo(e.clientX);
  }
  function onPointerMove(e: PointerEvent): void {
    if (dragging) moveTo(e.clientX);
  }
  function onPointerUp(e: PointerEvent): void {
    dragging = false;
    if (stage.hasPointerCapture(e.pointerId)) stage.releasePointerCapture(e.pointerId);
  }
  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'ArrowLeft') pos = Math.max(0, pos - 2);
    else if (e.key === 'ArrowRight') pos = Math.min(100, pos + 2);
    else return;
    e.preventDefault();
  }
</script>

<div
  class="checker relative h-full w-full touch-none overflow-hidden p-5"
  class:cursor-ew-resize={!dragging}
  class:cursor-grabbing={dragging}
  bind:this={stage}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onkeydown={onKeydown}
  role="slider"
  aria-label="Before / after comparison"
  aria-valuenow={Math.round(pos)}
  aria-valuemin={0}
  aria-valuemax={100}
  tabindex="0"
>
  <!-- After layer (compressed) — full width, below -->
  <div class="absolute inset-0 grid place-items-center p-5">
    <img
      src={afterUrl}
      alt="Compressed"
      style:transform="scale({zoom})"
      style="image-rendering: {zoom > 1 ? 'pixelated' : 'auto'};"
      class="max-h-full max-w-full origin-center transition-transform duration-[220ms] ease-[var(--ease)]"
      draggable="false"
    />
  </div>

  <!-- Before layer (original) — clipped to left portion -->
  <div
    class="absolute inset-0 grid place-items-center p-5"
    style:clip-path="inset(0 {100 - pos}% 0 0)"
  >
    <img
      src={beforeUrl}
      alt="Original"
      style:transform="scale({zoom})"
      style="image-rendering: {zoom > 1 ? 'pixelated' : 'auto'};"
      class="max-h-full max-w-full origin-center transition-transform duration-[220ms] ease-[var(--ease)]"
      draggable="false"
    />
  </div>

  <!-- Labels -->
  <span
    class="pointer-events-none absolute left-4 top-4 z-[4] rounded-md border border-border bg-card/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground"
    >Original</span
  >
  <span
    class="pointer-events-none absolute right-4 top-4 z-[4] rounded-md border border-primary/25 bg-primary/10 px-2 py-1 text-[10px] uppercase tracking-[0.14em] text-primary"
    >minipix</span
  >

  <!-- Divider -->
  <div
    class="pointer-events-none absolute bottom-0 top-0 z-[5] w-0"
    style:left="{pos}%"
    style="transform: translateX(-0.5px);"
  >
    <div class="absolute bottom-0 left-0 top-0 w-px bg-white/80"></div>
    <div
      class="absolute left-0 top-1/2 grid h-7 min-w-[46px] -translate-x-1/2 -translate-y-1/2 place-items-center rounded-md border border-primary/50 bg-card px-2.5 shadow-[var(--shadow-md)]"
    >
      <span class="text-[11px] font-medium tabular-nums text-foreground">{Math.round(pos)}%</span>
    </div>
  </div>
</div>
