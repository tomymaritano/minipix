<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';

  interface Props {
    openPicker: () => void;
    isMac: boolean;
  }
  const { openPicker, isMac }: Props = $props();
  const mod = $derived(isMac ? '⌘' : 'Ctrl');

  const formats = ['PNG', 'JPEG', 'WebP', 'AVIF'];
</script>

<div
  class="absolute inset-0 z-[2] grid place-items-center px-6 select-none"
  style="bottom: var(--bar-h);"
>
  <!-- Hero dropzone -->
  <button
    type="button"
    onclick={openPicker}
    aria-label="Select files to compress"
    class="dropzone group relative flex w-full max-w-[560px] flex-col items-center rounded-[18px] border border-dashed border-[var(--line-strong)] bg-[var(--bg-1)]/60 px-10 py-14 text-center backdrop-blur-[2px] transition-[border-color,background-color,transform] duration-200 ease-[var(--ease)] hover:border-[var(--mint)]/55 hover:bg-[var(--bg-2)]/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--mint)] focus-visible:ring-offset-4 focus-visible:ring-offset-[var(--bg)]"
    style="box-shadow: var(--shadow-lg), inset 0 1px 0 rgba(255,255,255,0.03);"
  >
    <!-- Layers icon with emerald glow -->
    <div
      class="icon-rise relative mb-7 grid h-[84px] w-[84px] place-items-center rounded-[20px] border border-[var(--line)] bg-[var(--bg-2)]"
      style="box-shadow: inset 0 1px 0 rgba(255,255,255,0.05), 0 8px 28px -10px rgba(0,0,0,0.7);"
    >
      <div
        class="pointer-events-none absolute -inset-3 rounded-full opacity-70 transition-opacity duration-300 group-hover:opacity-100"
        style="background: radial-gradient(closest-side, var(--mint-glow), transparent 72%); filter: blur(8px);"
      ></div>
      <svg
        class="icon-float relative text-[var(--mint)]"
        width="40"
        height="40"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.3"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path
          d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"
        />
        <path d="m22 12.81-9.17 4.16a2 2 0 0 1-1.66 0L2 12.81" />
        <path d="m22 17.31-9.17 4.16a2 2 0 0 1-1.66 0L2 17.31" />
      </svg>
    </div>

    <h1 class="reveal-1 text-[30px] font-medium leading-[1.1] tracking-[-0.02em] text-[var(--fg)]">
      Compress images, privately
    </h1>
    <p class="reveal-2 mt-3 max-w-[400px] text-[13px] leading-relaxed text-[var(--fg-dim)]">
      Drop a PNG, JPEG, WebP or AVIF. Everything runs in your browser —
      <span class="text-[var(--fg)]">nothing is ever uploaded.</span>
    </p>

    <!-- Primary CTA -->
    <div class="reveal-3 mt-8">
      <Button
        variant="primary"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          openPicker();
        }}
        class="h-[40px] px-6 text-[13px]"
      >
        Select files
        <span
          class="kbd ml-1 border-[rgba(6,18,13,0.25)] bg-[rgba(6,18,13,0.18)] text-[var(--ink-btn-fg)] shadow-none"
          >{mod}O</span
        >
      </Button>
    </div>

    <!-- Format chips -->
    <div class="reveal-4 mt-7 flex items-center gap-[6px]">
      {#each formats as f, i (f)}
        {#if i > 0}
          <span class="text-[var(--fg-ghost)]">·</span>
        {/if}
        <Badge variant="outline" class="px-[9px] py-[4px] text-[10px] tracking-[0.06em]">{f}</Badge>
      {/each}
    </div>
  </button>

  <!-- Paste hint, below the dropzone -->
  <p
    class="reveal-5 absolute bottom-[34px] inline-flex items-center gap-[7px] text-[11px] text-[var(--fg-faint)]"
  >
    <span class="kbd">{mod}V</span>
    <span>to paste from clipboard</span>
  </p>
</div>

<style>
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @keyframes iconPop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.92);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  @keyframes float {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-4px);
    }
  }

  .dropzone {
    animation: rise 0.5s var(--ease) both;
  }
  .icon-rise {
    animation: iconPop 0.6s var(--ease) 0.05s both;
  }
  .icon-float {
    animation: float 6s ease-in-out 0.8s infinite;
  }
  .reveal-1 {
    animation: rise 0.55s var(--ease) 0.12s both;
  }
  .reveal-2 {
    animation: rise 0.55s var(--ease) 0.18s both;
  }
  .reveal-3 {
    animation: rise 0.55s var(--ease) 0.24s both;
  }
  .reveal-4 {
    animation: rise 0.55s var(--ease) 0.3s both;
  }
  .reveal-5 {
    animation: rise 0.55s var(--ease) 0.38s both;
  }

  @media (prefers-reduced-motion: reduce) {
    .icon-float {
      animation: none;
    }
  }
</style>
