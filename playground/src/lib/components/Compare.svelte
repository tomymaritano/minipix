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
</script>

<div
  class="relative w-full h-full overflow-hidden cursor-ew-resize touch-none rounded-[10px]"
  style="background-color: #0e0f12; background-image: linear-gradient(45deg, #141518 25%, transparent 25%) -8px 0 / 16px 16px, linear-gradient(-45deg, #141518 25%, transparent 25%) -8px 0 / 16px 16px, linear-gradient(45deg, transparent 75%, #141518 75%) 0 0 / 16px 16px, linear-gradient(-45deg, transparent 75%, #141518 75%) 0 0 / 16px 16px;"
  class:cursor-grabbing={dragging}
  bind:this={stage}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  role="slider"
  aria-label="Before / after comparison"
  aria-valuenow={Math.round(pos)}
  aria-valuemin={0}
  aria-valuemax={100}
  tabindex="0"
>
  <!-- After layer (compressed) — full width, below -->
  <div class="absolute inset-0 grid place-items-center">
    <img
      src={afterUrl}
      alt="Compressed"
      style:transform="scale({zoom})"
      style="transition: transform 0.22s var(--ease);"
      class="max-w-full max-h-full w-auto h-auto transform-origin-center"
      draggable="false"
    />
  </div>

  <!-- Before layer (original) — clipped to left portion -->
  <div class="absolute inset-0 grid place-items-center" style:clip-path="inset(0 {100 - pos}% 0 0)">
    <img
      src={beforeUrl}
      alt="Original"
      style:transform="scale({zoom})"
      style="transition: transform 0.22s var(--ease);"
      class="max-w-full max-h-full w-auto h-auto transform-origin-center"
      draggable="false"
    />
  </div>

  <!-- Labels -->
  <span
    class="absolute top-3 left-3 text-[10px] tracking-[0.12em] uppercase text-[var(--fg-dim)] bg-black/45 backdrop-blur-md px-2 py-[3px] rounded-[5px] pointer-events-none z-[4]"
    >Original</span
  >
  <span
    class="absolute top-3 right-3 text-[10px] tracking-[0.12em] uppercase text-[var(--mint)] bg-black/45 backdrop-blur-md px-2 py-[3px] rounded-[5px] pointer-events-none z-[4]"
    >minipix</span
  >

  <!-- Divider -->
  <div
    class="absolute top-0 bottom-0 w-0 z-[5] pointer-events-none"
    style:left="{pos}%"
    style="transform: translateX(-0.5px);"
  >
    <!-- Line -->
    <div
      class="absolute top-0 bottom-0 left-0 w-px bg-white/85 shadow-[0_0_0_0.5px_rgba(0,0,0,0.3)]"
    ></div>
    <!-- Handle -->
    <div
      class="absolute top-1/2 left-0 -translate-x-1/2 -translate-y-1/2 min-w-[42px] h-[26px] px-[9px] grid place-items-center bg-[var(--bg)] border border-white/85 rounded-[13px] shadow-[0_4px_14px_rgba(0,0,0,0.5)]"
    >
      <span class="text-[11px] font-medium text-[var(--fg)] tabular-nums">{Math.round(pos)}%</span>
    </div>
  </div>
</div>
