<script lang="ts">
  import { Button } from '$lib/components/ui/button';

  interface Props {
    openPicker: () => void;
    isMac: boolean;
  }
  const { openPicker, isMac }: Props = $props();
  const mod = $derived(isMac ? '⌘' : 'Ctrl');
</script>

<div
  class="h-full flex flex-col items-center justify-center gap-0 select-none"
  style="height: calc(100% - var(--bar-h));"
>
  <!-- Layers icon -->
  <svg
    class="text-[var(--fg-faint)] mb-[26px]"
    style="animation: rise 0.7s var(--ease) both, float 6s ease-in-out 0.7s infinite;"
    width="56"
    height="56"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.1"
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

  <p
    class="text-[15px] tracking-tight text-[var(--fg)]"
    style="animation: rise 0.7s var(--ease) 0.06s both;"
  >
    Drop PNG, JPEG, WebP or AVIF here
  </p>
  <p
    class="mt-2 text-[12px] text-[var(--fg-dim)]"
    style="animation: rise 0.7s var(--ease) 0.12s both;"
  >
    compressed in your browser — nothing is uploaded
  </p>

  <div style="animation: rise 0.7s var(--ease) 0.18s both;" class="mt-[26px]">
    <Button
      onclick={openPicker}
      class="gap-[10px] text-[13px] px-[16px] py-[9px] shadow-[0_6px_22px_rgba(0,0,0,0.5)]"
    >
      Select Files
      <span class="kbd">{mod}O</span>
    </Button>
  </div>

  <p
    class="mt-[18px] inline-flex items-center gap-2 text-[11px] text-[var(--fg-dim)]"
    style="animation: rise 0.7s var(--ease) 0.24s both;"
  >
    <span class="kbd">{mod}V</span>
    <span>to paste from clipboard</span>
  </p>
</div>

<style>
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @keyframes float {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-5px);
    }
  }
</style>
